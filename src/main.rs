use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use std::collections::BTreeSet;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

const PERIPHERAL_NAME_MATCH_FILTER: &str = "Shape200";
const WEIGHT_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000181d_0000_1000_8000_00805f9b34fb);
const WEIGHT_CUSTOM_A_CHARACTERISTIC: Uuid =
    Uuid::from_u128(0x352e3001_28e9_40b8_a361_6db4cca4147c);
const WEIGHT_CUSTOM_B_CHARACTERISTIC: Uuid =
    Uuid::from_u128(0x352e3004_28e9_40b8_a361_6db4cca4147c);
const CMD_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3002_28e9_40b8_a361_6db4cca4147c);

const NOTIFY_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x6e400002_b534_f393_67a9_e50e24dccA9e);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    let adapter = adapter_list.get(0).expect("No Bluetooth adapters found");

    println!("Listening...");
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");

    loop {
        time::sleep(Duration::from_millis(1000)).await;
        let peripherals = adapter.peripherals().await?;
        if peripherals.is_empty() {
            continue;
        }

        for peripheral in peripherals.iter() {
            let properties = peripheral.properties().await?.unwrap();
            let local_name = properties.local_name.clone().unwrap_or_default();
            if !local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                continue;
            }
            println!("Found peripheral, connecting...");

            peripheral.connect().await?;
            println!("Connected to peripheral");

            peripheral.discover_services().await?;
            println!("Discovered services");
            let weight_characteristic_a = peripheral
                .characteristics()
                .into_iter()
                .find(|ch| ch.uuid == WEIGHT_CUSTOM_A_CHARACTERISTIC)
                .expect("Expected to find the weight characteristic");
            let weight_characteristic_b = peripheral
                .characteristics()
                .into_iter()
                .find(|ch| ch.uuid == WEIGHT_CUSTOM_B_CHARACTERISTIC)
                .expect("Expected to find the weight characteristic");

            println!("Found weight characteristics, subscribing...");
            peripheral.subscribe(&weight_characteristic_a).await?;
            peripheral.subscribe(&weight_characteristic_b).await?;
            println!("Subscribed to weight characteristics, awaiting notifications...");

            let cmd_characteristic = peripheral
                .characteristics()
                .into_iter()
                .find(|ch| ch.uuid == CMD_CHARACTERISTIC)
                .expect("Expected to find the cmd characteristic");
            peripheral
                .write(&cmd_characteristic, &[0x09, 1], WriteType::WithResponse)
                .await?;

            let mut notifications = peripheral.notifications().await?;
            while let Some(data) = notifications.next().await {
                if data.value.len() == 15 {
                    let year = (data.value[2] as usize) * 256 + (data.value[3] as usize);
                    let month = data.value[4];
                    let day = data.value[5];
                    let weight = ((data.value[9] as f64) * 256.0 + (data.value[10] as f64)) / 10.0;
                    /*println!(
                        "Received data from {:?}: {}-{:02}-{:02}, {} kg",
                        local_name, year, month, day, weight
                    );*/
                    println!("Received data from {:?}: {:?}", local_name, data.value);
                } else {
                    println!("Received data from {:?}: {:?}", local_name, data.value);
                }
            }

            println!("Disconnecting from peripheral");
            peripheral.disconnect().await?;
            return Ok(());
        }
    }
    // Ok(())
}

/*
// Current Time
Characteristic { uuid: 00002a0f-0000-1000-8000-00805f9b34fb, service_uuid: 00001805-0000-1000-8000-00805f9b34fb, properties: READ | WRITE }
Characteristic { uuid: 00002a14-0000-1000-8000-00805f9b34fb, service_uuid: 00001805-0000-1000-8000-00805f9b34fb, properties: READ }
Characteristic { uuid: 00002a2b-0000-1000-8000-00805f9b34fb, service_uuid: 00001805-0000-1000-8000-00805f9b34fb, properties: READ | WRITE | NOTIFY }

// Ago
Characteristic { uuid: 00002a80-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ | WRITE }

// Gender
Characteristic { uuid: 00002a8c-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ | WRITE }

// Height
Characteristic { uuid: 00002a8e-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ | WRITE }

// Weight
Characteristic { uuid: 00002a98-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ | WRITE }

// Database Change Increment
Characteristic { uuid: 00002a99-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ | WRITE | NOTIFY }

// User Index
Characteristic { uuid: 00002a9a-0000-1000-8000-00805f9b34fb, service_uuid: 0000181c-0000-1000-8000-00805f9b34fb, properties: READ }

// Body Composition Feature
Characteristic { uuid: 00002a9b-0000-1000-8000-00805f9b34fb, service_uuid: 0000181b-0000-1000-8000-00805f9b34fb, properties: READ }

// Body Composition Measurement ?
Characteristic { uuid: 00002a9c-0000-1000-8000-00805f9b34fb, service_uuid: 0000181b-0000-1000-8000-00805f9b34fb, properties: INDICATE }

// Weight Measurement
Characteristic { uuid: 00002a9d-0000-1000-8000-00805f9b34fb, service_uuid: 0000181d-0000-1000-8000-00805f9b34fb, properties: INDICATE }
*/

async fn track_mi_scale(adapter: &Adapter) -> Result<(), Box<dyn Error>> {
    println!("Listening...");
    let mut events = adapter.events().await?;
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");

    while let Some(event) = events.next().await {
        println!("{:#?}", event);
        match event {
            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                if let Ok(peripheral) = adapter.peripheral(&id).await {
                    let properties = peripheral.properties().await?.unwrap();
                    let local_name = properties.local_name.unwrap_or_default();
                    if local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                        let weight_data = service_data.get(&WEIGHT_SERVICE_UUID).unwrap();
                        let weight =
                            ((weight_data[2] as usize) * 256 + (weight_data[1] as usize)) / 2;
                        let stabilized = weight_data[0] & 32 != 0;
                        let weight_removed = weight_data[0] & 128 != 0;
                        println!(
                            "Measurement {:3}.{:02}, stabilized: {}, weight removed: {}",
                            weight / 100,
                            weight % 100,
                            stabilized,
                            weight_removed
                        );
                    }
                } else {
                    eprintln!("Unknown peripheral id={:?}", id);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
