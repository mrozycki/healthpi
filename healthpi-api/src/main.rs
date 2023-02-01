use actix_web::{get, web, App, HttpServer, Responder};
use healthpi_db::db::{
    connection::Connection,
    measurement::{MeasurementRepository, MeasurementRepositoryImpl},
};
use log::info;

#[get("/")]
async fn index(measurement_repository: web::Data<MeasurementRepositoryImpl>) -> impl Responder {
    web::Json(measurement_repository.fetch_records().await.unwrap())
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
