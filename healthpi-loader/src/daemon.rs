use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use log::info;

use healthpi_loader::devices::device::FactoryImpl;
use healthpi_loader::Loader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    let api_client = Box::new(healthpi_client::create("http://localhost:8080/".to_owned()));

    info!("Starting Bluetooth session");
    let ble_session = healthpi_bt::create_session().await?;
    let factory = Box::new(FactoryImpl::from_file("devices.csv")?);

    let running = Arc::new(AtomicBool::new(true));
    let loader = Arc::new(Loader::new(
        ble_session,
        factory,
        api_client,
        running.clone(),
    ));
    ctrlc::set_handler(move || running.store(false, Ordering::Relaxed))?;

    loader.run().await
}
