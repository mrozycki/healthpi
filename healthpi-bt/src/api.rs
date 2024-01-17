use std::{error::Error, fmt, pin::Pin};

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

use super::macaddress::MacAddress;

#[derive(Debug)]
pub enum DeviceError {
    ConnectionFailure(String),
    BluetoothError(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DeviceError {}

pub struct BleCharacteristicEvent {
    pub value: Vec<u8>,
}

#[mockall::automock]
#[async_trait]
pub trait BleCharacteristic: Send + Sync + fmt::Debug {
    async fn subscribe(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BleCharacteristicEvent> + Send>>, DeviceError>;
    async fn write(&self, bytes: &[u8]) -> Result<(), DeviceError>;
    async fn write_with_response(&self, bytes: &[u8]) -> Result<(), DeviceError>;
    async fn read(&self) -> Result<Vec<u8>, DeviceError>;
}

#[mockall::automock]
#[async_trait]
pub trait BleDevice: Send + Sync {
    async fn connect(&self) -> Result<(), DeviceError>;
    async fn disconnect(&self) -> Result<(), DeviceError>;

    fn in_range(&self) -> bool;
    fn mac_address(&self) -> MacAddress;
    fn name(&self) -> String;

    async fn get_characteristic(
        &self,
        service_id: Uuid,
        characteristic_id: Uuid,
    ) -> Result<Box<dyn BleCharacteristic>, DeviceError>;
}

#[mockall::automock]
#[async_trait]
pub trait BleSession: Send + Sync {
    async fn start_discovery(&self) -> Result<(), DeviceError>;
    async fn stop_discovery(&self) -> Result<(), DeviceError>;

    async fn get_devices(&self) -> Result<Vec<Box<dyn BleDevice>>, DeviceError>;
}
