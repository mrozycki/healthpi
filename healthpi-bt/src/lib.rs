#[cfg(all(feature = "bluez", feature = "btleplug"))]
compile_error!("\"bluez\" and \"btleplug\" cannot be used at the same time");

mod api;
#[cfg(feature = "bluez")]
mod bluez;
#[cfg(feature = "btleplug")]
mod btleplug;
mod macaddress;

pub use api::{
    BleCharacteristic, BleCharacteristicEvent, BleDevice, BleSession, DeviceError,
    MockBleCharacteristic, MockBleDevice, MockBleSession,
};
#[cfg(feature = "bluez")]
pub use bluez::create_session;
#[cfg(feature = "btleplug")]
pub use btleplug::create_session;
pub use macaddress::MacAddress;
