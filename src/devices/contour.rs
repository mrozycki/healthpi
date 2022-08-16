use std::{collections::BTreeMap, error::Error, time::Duration};

use async_trait::async_trait;
use bluez_async::{BluetoothEvent, BluetoothSession, CharacteristicEvent, DeviceInfo};
use futures::StreamExt;
use log::{debug, info};
use tokio::time::timeout;
use uuid::Uuid;

use crate::{
    devices::utils,
    store::measurement::{MealIndicator, Record, Source, Value},
};

use super::device::Device;

const GLUCOSE_SERVICE: Uuid = Uuid::from_u128(0x00001808_0000_1000_8000_00805f9b34fb);
const GLUCOSE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a18_0000_1000_8000_00805f9b34fb);
const GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC: Uuid =
    Uuid::from_u128(0x00002a34_0000_1000_8000_00805f9b34fb);
const RACP_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a52_0000_1000_8000_00805f9b34fb);

pub struct ElitePlus {
    device_info: DeviceInfo,
}

impl ElitePlus {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }
}

#[async_trait]
impl Device for ElitePlus {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>> {
        session.connect(&self.device_info.id).await?;
        Ok(())
    }

    async fn disconnect(
        &self,
        session: &BluetoothSession,
    ) -> Result<(), Box<dyn std::error::Error>> {
        session.disconnect(&self.device_info.id).await?;
        Ok(())
    }

    fn get_device_info(&self) -> &DeviceInfo {
        &self.device_info
    }

    async fn get_data(
        &self,
        session: &BluetoothSession,
    ) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
        info!("Finding appropriate characteristics");
        let measurements = session
            .get_service_characteristic_by_uuid(
                &self.device_info.id,
                GLUCOSE_SERVICE,
                GLUCOSE_CHARACTERISTIC,
            )
            .await?;
        let contexts = session
            .get_service_characteristic_by_uuid(
                &self.device_info.id,
                GLUCOSE_SERVICE,
                GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC,
            )
            .await?;
        let racp = session
            .get_service_characteristic_by_uuid(
                &self.device_info.id,
                GLUCOSE_SERVICE,
                RACP_CHARACTERISTIC,
            )
            .await?;

        info!("Subscribing to notifications");
        let mut measurement_events = session
            .characteristic_event_stream(&measurements.id)
            .await?;
        session.start_notify(&measurements.id).await?;
        let mut context_events = session.characteristic_event_stream(&contexts.id).await?;
        session.start_notify(&contexts.id).await?;
        session.start_notify(&racp.id).await?;

        session
            .write_characteristic_value(&racp.id, vec![1, 1])
            .await?;

        let mut records = BTreeMap::<u16, Record>::new();
        info!("Processing measurement notifications");
        while let Ok(Some(bt_event)) =
            timeout(Duration::from_secs(1), measurement_events.next()).await
        {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                let sequence_number = u16::from_be_bytes([value[2], value[1]]);
                let timestamp = utils::naive_date_time_from_le_bytes(&value[3..10]);

                let glucose = u16::from_be_bytes([value[11], value[12]]);
                records.insert(
                    sequence_number,
                    Record::new(
                        timestamp,
                        vec![Value::Glucose(glucose as i32)],
                        value,
                        Source::Device(self.device_info.mac_address),
                    ),
                );
            }
        }

        info!("Processing context notifications");
        while let Ok(Some(bt_event)) = timeout(Duration::from_secs(1), context_events.next()).await
        {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                let sequence_number = u16::from_be_bytes([value[2], value[1]]);
                let flags_field = value[0];
                let meal_field_index = 3 + (flags_field >> 7 & 1) + 2 * (flags_field & 1);
                if (flags_field >> 1 & 1) > 0 {
                    let meal = match value[meal_field_index as usize] {
                        1 => MealIndicator::BeforeMeal,
                        2 => MealIndicator::AfterMeal,
                        3 => MealIndicator::NoMeal,
                        _ => MealIndicator::NoIndication,
                    };
                    if let Some(record) = records.get_mut(&sequence_number) {
                        record.add_value(Value::Meal(meal))
                    }
                }
            }
        }
        debug!("Processed all events, produced {} records", records.len());

        Ok(records.into_values().collect())
    }
}
