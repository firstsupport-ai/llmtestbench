use std::env;

use actix_web::{App, HttpServer};
use aws_config::{BehaviorVersion, Region};
use resend_rs::Resend;
use sea_orm::Database;
use supabase_auth::models::AuthClient;
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

    let port = env::var("PORT").map(|v| v.parse::<u16>()).unwrap_or(Ok(8080)).unwrap();
    let db_url = env::var("DATABASE_URL").unwrap();
    let aws_config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await
        .to_builder()
        .region(Some(Region::new("ap-southeast-1")))
        .build();
    let resend_key = env::var("RESEND_KEY").unwrap();

    let db = Database::connect(db_url).await.unwrap();
    let aws_client = aws_sdk_s3::Client::new(&aws_config);
    let supabase_client = AuthClient::new_from_env().unwrap();
    let resend_client = Resend::new(&resend_key);

    #[cfg(not(debug_assertions))]
    migration::Migrator::up(&db, None).await.unwrap();

    let db_data = actix_web::web::Data::new(db);
    let aws_client_data = actix_web::web::Data::new(aws_client);
    let supabase_client_data = actix_web::web::Data::new(supabase_client);
    let resend_client_data = actix_web::web::Data::new(resend_client);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(db_data.to_owned())
            .app_data(aws_client_data.to_owned())
            .app_data(supabase_client_data.to_owned())
            .app_data(resend_client_data.to_owned())
            .configure(api::attach)
    })
    .bind(("0.0.0.0", port)).unwrap()
    .run().await.unwrap();
}
