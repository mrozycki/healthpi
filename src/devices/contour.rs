use std::{error::Error, time::Duration};

use async_trait::async_trait;
use bluez_async::{BluetoothEvent, BluetoothSession, CharacteristicEvent, DeviceId};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures::StreamExt;
use tokio::time::timeout;
use uuid::Uuid;

use crate::store::measurement::{Record, Value};

use super::device::Device;

const GLUCOSE_SERVICE: Uuid = Uuid::from_u128(0x00001808_0000_1000_8000_00805f9b34fb);
const GLUCOSE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a18_0000_1000_8000_00805f9b34fb);
const GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC: Uuid =
    Uuid::from_u128(0x00002a34_0000_1000_8000_00805f9b34fb);
const RACP_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a52_0000_1000_8000_00805f9b34fb);

pub struct ElitePlus {
    device_id: DeviceId,
}

impl ElitePlus {
    pub fn new(device_id: DeviceId) -> Self {
        Self { device_id }
    }
}

#[async_trait]
impl Device for ElitePlus {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>> {
        session
            .connect_with_timeout(&self.device_id, Duration::from_secs(1))
            .await?;
        Ok(())
    }

    async fn disconnect(
        &self,
        session: &BluetoothSession,
    ) -> Result<(), Box<dyn std::error::Error>> {
        session.disconnect(&self.device_id).await?;
        Ok(())
    }

    async fn get_data(
        &self,
        session: &BluetoothSession,
    ) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
        let glucose_characteristic = session
            .get_service_characteristic_by_uuid(
                &self.device_id,
                GLUCOSE_SERVICE,
                GLUCOSE_CHARACTERISTIC,
            )
            .await?;
        let glucose_measurement_context_characteristic = session
            .get_service_characteristic_by_uuid(
                &self.device_id,
                GLUCOSE_SERVICE,
                GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC,
            )
            .await?;
        let racp_characteristic = session
            .get_service_characteristic_by_uuid(
                &self.device_id,
                GLUCOSE_SERVICE,
                RACP_CHARACTERISTIC,
            )
            .await?;

        let mut events = session.event_stream().await?;
        session.start_notify(&glucose_characteristic.id).await?;
        session
            .start_notify(&glucose_measurement_context_characteristic.id)
            .await?;
        session.start_notify(&racp_characteristic.id).await?;

        session
            .write_characteristic_value(&racp_characteristic.id, vec![1, 1])
            .await?;

        let mut records = Vec::new();
        while let Ok(Some(bt_event)) = timeout(Duration::from_millis(1000), events.next()).await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                if value.len() == 15 {
                    let year = (value[4] as i32) * 256 + (value[3] as i32);
                    let date = NaiveDate::from_ymd(year, value[5] as u32, value[6] as u32);
                    let time =
                        NaiveTime::from_hms(value[7] as u32, value[8] as u32, value[9] as u32);

                    let glucose = (value[11] as i32) * 256 + (value[12] as i32);
                    records.push(Record::with_values(
                        NaiveDateTime::new(date, time),
                        vec![Value::Glucose(glucose)],
                    ));
                }
            }
        }
        Ok(records)
    }
}
