use std::time::Duration;

use async_trait::async_trait;
use bluez_async::{
    BluetoothEvent, BluetoothSession, CharacteristicEvent, DeviceInfo, WriteOptions, WriteType,
};
use chrono::Utc;
use futures::StreamExt;
use log::{debug, info};
use tokio::time::timeout;
use uuid::Uuid;

use healthpi_db::measurement::{Record, Source, Value};
use healthpi_db::user::User;

use crate::devices::utils;

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
        info!("Finding appropriate characteristics");
        let weight_service = session
            .get_service_by_uuid(&self.device_info.id, CUSTOM_SERVICE_UUID)
            .await?;
        let weight_characteristic = session
            .get_characteristic_by_uuid(&weight_service.id, WEIGHT_CUSTOM_CHARACTERISTIC)
            .await?;
        let cmd_characteristic = session
            .get_characteristic_by_uuid(&weight_service.id, CMD_CHARACTERISTIC)
            .await?;

        info!("Subscribing to notifications");
        let mut events = session
            .characteristic_event_stream(&weight_characteristic.id)
            .await?;
        session.start_notify(&weight_characteristic.id).await?;
        session
            .write_characteristic_value_with_options(
                &cmd_characteristic.id,
                vec![0x0c, 1],
                WriteOptions {
                    write_type: Some(WriteType::WithResponse),
                    ..Default::default()
                },
            )
            .await?;

        info!("Reading user data");
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
                    write_type: Some(WriteType::WithResponse),
                    ..Default::default()
                },
            )
            .await?;

        info!("Processing measurement notifications");
        let raw_mac_addres: [u8; 6] = self.device_info.mac_address.into();
        let mut records = Vec::new();
        while let Ok(Some(bt_event)) = timeout(Duration::from_millis(1000), events.next()).await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                if value.len() == 15 {
                    let timestamp = utils::naive_date_time_from_be_bytes(&value[2..9]);

                    let weight = u16::from_be_bytes([value[9], value[10]]) as f64 / 10.0;
                    let mut values = vec![
                        Value::Weight(weight),
                        Value::BodyMassIndex(get_body_mass_index(&user, weight)),
                        Value::BasalMetabolicRate(get_basal_metabolic_rate(&user, weight)),
                    ];

                    let imp5 = u16::from_be_bytes([value[11], value[12]]) as f64;
                    let imp50 = u16::from_be_bytes([value[13], value[14]]) as f64;
                    if imp50 > 0.0 {
                        values.append(&mut vec![
                            Value::FatPercent(get_fat_percentage(&user, weight, imp50)),
                            Value::WaterPercent(get_water_percentage(&user, weight, imp50)),
                            Value::MusclePercent(get_muscle_percentage(&user, weight, imp5, imp50)),
                        ]);
                    }

                    records.push(Record::new(
                        timestamp,
                        values,
                        value,
                        Source::Device(raw_mac_addres.into()),
                    ))
                }
            }
        }
        debug!("Processed all events, produced {} records", records.len());

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

    (0.3674 * (user.height_cm() as f64).powf(2.0) / imp50 + 0.17530 * weight
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
    ((0.47027 / imp50 - 0.24196 / imp5) * (user.height_cm() as f64).powf(2.0) + 0.13796 * weight
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

    1.847 * weight / user.height_m().powf(2.0)
        + sex_correction_factor * user.age() as f64
        + 0.062 * imp50
        - (activity_sex_div - activity_correction_factor)
}

fn get_body_mass_index(user: &User, weight: f64) -> f64 {
    weight / user.height_m().powf(2.0)
}

fn get_basal_metabolic_rate(user: &User, weight: f64) -> f64 {
    if user.is_female() {
        447.593 + 9.247 * weight + 3.098 * user.height_cm() as f64 - 4.330 * user.age() as f64
    } else {
        88.362 + 13.397 * weight + 4.799 * user.height_cm() as f64 - 5.677 * user.age() as f64
    }
}

pub struct SystoMC400 {
    device_info: DeviceInfo,
}

impl SystoMC400 {
    pub fn new(device_info: DeviceInfo) -> Self {
        Self { device_info }
    }

    fn read_record(&self, raw_data: Vec<u8>) -> Record {
        let mut i = 1;

        let mut values = Vec::new();
        let systolic_raw = u16::from_be_bytes([raw_data[i + 1], raw_data[i]]);
        let diastolic_raw = u16::from_be_bytes([raw_data[i + 3], raw_data[i + 2]]);
        let (systolic, diastolic) = if raw_data[0] & 1 == 0 {
            (systolic_raw, diastolic_raw)
        } else {
            (systolic_raw * 15 / 2, diastolic_raw * 15 / 2)
        };
        values.append(&mut vec![
            Value::BloodPressureSystolic(systolic as i32),
            Value::BloodPressureDiastolic(diastolic as i32),
        ]);
        i += 6;

        let timestamp = if raw_data[0] & 2 == 0 {
            Utc::now().naive_local()
        } else {
            let t = utils::naive_date_time_from_le_bytes(&raw_data[i..i + 7]);
            i += 7;
            t
        };

        if raw_data[0] & 4 != 0 {
            let heart_rate = u16::from_be_bytes([raw_data[i + 1], raw_data[i]]);
            values.push(Value::HeartRate(heart_rate as i32));
        }

        let raw_mac_address: [u8; 6] = self.device_info.mac_address.into();

        Record::new(
            timestamp,
            values,
            raw_data,
            Source::Device(raw_mac_address.into()),
        )
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
        info!("Finding appropriate characteristics");
        let measurements = session
            .get_service_characteristic_by_uuid(
                &self.device_info.id,
                BLOOD_PRESSURE_SERVICE,
                BLOOD_PRESSURE_CHARACTERISTIC,
            )
            .await?;
        debug!("Got: {:?}", &measurements.id);

        info!("Subscribing to notifications");
        let mut events = session
            .characteristic_event_stream(&measurements.id)
            .await?;
        session.start_notify(&measurements.id).await?;

        info!("Processing notifications");
        let mut records = Vec::new();
        while let Ok(Some(bt_event)) = timeout(Duration::from_millis(5000), events.next()).await {
            if let BluetoothEvent::Characteristic {
                event: CharacteristicEvent::Value { value },
                ..
            } = bt_event
            {
                records.push(self.read_record(value));
            }
        }
        debug!("Processed all events, produced {} records", records.len());

        Ok(records)
    }

    fn get_device_info(&self) -> &DeviceInfo {
        &self.device_info
    }
}
