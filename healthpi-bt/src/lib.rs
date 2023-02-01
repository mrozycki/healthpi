use std::{error::Error, fmt, pin::Pin, str::FromStr, sync::Arc};

use async_trait::async_trait;
use bluez_async::{
    BluetoothError, BluetoothEvent, BluetoothSession, CharacteristicEvent, CharacteristicInfo,
    DeviceId, DeviceInfo, DiscoveryFilter, Transport, WriteOptions, WriteType,
};
use futures::{lock::Mutex, stream, Stream, StreamExt};
use log::debug;
use uuid::Uuid;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct MacAddress([u8; 6]);

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
}

impl From<MacAddress> for [u8; 6] {
    fn from(mac: MacAddress) -> Self {
        mac.0
    }
}

impl FromStr for MacAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MacAddress(
            s.split(':')
                .map(|octet| {
                    if octet.len() != 2 {
                        Err(format!("Invalid octet \"{}\" in MAC address {}", octet, s))
                    } else {
                        u8::from_str_radix(octet, 16).map_err(|_| {
                            format!("Invalid octet \"{}\" in MAC address {}", octet, s)
                        })
                    }
                })
                .collect::<Result<Vec<u8>, _>>()?
                .try_into()
                .map_err(|_| format!("Invalid MAC address: {}", s))?,
        ))
    }
}

