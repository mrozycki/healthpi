use std::time::Duration;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use futures::StreamExt;
use healthpi_bt::{BleCharacteristicEvent, BleDevice, MacAddress};
use healthpi_db::measurement::{Record, Source, Value};
use healthpi_db::user::User;
use log::{debug, info};
use tokio::time::timeout;
use uuid::Uuid;

use crate::devices::utils;

use super::device::Device;

const WEIGHT_CUSTOM_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3001_28e9_40b8_a361_6db4cca4147c);
const CMD_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3002_28e9_40b8_a361_6db4cca4147c);
const CUSTOM_SERVICE_UUID: Uuid = Uuid::from_u128(0x352e3000_28e9_40b8_a361_6db4cca4147c);
const BLOOD_PRESSURE_SERVICE: Uuid = Uuid::from_u128(0x00001810_0000_1000_8000_00805f9b34fb);
const BLOOD_PRESSURE_CHARACTERISTIC: Uuid = Uuid::from_u128(0x00002a35_0000_1000_8000_00805f9b34fb);

pub struct Shape200 {
    ble_device: Box<dyn BleDevice>,
}

impl Shape200 {
    pub fn new(ble_device: Box<dyn BleDevice>) -> Self {
        Self { ble_device }
    }

    fn read_record(&self, user: &User, event: BleCharacteristicEvent) -> Option<Record> {
        if event.value.len() != 15 {
            return None;
        }
        let timestamp = utils::naive_date_time_from_be_bytes(&event.value[2..9])?;

        let weight = u16::from_be_bytes([event.value[9], event.value[10]]) as f64 / 10.0;
        let mut values = vec![
            Value::Weight(weight),
            Value::BodyMassIndex(get_body_mass_index(user, weight)),
            Value::BasalMetabolicRate(get_basal_metabolic_rate(user, weight)),
        ];

        let imp5 = u16::from_be_bytes([event.value[11], event.value[12]]) as f64;
        let imp50 = u16::from_be_bytes([event.value[13], event.value[14]]) as f64;
        if imp50 > 0.0 && imp50 < 1600.0 {
            // The upper bound for imp50 is not exact. A value of 1600 would contribute 100%
            // to the body fat calculation, so any value above that is likely a result
            // of incorrect measurement.
            values.append(&mut vec![
                Value::FatPercent(get_fat_percentage(user, weight, imp50)),
                Value::WaterPercent(get_water_percentage(user, weight, imp50)),
                Value::MusclePercent(get_muscle_percentage(user, weight, imp5, imp50)),
            ]);
        }
        Some(Record::new(
            timestamp,
            values,
            event.value,
            Source::Device(self.ble_device.mac_address()),
        ))
    }
}

#[async_trait]
impl Device for Shape200 {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
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
        let weight_characteristic = self
            .ble_device
            .get_characteristic(CUSTOM_SERVICE_UUID, WEIGHT_CUSTOM_CHARACTERISTIC)
            .await?;
        let cmd_characteristic = self
            .ble_device
            .get_characteristic(CUSTOM_SERVICE_UUID, CMD_CHARACTERISTIC)
            .await?;

        info!("Subscribing to notifications");
        let mut events = weight_characteristic.subscribe().await?;
        cmd_characteristic
            .write_with_response(vec![0x0c, 1])
            .await?;

        info!("Reading user data");
        let user = if let Some(event) = events.next().await {
            User::new(
                event.value[3],
                event.value[4] != 0,
                u16::from_be_bytes([event.value[5], event.value[6]]),
                event.value[9],
            )
        } else {
            panic!("Did not receive user data!")
        };

        cmd_characteristic
            .write_with_response(vec![0x09, 1])
            .await?;

