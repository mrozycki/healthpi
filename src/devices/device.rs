use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::{collections::HashSet, error::Error, fs::File};

use async_trait::async_trait;
use bluez_async::{BluetoothSession, DeviceInfo, MacAddress};
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

pub struct Factory {
    paired_devices: HashSet<MacAddress>,
}

impl Factory {
    #[allow(dead_code)]
    pub fn new(paired_devices: HashSet<MacAddress>) -> Self {
        Self { paired_devices }
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
        Ok(Self { paired_devices })
    }

    pub fn make_device(&self, device_info: DeviceInfo) -> Option<Box<dyn Device>> {
        if device_info.rssi.is_none() {
            None
        } else if !self.paired_devices.contains(&device_info.mac_address) {
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
}
