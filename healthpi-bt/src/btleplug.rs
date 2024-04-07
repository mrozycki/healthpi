use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter,
    ValueNotification, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::future;
use futures::{lock::Mutex, Stream, StreamExt};
use healthpi_model::device::DeviceId;
use uuid::Uuid;

use super::api::{BleCharacteristic, BleCharacteristicEvent, BleDevice, BleSession, DeviceError};

impl From<ValueNotification> for BleCharacteristicEvent {
    fn from(value: ValueNotification) -> Self {
        BleCharacteristicEvent { value: value.value }
    }
}

#[derive(Debug)]
struct BleCharacteristicImpl {
    peripheral: Peripheral,
    characteristic: Characteristic,
}

impl BleCharacteristicImpl {
    async fn new(
        peripheral: Peripheral,
        service_uuid: Uuid,
        characteristic_uuid: Uuid,
    ) -> Result<Self, DeviceError> {
        let characteristic = peripheral
            .characteristics()
            .iter()
            .find(|ch| ch.service_uuid == service_uuid && ch.uuid == characteristic_uuid)
            .ok_or(DeviceError::BluetoothError("No such characteristic".into()))?
            .clone();

        Ok(BleCharacteristicImpl {
            peripheral,
            characteristic,
        })
    }

    async fn write_inner(&self, bytes: &[u8], write_type: WriteType) -> Result<(), DeviceError> {
        self.peripheral
            .write(&self.characteristic, bytes, write_type)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }
}

#[async_trait]
impl BleCharacteristic for BleCharacteristicImpl {
    async fn subscribe(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BleCharacteristicEvent> + Send>>, DeviceError> {
        self.peripheral
            .subscribe(&self.characteristic)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?;

        let events = self
            .peripheral
            .notifications()
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
            .map(BleCharacteristicEvent::from);

        Ok(Box::pin(events))
    }

    async fn write(&self, bytes: &[u8]) -> Result<(), DeviceError> {
        self.write_inner(bytes, WriteType::WithoutResponse).await
    }

    async fn write_with_response(&self, bytes: &[u8]) -> Result<(), DeviceError> {
        self.write_inner(bytes, WriteType::WithResponse).await
    }

    async fn read(&self) -> Result<Vec<u8>, DeviceError> {
        self.peripheral
            .read(&self.characteristic)
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }
}

struct BleDeviceImpl {
    peripheral: Peripheral,
    properties: PeripheralProperties,
}

impl BleDeviceImpl {
    async fn new(peripheral: Peripheral) -> Result<Self, DeviceError> {
        let properties = peripheral
            .properties()
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
            .ok_or(DeviceError::BluetoothError(
                "Peripheral properties not available".into(),
            ))?;
        Ok(Self {
            peripheral,
            properties,
        })
    }
}

#[async_trait]
impl BleDevice for BleDeviceImpl {
    async fn connect(&self) -> Result<(), DeviceError> {
        self.peripheral
            .connect()
            .await
            .map_err(|e| DeviceError::ConnectionFailure(e.to_string()))?;

        self.peripheral
            .discover_services()
            .await
            .map_err(|e| DeviceError::ConnectionFailure(e.to_string()))
    }

    async fn disconnect(&self) -> Result<(), DeviceError> {
        self.peripheral
            .disconnect()
            .await
            .map_err(|e| DeviceError::ConnectionFailure(e.to_string()))
    }

    fn in_range(&self) -> bool {
        self.properties.rssi.is_some()
    }

    fn id(&self) -> DeviceId {
        if cfg!(target_os = "macos") {
            DeviceId::new(self.peripheral.id().to_string())
        } else {
            DeviceId::new(self.properties.address.to_string())
        }
    }

    fn name(&self) -> String {
        self.properties
            .local_name
            .clone()
            .unwrap_or(self.id().to_string())
    }

    async fn get_characteristic(
        &self,
        service_id: Uuid,
        characteristic_id: Uuid,
    ) -> Result<Box<dyn BleCharacteristic>, DeviceError> {
        Ok(Box::new(
            BleCharacteristicImpl::new(self.peripheral.clone(), service_id, characteristic_id)
                .await?,
        ))
    }
}

struct BleSessionImpl {
    adapter: Arc<Mutex<Adapter>>,
}

impl BleSessionImpl {
    fn new(adapter: Adapter) -> Self {
        Self {
            adapter: Arc::new(Mutex::new(adapter)),
        }
    }
}

#[async_trait]
impl BleSession for BleSessionImpl {
    async fn start_discovery(&self) -> Result<(), DeviceError> {
        self.adapter
            .lock()
            .await
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }

    async fn stop_discovery(&self) -> Result<(), DeviceError> {
        self.adapter
            .lock()
            .await
            .stop_scan()
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))
    }

    async fn get_devices(&self) -> Result<Vec<Box<dyn BleDevice>>, DeviceError> {
        let futures = self
            .adapter
            .lock()
            .await
            .peripherals()
            .await
            .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
            .into_iter()
            .map(BleDeviceImpl::new);

        Ok(future::join_all(futures)
            .await
            .into_iter()
            .flat_map(Result::ok)
            .map(|d| Box::new(d) as Box<dyn BleDevice>)
            .collect())
    }
}

pub async fn create_session() -> Result<Box<dyn BleSession>, DeviceError> {
    Manager::new()
        .await
        .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
        .adapters()
        .await
        .map_err(|e| DeviceError::BluetoothError(e.to_string()))?
        .into_iter()
        .find(|_| true)
        .ok_or(DeviceError::BluetoothError(
            "No Bluetooth adapters found".into(),
        ))
        .map(|adapter| Box::new(BleSessionImpl::new(adapter)) as Box<dyn BleSession>)
}