        info!("Processing measurement notifications");
        let mut records = Vec::new();
        while let Ok(Some(event)) = timeout(Duration::from_millis(1000), events.next()).await {
            if let Some(record) = self.read_record(&user, event) {
                records.push(record);
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
    ble_device: Box<dyn BleDevice>,
}

impl SystoMC400 {
    pub fn new(ble_device: Box<dyn BleDevice>) -> Self {
        Self { ble_device }
    }

    fn read_record(raw_data: Vec<u8>, mac_address: MacAddress) -> Option<Record> {
        let mut i = 1;

        let mut values = Vec::new();
        let systolic_raw: u32 = u16::from_be_bytes([raw_data[i + 1], raw_data[i]]).into();
        let diastolic_raw: u32 = u16::from_be_bytes([raw_data[i + 3], raw_data[i + 2]]).into();
        let (systolic, diastolic) = if raw_data[0] & 1 == 0 {
            (systolic_raw, diastolic_raw)
        } else {
            (systolic_raw * 15 / 2000, diastolic_raw * 15 / 2000)
        };
        values.append(&mut vec![
            Value::BloodPressureSystolic(systolic as i32),
            Value::BloodPressureDiastolic(diastolic as i32),
        ]);
        i += 6;

        let timestamp = if raw_data[0] & 2 == 0 {
            Utc::now().naive_local()
        } else {
            let t = utils::naive_date_time_from_le_bytes(&raw_data[i..i + 7])?;
            i += 7;
            t
        };

        if raw_data[0] & 4 != 0 {
            let heart_rate = u16::from_be_bytes([raw_data[i + 1], raw_data[i]]);
            values.push(Value::HeartRate(heart_rate as i32));
        }

        let raw_mac_address: [u8; 6] = mac_address.into();

        Some(Record::new(
            timestamp,
            values,
            raw_data,
            Source::Device(raw_mac_address.into()),
        ))
    }
}

#[async_trait]
impl Device for SystoMC400 {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ble_device.connect().await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.ble_device.disconnect().await?;
        Ok(())
    }

    async fn get_data(&self) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
        info!("Finding appropriate characteristics");
        let measurements = self
            .ble_device
            .get_characteristic(BLOOD_PRESSURE_SERVICE, BLOOD_PRESSURE_CHARACTERISTIC)
            .await?;
        debug!("Got: {:?}", measurements);

        info!("Subscribing to notifications");
        let mut events = measurements.subscribe().await?;

        info!("Processing notifications");
        let mut records = Vec::new();
        let mut prev_timestamp = NaiveDateTime::MIN;
        let mut timestamp_duplicate_count = 0;
        while let Ok(Some(event)) = timeout(Duration::from_millis(5000), events.next()).await {
            if let Some(mut record) = Self::read_record(event.value, self.ble_device.mac_address())
            {
                if record.timestamp == prev_timestamp {
                    timestamp_duplicate_count += 1;
                    record.timestamp += chrono::Duration::seconds(timestamp_duplicate_count);
                } else {
                    timestamp_duplicate_count = 0;
                    prev_timestamp = record.timestamp
                }
                records.push(record);
            }
        }
        debug!("Processed all events, produced {} records", records.len());

        Ok(records)
    }

    fn get_ble_device(&self) -> &dyn BleDevice {
        &*self.ble_device
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::*;

    #[test]
    fn read_record_all_fields_present() {
        let mac_address = MacAddress::from([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);
        let raw_data = vec![
            30, 128, 0, 75, 0, 93, 0, 230, 7, 8, 4, 13, 49, 0, 80, 0, 0, 0, 0,
        ];
        let expected = Record::new(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2022, 8, 4).unwrap(),
                NaiveTime::from_hms_opt(13, 49, 0).unwrap(),
            ),
            vec![
                Value::BloodPressureSystolic(128),
                Value::BloodPressureDiastolic(75),
                Value::HeartRate(80),
            ],
            raw_data.clone(),
            Source::Device(mac_address),
        );

        let record = SystoMC400::read_record(raw_data.clone(), mac_address).unwrap();

        assert_eq!(record, expected);
    }

    #[test]
    fn read_record_all_fields_present_kpa() {
        let mac_address = MacAddress::from([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);
        // 128 mmHg = 17100 Pa = 66 * 256 + 204
        // 75 mmHg = 10000 Pa = 39 * 256 + 16
        let raw_data = vec![
            31, 204, 66, 16, 39, 93, 0, 230, 7, 8, 4, 13, 49, 0, 80, 0, 0, 0, 0,
        ];
        let expected = Record::new(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2022, 8, 4).unwrap(),
                NaiveTime::from_hms_opt(13, 49, 0).unwrap(),
            ),
            vec![
                Value::BloodPressureSystolic(128),
                Value::BloodPressureDiastolic(75),
                Value::HeartRate(80),
            ],
            raw_data.clone(),
            Source::Device(mac_address),
        );

        let record = SystoMC400::read_record(raw_data.clone(), mac_address).unwrap();

        assert_eq!(record, expected);
    }

    #[test]
    fn read_record_without_timestamp() {
        let mac_address = MacAddress::from([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);
        let raw_data = vec![28, 128, 0, 75, 0, 93, 0, 80, 0, 0, 0, 0];
        let expected_values = vec![
            Value::BloodPressureSystolic(128),
            Value::BloodPressureDiastolic(75),
            Value::HeartRate(80),
        ];

        let record = SystoMC400::read_record(raw_data.clone(), mac_address).unwrap();

        assert_eq!(record.raw_data, raw_data);
        assert_eq!(record.source, Source::Device(mac_address));
        assert_eq!(record.values, expected_values);
    }

    #[test]
    fn read_record_without_heart_rate() {
        let mac_address = MacAddress::from([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);
        let raw_data = vec![26, 128, 0, 75, 0, 93, 0, 230, 7, 8, 4, 13, 49, 0, 0, 0, 0];
        let expected = Record::new(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2022, 8, 4).unwrap(),
                NaiveTime::from_hms_opt(13, 49, 0).unwrap(),
            ),
            vec![
                Value::BloodPressureSystolic(128),
                Value::BloodPressureDiastolic(75),
            ],
            raw_data.clone(),
            Source::Device(mac_address),
        );

        let record = SystoMC400::read_record(raw_data.clone(), mac_address).unwrap();

        assert_eq!(record, expected);
    }
}
