use std::error::Error;

use async_trait::async_trait;
use bluez_async::{BluetoothSession, DeviceInfo};

use super::{contour, soehnle};
use crate::store::measurement::Record;

#[async_trait]
pub trait Device {
    async fn connect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>>;
    async fn disconnect(&self, session: &BluetoothSession) -> Result<(), Box<dyn Error>>;
    async fn get_data(&self, session: &BluetoothSession) -> Result<Vec<Record>, Box<dyn Error>>;
    fn get_device_info(&self) -> &DeviceInfo;
}

pub fn make_device(device_info: DeviceInfo) -> Option<Box<dyn Device>> {
    if device_info.rssi.is_none() {
        None
    } else if let Some(name) = &device_info.name {
        if name.contains("Contour") {
            Some(Box::new(contour::ElitePlus::new(device_info)))
        } else if name.contains("Shape200") {
            Some(Box::new(soehnle::Shape200::new(device_info)))
        } else if name.contains("Systo MC 400") {
            Some(Box::new(soehnle::SystoMC400::new(device_info)))
        } else {
            None
        }
    } else {
        None
    }
}
