mod devices;
mod store;

use std::time::Duration;
use std::{collections::HashMap, error::Error};

use bluez_async::{BluetoothSession, DeviceId, DiscoveryFilter, Transport};
use chrono::{DateTime, Utc};
use devices::device::Device;
use tokio::time;

use crate::devices::device::make_device;

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
    let (_, session) = BluetoothSession::new().await?;

    session
        .start_discovery_with_filter(&DiscoveryFilter {
            transport: Some(Transport::Le),
            duplicate_data: Some(false),
            ..DiscoveryFilter::default()
        })
        .await?;

    let mut backoff_table = HashMap::<DeviceId, DateTime<Utc>>::new();

    loop {
        println!("Discovering...");
        time::sleep(Duration::from_secs(1)).await;

        let mut devices = session.get_devices().await?.into_iter();

        while let Some(device) = devices.next().and_then(make_device) {
            if backoff_table
                .get(&device.get_device_info().id)
                .filter(|expiry| expiry > &&chrono::Utc::now())
                .is_some()
            {
                println!(
                    "Found device {}, ignoring because of backoff",
                    display_device(device.as_ref())
                );
                continue;
            }
            println!(
                "Found device {}, connecting",
                display_device(device.as_ref())
            );
            if let Err(e) = device.connect(&session).await {
                eprintln!("Failed to connect, skipping: {:?}", e);
                continue;
            }

            println!("Getting data");
            match device.get_data(&session).await {
                Ok(data) => {
                    println!(
                        "Fetched {} records, last 5: {:?}",
                        data.len(),
                        data.iter().rev().take(5).collect::<Vec<_>>()
                    );
                    backoff_table.insert(
                        device.get_device_info().id.clone(),
                        chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::minutes(5))
                            .unwrap(),
                    );

                    device.disconnect(&session).await?;
                }
                Err(e) => eprintln!("Failed to get data: {:?}", e),
            }
        }
    }
}
