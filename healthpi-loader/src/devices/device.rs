use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::{collections::HashSet, error::Error, fs::File};

use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use healthpi_bt::{BleDevice, MacAddress};
use log::{debug, info, warn};

use healthpi_db::measurement::Record;

use super::{contour, soehnle};

#[async_trait]
pub trait Device {
    async fn connect(&self) -> Result<(), Box<dyn Error>>;
    async fn disconnect(&self) -> Result<(), Box<dyn Error>>;
    async fn get_data(&self) -> Result<Vec<Record>, Box<dyn Error>>;
    fn get_ble_device(&self) -> &dyn BleDevice;
}

struct BackoffTable {
    expiry_timestamps: HashMap<MacAddress, DateTime<Utc>>,
}

impl BackoffTable {
    fn new() -> Self {
        Self {
            expiry_timestamps: HashMap::<MacAddress, DateTime<Utc>>::new(),
        }
    }

    fn check(&self, device: &dyn BleDevice) -> bool {
        self.expiry_timestamps
            .get(&device.mac_address())
            .filter(|expiry| expiry > &&chrono::Utc::now())
            .is_some()
    }

    fn mark(&mut self, device: &dyn BleDevice) -> DateTime<Utc> {
        let backoff_expiry = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(5))
            .unwrap();
        self.expiry_timestamps
            .insert(device.mac_address(), backoff_expiry);
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

    pub fn make_device(&self, ble_device: Box<dyn BleDevice>) -> Option<Box<dyn Device>> {
        if !ble_device.in_range() {
            None
        } else if !self.paired_devices.contains(&ble_device.mac_address()) {
            None
        } else if self.backoff_table.check(&*ble_device) {
            debug!(
                "Found device {}, ignoring because of backoff",
                ble_device.name()
            );
            None
        } else if ble_device.name().contains("Contour") {
            Some(Box::new(contour::ElitePlus::new(ble_device)))
        } else if ble_device.name().contains("Shape200") {
            Some(Box::new(soehnle::Shape200::new(ble_device)))
        } else if ble_device.name().contains("Systo MC 400") {
            Some(Box::new(soehnle::SystoMC400::new(ble_device)))
        } else {
            warn!(
                "Device with MAC={} is not of any supported types",
                ble_device.mac_address()
            );
            None
        }
    }

    pub fn mark_processed(&mut self, device: &dyn Device) -> DateTime<Utc> {
        let expiry = self.backoff_table.mark(device.get_ble_device());
        info!(
            "Ignoring device {} until {}",
            device.get_ble_device().name(),
            expiry.with_timezone(&Local)
        );
        expiry
    }
}
