mod devices;
mod store;

use std::error::Error;
use std::time::Duration;

use bluez_async::BluetoothSession;
use tokio::time;

use crate::devices::device::make_device;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (_, session) = BluetoothSession::new().await?;

    session.start_discovery().await?;
    loop {
        println!("Discovering...");
        time::sleep(Duration::from_secs(1)).await;

        let mut devices = session.get_devices().await?.into_iter();

        while let Some(device) = devices.next().and_then(make_device) {
            println!(
                "Found device {}, connecting",
                device
                    .get_device_info()
                    .name
                    .as_ref()
                    .unwrap_or(&device.get_device_info().mac_address.to_string())
            );
            if let Err(_) = device.connect(&session).await {
                eprintln!("Failed to connect, skipping");
                continue;
            }

            println!("Getting data");
            let data = device.get_data(&session).await?;
            println!(
                "Fetched {} records, last 5: {:?}",
                data.len(),
                data.iter().rev().take(5).collect::<Vec<_>>()
            );

            device.disconnect(&session).await?;
        }
    }
}
