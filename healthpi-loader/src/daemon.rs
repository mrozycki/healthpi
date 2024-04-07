use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use healthpi_db::connection::Connection;
use healthpi_db::measurement::MeasurementRepositoryImpl;
use log::info;

use healthpi_loader::devices::device::FactoryImpl;
use healthpi_loader::Loader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    info!("Connecting to database");
    let conn = Connection::establish().await?;
    let measurement_repository = Box::new(MeasurementRepositoryImpl::new(conn.clone()));

    info!("Starting Bluetooth session");
    let ble_session = healthpi_bt::create_session().await?;
    let factory = Box::new(FactoryImpl::from_file("devices.csv")?);

    let running = Arc::new(AtomicBool::new(true));
    let loader = Arc::new(Loader::new(
        ble_session,
        factory,
        measurement_repository,
        running.clone(),
    ));
    ctrlc::set_handler(move || running.store(false, Ordering::Relaxed))?;

    loader.run().await
}
