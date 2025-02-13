use std::env;

use actix_web::{App, HttpServer};
use aws_config::{BehaviorVersion, Region};
use sea_orm::Database;
use tracing_actix_web::TracingLogger;

#[cfg(not(debug_assertions))]
use migration::MigratorTrait as _;

mod api;
mod error;
mod util;

#[actix_web::main]
async fn main() {
    dotenvy::dotenv_override().unwrap();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "debug");
    }

    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").unwrap();
    let aws_config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await
        .to_builder()
        .region(Some(Region::new("ap-southeast-1")))
        .build();

    let db = Database::connect(db_url).await.unwrap();
    let aws_client = aws_sdk_s3::Client::new(&aws_config);

    #[cfg(not(debug_assertions))]
    migration::Migrator::up(&db, None).await.unwrap();

    let db_data = actix_web::web::Data::new(db);
    let aws_client_data = actix_web::web::Data::new(aws_client);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(db_data.to_owned())
            .app_data(aws_client_data.to_owned())
            .configure(api::attach)
    })
    .bind(("0.0.0.0", 8080)).unwrap()
    .run().await.unwrap();
}
