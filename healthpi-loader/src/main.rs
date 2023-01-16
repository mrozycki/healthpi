mod devices;

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use devices::device::Factory;
use healthpi_db::db::connection::Connection;
use log::{debug, error, info};
use tokio::time;

use healthpi_db::db::measurement::MeasurementRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    info!("Connecting to database");
    let conn = Connection::establish()?;
    let measurement_repository = MeasurementRepository::new(conn.clone());

    info!("Starting Bluetooth session");
    let ble_session = healthpi_bt::create_session().await?;
    let mut factory = Factory::from_file("devices.csv")?;

    info!("Starting discovery");
    ble_session.start_discovery().await?;

    let running = Arc::new(AtomicBool::new(true));
    let running2 = running.clone();
    ctrlc::set_handler(move || running2.store(false, Ordering::Relaxed))?;

    info!("Waiting for devices");
    while running.load(Ordering::Relaxed) {
        time::sleep(Duration::from_secs(1)).await;
        let mut devices = ble_session.get_devices().await?.into_iter();

        while let Some(device) = devices.next().and_then(|x| factory.make_device(x)) {
            info!(
                "Found device {}, connecting",
                device.get_ble_device().name()
            );
            if let Err(e) = device.connect().await {
                error!("Failed to connect, skipping: {:?}", e);
                continue;
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
            device.disconnect().await?;

            info!("Storing records in database");
            if let Err(e) = measurement_repository.store_records(records) {
                error!("Failed to store records in database, skipping. {}", e);
                continue;
            }

            info!("Device processed successfully");
            factory.mark_processed(device.as_ref());
        }
    }
    info!("Received SIGINT, terminating...");
    ble_session.stop_discovery().await?;

    Ok(())
}
