use std::env;

use actix_web::{App, HttpServer};
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
    dbg!(&db_url);

    let db = Database::connect(db_url).await.unwrap();

    #[cfg(not(debug_assertions))]
    migration::Migrator::up(&db, None).await.unwrap();

    let db_data = actix_web::web::Data::new(db);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(db_data.to_owned())
            .configure(api::attach)
    })
    .bind(("0.0.0.0", 8080)).unwrap()
    .run().await.unwrap();
}
