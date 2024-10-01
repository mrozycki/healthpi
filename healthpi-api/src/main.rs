mod db;

use std::str::FromStr;

use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use healthpi_model::measurement::{Record, ValueType};
use log::{error, info};
use serde::{de, Deserialize};

use crate::db::{
    connection::Connection,
    measurement::{MeasurementRepository, MeasurementRepositoryImpl},
};

fn comma_separated_value_types<'de, D>(deserializer: D) -> Result<Vec<ValueType>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.split(',')
        .map(|s| ValueType::from_str(s).map_err(|e| de::Error::custom(e.to_string())))
        .collect()
}

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(default)]
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

#[post("/")]
async fn post_measurements(
    measurement_repository: web::Data<MeasurementRepositoryImpl>,
    measurements: web::Json<Vec<Record>>,
) -> impl Responder {
    match measurement_repository.store_records(measurements.0).await {
        Ok(_) => {
            info!("Successfully stored records");
            HttpResponse::Created().json(())
        }
        Err(e) => {
            error!("Failed to store records: {e}");
            HttpResponse::InternalServerError().json(())
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    info!("Connecting to database");
    let conn = Connection::establish().await.unwrap();
    let measurement_repository = MeasurementRepositoryImpl::new(conn.clone());

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(measurement_repository.clone()))
            .service(index)
            .service(post_measurements)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
