use bluez_async::{BluetoothEvent, BluetoothSession, CharacteristicEvent, WriteOptions, WriteType};
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
const CUSTOM_SERVICE_UUID: Uuid = Uuid::from_u128(0x352e3000_28e9_40b8_a361_6db4cca4147c);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (_, session) = BluetoothSession::new().await?;

    loop {
        println!("Listening...");
        session.start_discovery().await?;
        println!("Started discovery...");
        time::sleep(Duration::from_millis(1000)).await;
        session.stop_discovery().await?;
        println!("Discovery done");

        let devices = session.get_devices().await?;

        if let Some(scale) = devices
            .into_iter()
            .find(|device| device.name.as_deref() == Some(PERIPHERAL_NAME_MATCH_FILTER))
        {
            println!("Connecting to peripheral");
            session.connect(&scale.id).await?;
            println!("Connected");

            let weight_service = session
                .get_service_by_uuid(&scale.id, CUSTOM_SERVICE_UUID)
                .await?;
            let characteristics = session.get_characteristics(&weight_service.id).await?;
            let weight_characteristic_a = session
                .get_characteristic_by_uuid(&weight_service.id, WEIGHT_CUSTOM_A_CHARACTERISTIC)
                .await?;
            let weight_characteristic_b = session
                .get_characteristic_by_uuid(&weight_service.id, WEIGHT_CUSTOM_B_CHARACTERISTIC)
                .await?;

            let cmd_characteristic = session
                .get_characteristic_by_uuid(&weight_service.id, CMD_CHARACTERISTIC)
                .await?;
            session
                .write_characteristic_value_with_options(
                    &cmd_characteristic.id,
                    vec![0x09, 1],
                    WriteOptions {
                        offset: 0,
                        write_type: Some(WriteType::WithResponse),
                    },
                )
                .await?;

            let mut events = session
                .characteristic_event_stream(&weight_characteristic_a.id)
                .await?;
            session.start_notify(&weight_characteristic_a.id).await?;

            while let Some(bt_event) = events.next().await {
                if let BluetoothEvent::Characteristic {
                    id,
                    event: CharacteristicEvent::Value { value },
                } = bt_event
                {
                    if value.len() == 15 {
                        let year = (value[2] as usize) * 256 + (value[3] as usize);
                        let month = value[4];
                        let day = value[5];
                        let weight = ((value[9] as f64) * 256.0 + (value[10] as f64)) / 10.0;
                        println!(
                            "Received data from: {}-{:02}-{:02}, {} kg",
                            year, month, day, weight
                        );
                    }
                }
            }

            return Ok(());
        }
    }
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

/*
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
*/