#[derive(Debug)]
pub enum DeviceError {
    ConnectionFailure(String),
    BluezError(BluetoothError),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DeviceError {}

#[derive(Debug)]
pub struct BleCharacteristicEvent {
    pub value: Vec<u8>,
}

impl BleCharacteristicEvent {
    fn new(bluez_event: BluetoothEvent) -> Option<Self> {
        if let BluetoothEvent::Characteristic { event, .. } = bluez_event {
            match event {
                CharacteristicEvent::Value { value } => Some(Self { value }),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[mockall::automock]
#[async_trait]
pub trait BleCharacteristic: Send + Sync + fmt::Debug {
    async fn subscribe(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BleCharacteristicEvent> + Send + Sync>>, DeviceError>;
    async fn write(&self, bytes: Vec<u8>) -> Result<(), DeviceError>;
    async fn write_with_response(&self, bytes: Vec<u8>) -> Result<(), DeviceError>;
    async fn read(&self) -> Result<Vec<u8>, DeviceError>;
}

#[derive(Debug)]
struct BleCharacteristicImpl {
    session: Arc<Mutex<BluetoothSession>>,
    info: CharacteristicInfo,
}

impl BleCharacteristicImpl {
    pub async fn new(
        session: Arc<Mutex<BluetoothSession>>,
        device_id: &DeviceId,
        service_uuid: Uuid,
        characteristic_uuid: Uuid,
    ) -> Result<Self, DeviceError> {
        let info = session
            .lock()
            .await
            .get_service_characteristic_by_uuid(device_id, service_uuid, characteristic_uuid)
            .await
            .map_err(DeviceError::BluezError)?;

        Ok(BleCharacteristicImpl { session, info })
    }
}

#[async_trait]
impl BleCharacteristic for BleCharacteristicImpl {
    async fn subscribe(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BleCharacteristicEvent> + Send + Sync>>, DeviceError>
    {
        debug!("Subscribing to {:?}", self.info.uuid);
        let uuid = self.info.uuid.clone();
        let events = self
            .session
            .lock()
            .await
            .characteristic_event_stream(&self.info.id)
            .await
            .map_err(DeviceError::BluezError)?
            .flat_map(|event| stream::iter(BleCharacteristicEvent::new(event)))
            .inspect(move |event| {
                debug!(
                    "Received event on characteristic {:?}: {:02X?}",
                    uuid, event.value
                )
            });

        self.session
            .lock()
            .await
            .start_notify(&self.info.id)
            .await
            .map_err(DeviceError::BluezError)?;

        Ok(Box::pin(events))
    }

    async fn write(&self, bytes: Vec<u8>) -> Result<(), DeviceError> {
        debug!(
            "Writing to characteristic {:?}: {:02X?}",
            self.info.uuid, bytes
        );
        self.session
            .lock()
            .await
            .write_characteristic_value(&self.info.id, bytes)
            .await
            .map_err(DeviceError::BluezError)
    }

    async fn write_with_response(&self, bytes: Vec<u8>) -> Result<(), DeviceError> {
        debug!(
            "Writing with response to characteristic {:?}: {:02X?}",
            self.info.uuid, bytes
        );
        self.session
            .lock()
            .await
            .write_characteristic_value_with_options(
                &self.info.id,
                bytes,
                WriteOptions {
                    write_type: Some(WriteType::WithResponse),
                    ..Default::default()
                },
            )
            .await
            .map_err(DeviceError::BluezError)
    }

    async fn read(&self) -> Result<Vec<u8>, DeviceError> {
        self.session
            .lock()
            .await
            .read_characteristic_value(&self.info.id)
            .await
            .map_err(DeviceError::BluezError)
    }
}

#[mockall::automock]
#[async_trait]
pub trait BleDevice: Send + Sync {
    async fn connect(&self) -> Result<(), DeviceError>;
    async fn disconnect(&self) -> Result<(), DeviceError>;

    fn in_range(&self) -> bool;
    fn mac_address(&self) -> MacAddress;
    fn display_name(&self) -> &str;

    async fn get_characteristic(
        &self,
        service_id: Uuid,
        characteristic_id: Uuid,
    ) -> Result<Box<dyn BleCharacteristic>, DeviceError>;
}

pub struct BleDeviceImpl {
    session: Arc<Mutex<BluetoothSession>>,
    display_name: String,
    device_info: DeviceInfo,
}

impl BleDeviceImpl {
    fn new(session: Arc<Mutex<BluetoothSession>>, device_info: DeviceInfo) -> Self {
        Self {
            session,
            display_name: device_info
                .name
                .clone()
                .unwrap_or(device_info.mac_address.to_string()),
            device_info,
        }
    }
}

#[async_trait]
impl BleDevice for BleDeviceImpl {
    async fn connect(&self) -> Result<(), DeviceError> {
        debug!("Connecting to {:?}", self.device_info.mac_address);
        self.session
            .lock()
            .await
            .connect(&self.device_info.id)
            .await
            .map_err(|e| DeviceError::ConnectionFailure(e.to_string()))
    }

    async fn disconnect(&self) -> Result<(), DeviceError> {
        debug!("Disconnecting from {:?}", self.device_info.mac_address);
        self.session
            .lock()
            .await
            .disconnect(&self.device_info.id)
            .await
            .map_err(|e| DeviceError::ConnectionFailure(e.to_string()))
    }

    fn in_range(&self) -> bool {
        self.device_info.rssi.is_some()
    }

    fn mac_address(&self) -> MacAddress {
        let raw_mac_address: [u8; 6] = self.device_info.mac_address.into();
        raw_mac_address.into()
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn get_characteristic(
        &self,
        service_id: Uuid,
        characteristic_id: Uuid,
    ) -> Result<Box<dyn BleCharacteristic>, DeviceError> {
        Ok(Box::new(
            BleCharacteristicImpl::new(
                self.session.clone(),
                &self.device_info.id,
                service_id,
                characteristic_id,
            )
            .await?,
        ))
    }
}

#[mockall::automock]
#[async_trait]
pub trait BleSession: Send + Sync {
    async fn start_discovery(&self) -> Result<(), DeviceError>;
    async fn stop_discovery(&self) -> Result<(), DeviceError>;

    async fn get_devices(&self) -> Result<Vec<Box<dyn BleDevice>>, DeviceError>;
}

struct BleSessionImpl {
    session: Arc<Mutex<BluetoothSession>>,
}

impl BleSessionImpl {
    fn new(session: BluetoothSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }
}

#[async_trait]
impl BleSession for BleSessionImpl {
    async fn start_discovery(&self) -> Result<(), DeviceError> {
        self.session
            .lock()
            .await
            .start_discovery_with_filter(&DiscoveryFilter {
                transport: Some(Transport::Le),
                ..Default::default()
            })
            .await
            .map_err(DeviceError::BluezError)
    }

    async fn stop_discovery(&self) -> Result<(), DeviceError> {
        self.session
            .lock()
            .await
            .stop_discovery()
            .await
            .map_err(DeviceError::BluezError)
    }

    async fn get_devices(&self) -> Result<Vec<Box<dyn BleDevice>>, DeviceError> {
        self.session
            .lock()
            .await
            .get_devices()
            .await
            .map(|devices| {
                devices
                    .into_iter()
                    .map(|d| {
                        Box::new(BleDeviceImpl::new(self.session.clone(), d)) as Box<dyn BleDevice>
                    })
                    .collect::<Vec<Box<dyn BleDevice>>>()
            })
            .map_err(DeviceError::BluezError)
    }
}

pub async fn create_session() -> Result<Box<dyn BleSession>, DeviceError> {
    let (_, session) = BluetoothSession::new()
        .await
        .map_err(DeviceError::BluezError)?;

    Ok(Box::new(BleSessionImpl::new(session)))
}
