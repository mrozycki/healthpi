use std::{error::Error, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use healthpi_bt::{BleCharacteristic, BleDevice, MacAddress};
use healthpi_db::measurement::Record;
use log::{debug, error, info};
use tokio::{sync::Mutex, time::timeout};
use uuid::Uuid;

use super::device::Device;

const AH100_CUSTOM_SERVICE: Uuid = Uuid::from_u128(0x0000faa0_0000_1000_8000_00805f9b34fb);
const AH100_CUSTOM_SEND: Uuid = Uuid::from_u128(0x0000faa1_0000_1000_8000_00805f9b34fb);
const AH100_CUSTOM_RECV: Uuid = Uuid::from_u128(0x0000faa2_0000_1000_8000_00805f9b34fb);

#[repr(u8)]
enum Command {
    GetRecords = 11,
    HeartBeat = 32,
    Auth = 36,
    Bind = 37,
}

#[derive(Clone)]
struct AH100Interface {
    mac: MacAddress,
    ble_device: Arc<Mutex<Box<dyn BleDevice>>>,
}

impl AH100Interface {
    fn new(ble_device: Box<dyn BleDevice>) -> Self {
        Self {
            mac: ble_device.mac_address(),
            ble_device: Arc::new(Mutex::new(ble_device)),
        }
    }

    async fn connect(&self) -> Result<(), Box<dyn Error>> {
        self.ble_device.lock().await.connect().await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        self.ble_device.lock().await.disconnect().await?;
        Ok(())
    }

    async fn send_characteristic(&self) -> Box<dyn BleCharacteristic> {
        self.ble_device
            .lock()
            .await
            .get_characteristic(AH100_CUSTOM_SERVICE, AH100_CUSTOM_SEND)
            .await
            .unwrap()
    }

    async fn recv_characteristic(&self) -> Box<dyn BleCharacteristic> {
        self.ble_device
            .lock()
            .await
            .get_characteristic(AH100_CUSTOM_SERVICE, AH100_CUSTOM_RECV)
            .await
            .unwrap()
    }

    fn encode_payload(&self, data: &[u8]) -> Vec<u8> {
        let mac_bytes: [u8; 6] = self.mac.into();
        debug!("Encoding payload {:02X?} with mac {:02X?}", data, mac_bytes);
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ mac_bytes[i % 6])
            .collect()
    }

    async fn command(&self, code: Command, payload: &[u8]) {
        self.send_characteristic()
            .await
            .write_with_response(
                [0xDB, payload.len() as u8 + 1, code as u8]
                    .into_iter()
                    .chain(self.encode_payload(payload))
                    .collect(),
            )
            .await
            .unwrap();
    }

    async fn send_heartbeat(&self) {
        debug!("Sending hearbeat");
        self.command(Command::HeartBeat, &[0]).await
    }

    async fn run_heartbeat(self) {
        debug!("Starting heartbeat");
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = self.send_heartbeat().await;
        }
    }

    async fn auth(&self) {
        debug!("Attempting authorization");
        self.command(Command::Auth, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x10, 0x01])
            .await
    }

    async fn bind(&self) {
        debug!("Attempting bind");
        self.command(Command::Bind, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x10, 0x01])
            .await
    }

    async fn request_measurements(&self) {
        debug!("Requesting measurements");
        self.command(
            Command::GetRecords,
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x10, 0x01, 0x00],
        )
        .await
    }
}

pub struct AH100 {
    name: String,
    mac_address: MacAddress,
    interface: AH100Interface,
}

impl AH100 {
    pub fn new(ble_device: Box<dyn BleDevice>) -> Self {
        Self {
            name: ble_device.display_name().to_owned(),
            mac_address: ble_device.mac_address(),
            interface: AH100Interface::new(ble_device),
        }
    }
}

#[async_trait]
impl Device for AH100 {
    async fn connect(&self) -> Result<(), Box<dyn Error>> {
        self.interface.connect().await
    }

    async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        self.interface.disconnect().await
    }

    async fn get_data(&self) -> Result<Vec<Record>, Box<dyn Error>> {
        let mut events = self
            .interface
            .recv_characteristic()
            .await
            .subscribe()
            .await?;
        events.next().await;
        let _join = {
            debug!("Creating heartbeat thread");
            let interface = self.interface.clone();
            tokio::spawn(async move { interface.run_heartbeat().await })
        };

        info!("Trying to authenticate");
        self.interface.auth().await;
        let raw_payload = self
            .interface
            .encode_payload(&events.next().await.unwrap().value[3..]);
        if raw_payload[0] != 1 {
            info!("Failed to authenticate, trying to bind");
            let mut bind_success = false;
            for _ in 0..3 {
                self.interface.bind().await;
                while let Ok(Some(event)) = timeout(Duration::from_secs(60), events.next()).await {
                    if event.value[2] == 0x27 {
                        info!("Bound successfully");
                        bind_success = true;
                    }
                }
            }
            if !bind_success {
                Err("Failed to bind")?;
            } else {
                info!("Binding successful, trying authentication again");
            }

            let mut auth_success = false;
            for _ in 0..3 {
                self.interface.auth().await;
                let raw_payload = events.next().await.unwrap().value;

                debug!("Authorization response: {:?}", raw_payload);

                let payload = self.interface.encode_payload(&raw_payload[3..]);
                if payload[0] == 1 {
                    info!("Authenticated successfully");
                    auth_success = true;
                }
            }
            if !auth_success {
                Err("Failed to authenticate")?;
            }
        } else {
            info!("Authenticated successfully");
        }

        self.interface.request_measurements().await;
        while let Ok(Some(_event)) = timeout(Duration::from_secs(5), events.next()).await {}

        Ok(Vec::new())
    }

    fn get_device_name(&self) -> &str {
        &self.name
    }

    fn get_device_address(&self) -> MacAddress {
        self.mac_address
    }
}
