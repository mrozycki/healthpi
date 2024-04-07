use std::{error::Error, fs::File, io::BufReader};

use healthpi_client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open("data.json")?;
    let values: Vec<_> = serde_json::from_reader(BufReader::new(file))?;
    let client = healthpi_client::create("http://localhost:8080/".to_owned());
    client.post_records(&values).await?;

    Ok(())
}
