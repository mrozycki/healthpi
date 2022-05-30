use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

const PERIPHERAL_NAME_MATCH_FILTER: &str = "MI SCALE2";
const SERVICE_INFO_UUID: Uuid = Uuid::from_u128(0x0000181d_0000_1000_8000_00805f9b34fb);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    let adapter = adapter_list.get(0).unwrap();
    println!("Listening...");
    let mut last_weight = 0;
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");

    loop {
        time::sleep(Duration::from_millis(100)).await;
        let peripherals = adapter.peripherals().await?;
        if peripherals.is_empty() {
            continue;
        }

        for peripheral in peripherals.iter() {
            let properties = peripheral.properties().await?.unwrap();
            let local_name = properties.local_name.unwrap_or_default();
            if local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                let service_info = properties.service_data.get(&SERVICE_INFO_UUID).unwrap();
                let weight = ((service_info[2] as usize) * 256 + (service_info[1] as usize)) / 2;
                let stabilized = service_info[0] & 32 != 0;
                let weight_removed = service_info[0] & 128 != 0;
                if weight != last_weight {
                    println!(
                        "Measurement {:3}.{:02}, stabilized: {}, weight removed: {}",
                        weight / 100,
                        weight % 100,
                        stabilized,
                        weight_removed
                    );
                    last_weight = weight;
                }
            }
        }
    }
}
