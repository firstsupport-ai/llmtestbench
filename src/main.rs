use std::env;

use actix_web::{middleware::NormalizePath, App, HttpServer};
use aws_config::{BehaviorVersion, Region};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
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
    
    let mut http_server= HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(NormalizePath::trim())
            .app_data(db_data.to_owned())
            .app_data(aws_client_data.to_owned())
            .app_data(supabase_client_data.to_owned())
            .app_data(resend_client_data.to_owned())
            .configure(api::attach)
    }).bind(("0.0.0.0", port)).unwrap();
    
    if std::fs::exists("private.key").unwrap() && std::fs::exists("certificate.crt").unwrap() && std::fs::exists("ca_bundle.crt").unwrap() {
        let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

        ssl_builder.set_private_key_file("private.key", SslFiletype::PEM).unwrap();
        ssl_builder.set_certificate_chain_file("certificate.crt").unwrap();
        ssl_builder.set_ca_file("ca_bundle.crt").unwrap();
        
        http_server = http_server.bind_openssl("0.0.0.0:443", ssl_builder).unwrap();
    }
    
    http_server.run().await.unwrap();
}
