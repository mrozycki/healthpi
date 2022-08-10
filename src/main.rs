mod devices;
mod store;

use std::time::Duration;
use std::{collections::HashMap, error::Error};

use bluez_async::{BluetoothSession, DeviceId, DiscoveryFilter, Transport};
use chrono::{DateTime, Utc};
use devices::device::{Device, Factory};
use log::{debug, error, info};
use tokio::time;

fn display_device(device: &dyn Device) -> String {
    device
        .get_device_info()
        .name
        .as_ref()
        .unwrap_or(&device.get_device_info().mac_address.to_string())
        .to_owned()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;
    let (_, session) = BluetoothSession::new().await?;
    let factory = Factory::from_file("devices.csv")?;

    info!("Starting discovery");
    session
        .start_discovery_with_filter(&DiscoveryFilter {
            transport: Some(Transport::Le),
            duplicate_data: Some(false),
            ..Default::default()
        })
        .await?;

    let mut backoff_table = HashMap::<DeviceId, DateTime<Utc>>::new();

    loop {
        debug!("Waiting for devices...");
        time::sleep(Duration::from_secs(1)).await;

        let mut devices = session.get_devices().await?.into_iter();

        while let Some(device) = devices.next().and_then(|x| factory.make_device(x)) {
            if backoff_table
                .get(&device.get_device_info().id)
                .filter(|expiry| expiry > &&chrono::Utc::now())
                .is_some()
            {
                debug!(
                    "Found device {}, ignoring because of backoff",
                    display_device(device.as_ref())
                );
                continue;
            }
            info!(
                "Found device {}, connecting",
                display_device(device.as_ref())
            );
            if let Err(e) = device.connect(&session).await {
                error!("Failed to connect, skipping: {:?}", e);
                continue;
            }

            info!("Getting data from {}", display_device(device.as_ref()));
            match device.get_data(&session).await {
                Ok(data) => {
                    info!("Fetched {} records", data.len());
                    debug!("Last 5 records loaded",);
                    data.iter()
                        .rev()
                        .take(5)
                        .for_each(|record| debug!("{:?}", record));

                    let backoff_expiry = chrono::Utc::now()
                        .checked_add_signed(chrono::Duration::minutes(5))
                        .unwrap();
                    info!(
                        "Ignoring device {} until {}",
                        display_device(device.as_ref()),
                        backoff_expiry
                    );
                    backoff_table.insert(device.get_device_info().id.clone(), backoff_expiry);

                    info!("Disconnecting");
                    device.disconnect(&session).await?;
                }
                Err(e) => error!("Failed to get data: {:?}", e),
            }
        }
    }
}
