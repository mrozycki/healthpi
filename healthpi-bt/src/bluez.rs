use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use bluez_async::{
    BluetoothEvent, BluetoothSession, CharacteristicEvent, CharacteristicInfo, DeviceId,
    DeviceInfo, DiscoveryFilter, Transport, WriteOptions, WriteType,
};
use futures::{lock::Mutex, stream, Stream, StreamExt};
use uuid::Uuid;

use super::api::{BleCharacteristic, BleCharacteristicEvent, BleDevice, BleSession, DeviceError};
use super::macaddress::MacAddress;

pub struct NotBleCharacteristicEvent;

impl TryFrom<BluetoothEvent> for BleCharacteristicEvent {
    type Error = NotBleCharacteristicEvent;

    fn try_from(value: BluetoothEvent) -> Result<Self, Self::Error> {
        if let BluetoothEvent::Characteristic { event, .. } = value {
            match event {
                CharacteristicEvent::Value { value } => Ok(Self { value }),
                _ => Err(NotBleCharacteristicEvent),
            }
        } else {
            Err(NotBleCharacteristicEvent)
        }
    }
}

#[derive(Debug)]
struct BleCharacteristicImpl {
    session: Arc<Mutex<BluetoothSession>>,
    info: CharacteristicInfo,
}

impl BleCharacteristicImpl {
    async fn new(
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
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?;

        Ok(BleCharacteristicImpl { session, info })
    }
}

#[async_trait]
impl BleCharacteristic for BleCharacteristicImpl {
    async fn subscribe(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BleCharacteristicEvent> + Send>>, DeviceError> {
        let events = self
            .session
            .lock()
            .await
            .characteristic_event_stream(&self.info.id)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
            .flat_map(|event| stream::iter(BleCharacteristicEvent::try_from(event).ok()));

        self.session
            .lock()
            .await
            .start_notify(&self.info.id)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?;

        Ok(Box::pin(events))
    }

    async fn write(&self, bytes: &[u8]) -> Result<(), DeviceError> {
        self.session
            .lock()
            .await
            .write_characteristic_value(&self.info.id, bytes)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }

    async fn write_with_response(&self, bytes: &[u8]) -> Result<(), DeviceError> {
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
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }

    async fn read(&self) -> Result<Vec<u8>, DeviceError> {
        self.session
            .lock()
            .await
            .read_characteristic_value(&self.info.id)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }
}

struct BleDeviceImpl {
    session: Arc<Mutex<BluetoothSession>>,
    device_info: DeviceInfo,
}

impl BleDeviceImpl {
    fn new(session: Arc<Mutex<BluetoothSession>>, device_info: DeviceInfo) -> Self {
        Self {
            session,
            device_info,
        }
    }
}

#[async_trait]
impl BleDevice for BleDeviceImpl {
    async fn connect(&self) -> Result<(), DeviceError> {
        if let Err(e) = self
            .session
            .lock()
            .await
            .connect(&self.device_info.id)
            .await
        {
            Err(DeviceError::ConnectionFailure(e.to_string()))
        } else {
            Ok(())
        }
    }

    async fn disconnect(&self) -> Result<(), DeviceError> {
        if let Err(e) = self
            .session
            .lock()
            .await
            .disconnect(&self.device_info.id)
            .await
        {
            Err(DeviceError::ConnectionFailure(e.to_string()))
        } else {
            Ok(())
        }
    }

    fn in_range(&self) -> bool {
        self.device_info.rssi.is_some()
    }

    fn mac_address(&self) -> MacAddress {
        let raw_mac_address: [u8; 6] = self.device_info.mac_address.into();
        raw_mac_address.into()
    }

    fn name(&self) -> String {
        self.device_info
            .name
            .clone()
            .unwrap_or(self.mac_address().to_string())
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
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }

    async fn stop_discovery(&self) -> Result<(), DeviceError> {
        self.session
            .lock()
            .await
            .stop_discovery()
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
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
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }
}

pub async fn create_session() -> Result<Box<dyn BleSession>, DeviceError> {
    let (_, session) = BluetoothSession::new()
        .await
        .map_err(|e| DeviceError::BluetoothError(e.to_string()))?;

    Ok(Box::new(BleSessionImpl::new(session)))
}
