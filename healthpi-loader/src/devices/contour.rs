use std::{collections::BTreeMap, error::Error, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use healthpi_bt::{BleCharacteristicEvent, BleDevice};
use healthpi_db::measurement::{MealIndicator, Record, Source, Value};
use log::{debug, info};
use tokio::time::timeout;
use uuid::Uuid;

use crate::devices::utils;

use super::device::Device;

const GLUCOSE_SERVICE: Uuid = Uuid::from_u128(0x00001808_0000_1000_8000_00805f9b34fb);
const GLUCOSE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a18_0000_1000_8000_00805f9b34fb);
const GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC: Uuid =
    Uuid::from_u128(0x00002a34_0000_1000_8000_00805f9b34fb);
const RACP_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a52_0000_1000_8000_00805f9b34fb);

pub struct ElitePlus {
    ble_device: Box<dyn BleDevice>,
}

impl ElitePlus {
    pub fn new(ble_device: Box<dyn BleDevice>) -> Self {
        Self { ble_device }
    }

    fn read_record(&self, event: BleCharacteristicEvent) -> Option<(u16, Record)> {
        if event.value.len() < 13 {
            return None;
        }
        let sequence_number = u16::from_be_bytes([event.value[2], event.value[1]]);
        let timestamp = utils::naive_date_time_from_le_bytes(&event.value[3..10])?;
        let glucose = u16::from_be_bytes([event.value[11], event.value[12]]);
        Some((
            sequence_number,
            Record::new(
                timestamp,
                vec![Value::Glucose(glucose as i32)],
                event.value,
                Source::Device(self.ble_device.mac_address()),
            ),
        ))
    }
}

#[async_trait]
impl Device for ElitePlus {
    async fn connect(&self) -> Result<(), Box<dyn Error>> {
        self.ble_device.connect().await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ble_device.disconnect().await?;
        Ok(())
    }

    fn get_ble_device(&self) -> &dyn BleDevice {
        &*self.ble_device
    }

    async fn get_data(&self) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
        info!("Finding appropriate characteristics");
        let measurements = self
            .ble_device
            .get_characteristic(GLUCOSE_SERVICE, GLUCOSE_CHARACTERISTIC)
            .await?;
        let contexts = self
            .ble_device
            .get_characteristic(GLUCOSE_SERVICE, GLUCOSE_MEASUREMENT_CONTEXT_CHARACTERISTIC)
            .await?;
        let racp = self
            .ble_device
            .get_characteristic(GLUCOSE_SERVICE, RACP_CHARACTERISTIC)
            .await?;

        info!("Subscribing to notifications");
        let mut measurement_events = measurements.subscribe().await?;
        let mut context_events = contexts.subscribe().await?;
        let _ = racp.subscribe().await?;

        racp.write(vec![1, 1]).await?;

        let mut records = BTreeMap::<u16, Record>::new();
        info!("Processing measurement notifications");
        while let Ok(Some(event)) = timeout(Duration::from_secs(1), measurement_events.next()).await
        {
            if let Some((sequence_number, record)) = self.read_record(event) {
                records.insert(sequence_number, record);
            }
        }

        info!("Processing context notifications");
        while let Ok(Some(event)) = timeout(Duration::from_secs(1), context_events.next()).await {
            let sequence_number = u16::from_be_bytes([event.value[2], event.value[1]]);
            let flags_field = event.value[0];
            let meal_field_index = 3 + (flags_field >> 7 & 1) + 2 * (flags_field & 1);
            if (flags_field >> 1 & 1) > 0 {
                let meal = match event.value[meal_field_index as usize] {
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
        debug!("Processed all events, produced {} records", records.len());

        Ok(records.into_values().collect())
    }
}
