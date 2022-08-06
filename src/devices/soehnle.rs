use std::time::Duration;

use async_trait::async_trait;
use bluez_async::{
    BluetoothEvent, BluetoothSession, CharacteristicEvent, DeviceInfo, WriteOptions, WriteType,
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures::StreamExt;
use log::debug;
use tokio::time::timeout;
use uuid::Uuid;

use crate::store::measurement::{Record, Value};
use crate::store::user::User;

use super::device::Device;

const WEIGHT_CUSTOM_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3001_28e9_40b8_a361_6db4cca4147c);
const CMD_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3002_28e9_40b8_a361_6db4cca4147c);
const CUSTOM_SERVICE_UUID: Uuid = Uuid::from_u128(0x352e3000_28e9_40b8_a361_6db4cca4147c);
const BLOOD_PRESSURE_SERVICE: Uuid = Uuid::from_u128(0x00001810_0000_1000_8000_00805f9b34fb);
const BLOOD_PRESSURE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a35_0000_1000_8000_00805f9b34fb);

pub struct Shape200 {
    device_info: DeviceInfo,
}

impl Shape200 {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }
}

#[async_trait]
impl Device for Shape200 {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn std::error::Error>> {
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
        let weight_service = session
            .get_service_by_uuid(&self.device_info.id, CUSTOM_SERVICE_UUID)
            .await?;
        let weight_characteristic = session
            .get_characteristic_by_uuid(&weight_service.id, WEIGHT_CUSTOM_CHARACTERISTIC)
            .await?;
        let cmd_characteristic = session
            .get_characteristic_by_uuid(&weight_service.id, CMD_CHARACTERISTIC)
            .await?;

        let mut events = session
            .characteristic_event_stream(&weight_characteristic.id)
            .await?;
        session.start_notify(&weight_characteristic.id).await?;
        session
            .write_characteristic_value_with_options(
                &cmd_characteristic.id,
                vec![0x0c, 1],
                WriteOptions {
                    offset: 0,
                    write_type: Some(WriteType::WithResponse),
                },
            )
            .await?;

        let mut records = Vec::new();

        let user = if let Some(bt_event) = events.next().await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                User::new(
                    value[3],
                    value[4] != 0,
                    u16::from_be_bytes([value[5], value[6]]),
                    value[9],
                )
            } else {
                panic!("Wrong data received!")
            }
        } else {
            panic!("Did not receive user data!")
        };

        session
            .write_characteristic_value_with_options(
                &cmd_characteristic.id,
                vec![0x09, 1],
                WriteOptions {
                    offset: 0,
                    write_type: Some(WriteType::WithResponse),
                },
            )
            .await?;
        while let Ok(Some(bt_event)) = timeout(Duration::from_millis(1000), events.next()).await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                if value.len() == 15 {
                    let year = u16::from_be_bytes([value[2], value[3]]);
                    let date = NaiveDate::from_ymd(year as i32, value[4] as u32, value[5] as u32);
                    let time =
                        NaiveTime::from_hms(value[6] as u32, value[7] as u32, value[8] as u32);

                    let weight = u16::from_be_bytes([value[9], value[10]]) as f64 / 10.0;
                    let mut values = vec![Value::Weight(weight)];
                    let imp5 = u16::from_be_bytes([value[11], value[12]]);
                    let imp50 = u16::from_be_bytes([value[13], value[14]]);
                    if imp50 > 0 {
                        let fat_percentage = get_fat_percentage(&user, weight, imp50 as f64);
                        let water_percentage = get_water_percentage(&user, weight, imp50 as f64);
                        let muscle_percentage =
                            get_muscle_percentage(&user, weight, imp5 as f64, imp50 as f64);
                        values.append(&mut vec![
                            Value::FatPercent(fat_percentage),
                            Value::WaterPercent(water_percentage),
                            Value::MusclePercent(muscle_percentage),
                        ]);
                    }

                    records.push(Record::with_values(NaiveDateTime::new(date, time), values))
                }
            }
        }

        Ok(records)
    }
}

