use std::{error::Error, fs::File, io::BufReader};

use healthpi_db::{
    connection::Connection,
    measurement::{MeasurementRepository, MeasurementRepositoryImpl},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open("data.json")?;
    let values = serde_json::from_reader(BufReader::new(file))?;
    let db = Connection::establish().await?;
    let repository = MeasurementRepositoryImpl::new(db);
    repository.store_records(values).await?;

    Ok(())
}
