pub mod devices;

use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::lock::Mutex;
use healthpi_bt::BleSession;
use log::{debug, error, info, warn};
use tokio::time;

use crate::devices::device::Factory;

pub struct Loader {
    ble_session: Box<dyn BleSession>,
    factory: Arc<Mutex<Box<dyn Factory>>>,
    api_client: Box<dyn healthpi_client::Client>,
    running: Arc<AtomicBool>,
}

impl Loader {
    pub fn new(
        ble_session: Box<dyn BleSession>,
        factory: Box<dyn Factory>,
        api_client: Box<dyn healthpi_client::Client>,
        running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            ble_session,
            factory: Arc::new(Mutex::new(factory)),
            api_client,
            running,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting discovery");
        self.ble_session.start_discovery().await?;

        info!("Waiting for devices");
        while self.running.load(Ordering::Relaxed) {
            time::sleep(Duration::from_secs(1)).await;
            let devices = self.ble_session.get_devices().await?;

            for ble_device in devices.into_iter() {
                let Some(device) = self.factory.lock().await.make_device(ble_device) else {
                    continue;
                };

                info!(
                    "Found device {}, connecting",
                    device.get_ble_device().name()
                );
                match tokio::time::timeout(Duration::from_secs(5), device.connect()).await {
                    Err(_) => {
                        error!("Failed to connect within 5 seconds, skipping");
                        continue;
                    }
                    Ok(Err(e)) => {
                        error!("Failed to connect, skipping: {:?}", e);
                        continue;
                    }
                    _ => {}
                }

                info!("Getting data");
                let records = match device.get_data().await {
                    Ok(records) => records,
                    Err(e) => {
                        error!("Failed to get data: {:?}", e);
                        continue;
                    }
                };
                info!("Fetched {} records", records.len());
                debug!("Last 3 records loaded",);
                records
                    .iter()
                    .rev()
                    .take(3)
                    .for_each(|record| debug!("{:?}", record));

                info!("Disconnecting");
                match tokio::time::timeout(Duration::from_secs(5), device.disconnect()).await {
                    Err(_) => {
                        warn!("Failed to disconnect within 5 seconds");
                    }
                    Ok(Err(e)) => {
                        warn!("Failed to disconnect: {:?}", e);
                    }
                    _ => {}
                }

                info!("Storing records in database");
                if let Err(e) = self.api_client.post_records(&records).await {
                    error!("Failed to store records in database, skipping: {}", e);
                    continue;
                }

                info!("Device processed successfully");
                self.factory.lock().await.mark_processed(device.as_ref());
            }
        }

        info!("Received stop signal, terminating...");
        self.ble_session.stop_discovery().await?;
        Ok(())
    }
}
