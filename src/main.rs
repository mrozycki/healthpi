#[macro_use]
extern crate diesel;

mod devices;
mod store;

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{collections::HashMap, error::Error};

use bluez_async::{BluetoothSession, DeviceId, DiscoveryFilter, Transport};
use chrono::{DateTime, Utc};
use devices::device::{Device, Factory};
use diesel::{Connection, SqliteConnection};
use dotenv::dotenv;
use log::{debug, error, info};
use tokio::time;

use crate::store::db::measurement::MeasurementRepository;

fn display_device(device: &dyn Device) -> String {
    device
        .get_device_info()
        .name
        .as_ref()
        .unwrap_or(&device.get_device_info().mac_address.to_string())
        .to_owned()
}

pub fn connect_to_database() -> Result<Arc<Mutex<SqliteConnection>>, Box<dyn Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connection = SqliteConnection::establish(&database_url)?;
    Ok(Arc::new(Mutex::new(connection)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    info!("Connecting to database");
    let conn = connect_to_database()?;
    let measurement_repository = MeasurementRepository::new(conn.clone());

    info!("Starting Bluetooth session");
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

    let running = Arc::new(AtomicBool::new(true));
    let running2 = running.clone();
    ctrlc::set_handler(move || running2.store(false, Ordering::Relaxed))?;
    while running.load(Ordering::Relaxed) {
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
                Ok(records) => {
                    info!("Fetched {} records", records.len());
                    debug!("Last 3 records loaded",);
                    records
                        .iter()
                        .rev()
                        .take(3)
                        .for_each(|record| debug!("{:?}", record));

                    info!("Disconnecting");
                    device.disconnect(&session).await?;

                    info!("Storing records in database");
                    if let Err(e) = measurement_repository.store_records(records) {
                        error!("Failed to store records in database, skipping. {}", e);
                        continue;
                    }

                    let backoff_expiry = chrono::Utc::now()
                        .checked_add_signed(chrono::Duration::minutes(5))
                        .unwrap();
                    info!(
                        "Ignoring device {} until {}",
                        display_device(device.as_ref()),
                        backoff_expiry
                    );
                    backoff_table.insert(device.get_device_info().id.clone(), backoff_expiry);
                }
                Err(e) => error!("Failed to get data: {:?}", e),
            }
        }
    }
    info!("Received SIGINT, terminating...");
    session.stop_discovery().await?;

    Ok(())
}
