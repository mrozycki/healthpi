use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::{collections::HashSet, error::Error, fs::File};

use async_trait::async_trait;
use bluez_async::{BluetoothSession, DeviceId, DeviceInfo, MacAddress};
use chrono::{DateTime, Utc};
use log::{debug, info, warn};

use super::{contour, soehnle};
use crate::store::measurement::Record;

#[async_trait]
pub trait Device {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>>;
    async fn disconnect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>>;
    async fn get_data(&self, session: &BluetoothSession) -> Result<Vec<Record>, Box<dyn Error>>;
    fn get_device_info(&self) -> &DeviceInfo;
}

pub fn display_device(device_info: &DeviceInfo) -> String {
    device_info
        .name
        .as_ref()
        .unwrap_or(&device_info.mac_address.to_string())
        .to_owned()
}

struct BackoffTable {
    expiry_timestamps: HashMap<DeviceId, DateTime<Utc>>,
}

impl BackoffTable {
    fn new() -> Self {
        Self {
            expiry_timestamps: HashMap::<DeviceId, DateTime<Utc>>::new(),
        }
    }

    fn check(&self, device_id: &DeviceId) -> bool {
        self.expiry_timestamps
            .get(device_id)
            .filter(|expiry| expiry > &&chrono::Utc::now())
            .is_some()
    }

    fn mark(&mut self, device_id: &DeviceId) -> DateTime<Utc> {
        let backoff_expiry = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(5))
            .unwrap();
        self.expiry_timestamps
            .insert(device_id.clone(), backoff_expiry);
        backoff_expiry
    }
}

pub struct Factory {
    paired_devices: HashSet<MacAddress>,
    backoff_table: BackoffTable,
}

impl Factory {
    #[allow(dead_code)]
    pub fn new(paired_devices: HashSet<MacAddress>) -> Self {
        Self {
            paired_devices,
            backoff_table: BackoffTable::new(),
        }
    }

    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let paired_devices: HashSet<MacAddress> = BufReader::new(file)
            .lines()
            .filter_map(|l| l.ok())
            .map(|s| MacAddress::from_str(&s))
            .filter_map(|l| l.ok())
            .collect();

        info!("Loaded {} paired devices from file", paired_devices.len());
        debug!("Loaded devices: {:?}", paired_devices);
        Ok(Self::new(paired_devices))
    }

    pub fn make_device(&self, device_info: DeviceInfo) -> Option<Box<dyn Device>> {
        if device_info.rssi.is_none() {
            None
        } else if !self.paired_devices.contains(&device_info.mac_address) {
            None
        } else if self.backoff_table.check(&device_info.id) {
            debug!(
                "Found device {}, ignoring because of backoff",
                display_device(&device_info)
            );
            None
        } else if let Some(name) = &device_info.name {
            if name.contains("Contour") {
                Some(Box::new(contour::ElitePlus::new(device_info)))
            } else if name.contains("Shape200") {
                Some(Box::new(soehnle::Shape200::new(device_info)))
            } else if name.contains("Systo MC 400") {
                Some(Box::new(soehnle::SystoMC400::new(device_info)))
            } else {
                warn!(
                    "Device with MAC={} is not of any supported types",
                    device_info.mac_address
                );
                None
            }
        } else {
            None
        }
    }

    pub fn mark_processed(&mut self, device: &dyn Device) -> DateTime<Utc> {
        let expiry = self.backoff_table.mark(&device.get_device_info().id);
        info!(
            "Ignoring device {} until {}",
            display_device(device.get_device_info()),
            expiry
        );
        expiry
    }
}