fn get_water_percentage(user: &User, weight: f64, imp50: f64) -> f64 {
    let activity_correction_factor = match (user.activity_level(), user.is_female()) {
        (1..=3, true) => 0.0,
        (1..=3, false) => 2.83,
        (4, true) => 0.4,
        (4, false) => 3.93,
        (5, true) => 1.4,
        (5, false) => 5.33,
        _ => 0.0,
    };

    (0.3674 * (user.height() as f64).powf(2.0) / imp50 + 0.17530 * weight
        - 0.11 * user.age() as f64
        + (6.53 + activity_correction_factor))
        / weight
        * 100.0
}

fn get_muscle_percentage(user: &User, weight: f64, imp5: f64, imp50: f64) -> f64 {
    let activity_correction_factor = match (user.activity_level(), user.is_female()) {
        (1..=3, true) => 0.0,
        (1..=3, false) => 3.6224,
        (4, true) => 0.0,
        (4, false) => 4.3904,
        (5, true) => 1.664,
        (5, false) => 5.4144,
        _ => 0.0,
    };
    ((0.47027 / imp50 - 0.24196 / imp5) * (user.height() as f64).powf(2.0) + 0.13796 * weight
        - 0.1152 * user.age() as f64
        + (5.12 + activity_correction_factor))
        / weight
        * 100.0
}

fn get_fat_percentage(user: &User, weight: f64, imp50: f64) -> f64 {
    let activity_correction_factor = match (user.activity_level(), user.is_female()) {
        (4, true) => 2.3,
        (4, false) => 2.5,
        (5, true) => 4.1,
        (5, false) => 4.3,
        _ => 0.0,
    };

    let (sex_correction_factor, activity_sex_div) = if user.is_female() {
        (0.214, 55.1)
    } else {
        (0.250, 65.5)
    };

    1.847 * weight * 10000.0 / ((user.height() as f64).powf(2.0))
        + sex_correction_factor * user.age() as f64
        + 0.062 * imp50
        - (activity_sex_div - activity_correction_factor)
}

pub struct SystoMC400 {
    device_info: DeviceInfo,
}

impl SystoMC400 {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }
}

#[async_trait]
impl Device for SystoMC400 {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn std::error::Error>> {
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

    async fn get_data(
        &self,
        session: &BluetoothSession,
    ) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
        debug!("Getting measurements characteristic");
        let measurements = session
            .get_service_characteristic_by_uuid(
                &self.device_info.id,
                BLOOD_PRESSURE_SERVICE,
                BLOOD_PRESSURE_CHARACTERISTIC,
            )
            .await?;
        debug!("Got: {:?}", &measurements.id);

        debug!("Listening for events");
        let mut events = session
            .characteristic_event_stream(&measurements.id)
            .await?;
        session.start_notify(&measurements.id).await?;

        let mut records = Vec::new();

        debug!("Waiting for events");
        while let Ok(Some(bt_event)) = timeout(Duration::from_millis(5000), events.next()).await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                debug!("Received characteristic event: {:?}", value);
                let systolic = u16::from_be_bytes([value[2], value[1]]);
                let diastolic = u16::from_be_bytes([value[4], value[3]]);
                let heart_rate = u16::from_be_bytes([value[15], value[14]]);

                let year = u16::from_be_bytes([value[8], value[7]]);
                let date = NaiveDate::from_ymd(year as i32, value[9] as u32, value[10] as u32);
                let time =
                    NaiveTime::from_hms(value[11] as u32, value[12] as u32, value[13] as u32);
                let timestamp = NaiveDateTime::new(date, time);

                records.push(Record::with_values(
                    timestamp,
                    vec![
                        Value::BloodPressureSystolic(systolic as i32),
                        Value::BloodPressureDiastolic(diastolic as i32),
                        Value::HeartRate(heart_rate as i32),
                    ],
                ))
            } else {
                debug!("Received unexpected type of event");
            }
        }
        debug!("Processed all events, produced {} records", records.len());

        Ok(records)
    }

    fn get_device_info(&self) -> &DeviceInfo {
        &self.device_info
    }
}
