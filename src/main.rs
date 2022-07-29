mod devices;
mod store;

use bluez_async::BluetoothSession;
use std::error::Error;
use std::time::Duration;
use tokio::time;

use crate::devices::{contour, device::Device};

const PERIPHERAL_NAME_MATCH_FILTER: &str = "Contour";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (_, session) = BluetoothSession::new().await?;

    println!("Listening...");
    loop {
        session.start_discovery().await?;
        time::sleep(Duration::from_millis(1000)).await;
        session.stop_discovery().await?;

        let devices = session.get_devices().await?;

        if let Some(glucometer) = devices
            .into_iter()
            .find(|device| {
                device
                    .name
                    .as_deref()
                    .filter(|name| name.contains(PERIPHERAL_NAME_MATCH_FILTER))
                    .is_some()
            })
            .map(|info| contour::ElitePlus::new(info.id))
        {
            println!("Found device");
            glucometer.connect(&session).await?;
            println!("Connected");

            println!("Getting data");
            let data = glucometer.get_data(&session).await?;
            println!("Fetched: {:?}", data);

            return Ok(());
        }
    }
}
