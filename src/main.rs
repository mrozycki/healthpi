#[macro_use]
extern crate diesel;

mod devices;
mod store;

use std::env;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bluez_async::{BluetoothSession, DiscoveryFilter, Transport};
use devices::device::Factory;
use diesel::{Connection, SqliteConnection};
use dotenv::dotenv;
use log::{debug, error, info};
use tokio::time;

use crate::devices::device::display_device;
use crate::store::db::measurement::MeasurementRepository;

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
    let mut factory = Factory::from_file("devices.csv")?;

    info!("Starting discovery");
    session
        .start_discovery_with_filter(&DiscoveryFilter {
            transport: Some(Transport::Le),
            duplicate_data: Some(false),
            ..Default::default()
        })
        .await?;

    let running = Arc::new(AtomicBool::new(true));
    let running2 = running.clone();
    ctrlc::set_handler(move || running2.store(false, Ordering::Relaxed))?;
    while running.load(Ordering::Relaxed) {
        debug!("Waiting for devices...");
        time::sleep(Duration::from_secs(1)).await;

        let mut devices = session.get_devices().await?.into_iter();

        while let Some(device) = devices.next().and_then(|x| factory.make_device(x)) {
            info!(
                "Found device {}, connecting",
                display_device(device.get_device_info())
            );
            if let Err(e) = device.connect(&session).await {
                error!("Failed to connect, skipping: {:?}", e);
                continue;
            }

            info!(
                "Getting data from {}",
                display_device(device.get_device_info())
            );
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
                    factory.mark_processed(device.as_ref());
                }
                Err(e) => error!("Failed to get data: {:?}", e),
            }
        }
    }
    info!("Received SIGINT, terminating...");
    session.stop_discovery().await?;

    Ok(())
}
