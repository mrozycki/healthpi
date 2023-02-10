use std::str::FromStr;

use actix_web::{get, web, App, HttpServer, Responder};
use healthpi_db::{
    db::{
        connection::Connection,
        measurement::{MeasurementRepository, MeasurementRepositoryImpl},
    },
    measurement::ValueType,
};
use log::info;
use serde::{de, Deserialize};

fn comma_separated_value_types<'de, D>(deserializer: D) -> Result<Vec<ValueType>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.split(",")
        .map(|s| ValueType::from_str(s).map_err(|e| de::Error::custom(e.to_string())))
        .collect()
}

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(deserialize_with = "comma_separated_value_types")]
    select: Vec<ValueType>,
}

#[get("/")]
async fn index(
    measurement_repository: web::Data<MeasurementRepositoryImpl>,
    query: web::Query<Query>,
) -> impl Responder {
    web::Json(
        measurement_repository
            .fetch_records(&query.select)
            .await
            .unwrap(),
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    info!("Connecting to database");
    let conn = Connection::establish().await.unwrap();
    let measurement_repository = MeasurementRepositoryImpl::new(conn.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(measurement_repository.clone()))
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
