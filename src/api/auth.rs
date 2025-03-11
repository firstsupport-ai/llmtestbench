use actix_web::{get, post, web::{self, Data, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use sea_orm::{Condition, DatabaseConnection, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use supabase_auth::models::{AuthClient, IdTokenCredentials, Provider};

use entity::public::prelude::*;
use tracing::info;

use crate::{error::Result, util};

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(show_auth)
        .service(process_auth);
}

#[get("/")]
async fn show_auth(req: HttpRequest) -> impl Responder {
    const TEMPLATE: &str = include_str!("../../template/auth.liquid");

    let template = liquid::ParserBuilder::with_stdlib().build().unwrap()
        .parse(TEMPLATE).unwrap();

    let obj = liquid::object!({
        "login_uri": req.full_url().as_str()
    });

    HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render(&obj).unwrap())
}

#[derive(Debug, Deserialize)]
struct AuthCallback {
    credential: String,
}

#[post("/")]
async fn process_auth(db: Data<DatabaseConnection>, payload: web::Form<AuthCallback>, supabase_client: Data<AuthClient>) -> Result<impl Responder> {
    let session = supabase_client
        .login_with_id_token(IdTokenCredentials {
            provider: Provider::Google,
            token: payload.credential.to_owned(),
            access_token: None,
            nonce: None,
            gotrue_meta_security: None
        }).await?;

    let whitelist_mode = std::env::var("WAITING_LIST_MODE").is_ok_and(|v| v.to_lowercase() == "on");
    
    let key = match ApiKey::find()
        .filter(
            Condition::all()
                .add(entity::public::api_key::Column::UserId.eq(session.user.id))
        ).one(db.get_ref()).await? {
            Some(model) => model.key,
            None => {
                let key = rand::random::<u128>().to_be_bytes();

                ApiKey::insert(entity::public::api_key::ActiveModel {
                    user_id: Set(session.user.id),
                    key: Set(key.to_vec()),
                    ..Default::default()
                }).exec(db.get_ref()).await?;
                
                util::notify_slack(
                    format!("A new user is registered {}, their key is `{:02X?}`",
                        session.user.email.clone(),
                        u128::from_be_bytes(key.try_into().unwrap())
                    ),
                );
                
                key.to_vec()
            }
        };
    
    let key = u128::from_be_bytes(key.try_into().unwrap());
    
    info!(info = "Generated key", email = session.user.email, key);
    
    if !whitelist_mode {
        Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(format!(r#"
<p>Your API Key is: <bold>{:02X?}</bold><p>
<p>Checkout this <a href="https://warped-equinox-789659.postman.co/workspace/FirstSupport~7730245e-2ff6-4e85-b6ff-401751ebd8cd/collection/29136599-3a4edf9e-f1a5-4a87-8176-c9945027e081?action=share&creator=29136599">Postman collection</a> to use the API.<p>
<p>Contact <a href="mailto:llmtest@terrydjony.com">llmtest@terrydjony.com</a> for any support.</p>
        "#, key).trim().to_owned()))
    } else {
        Ok(HttpResponse::Ok()
            .content_type("text/html")
            .body(r#"
<p>Currently we restrict access to our platform due to surge of demand.<p>
<p>Please fill in <a href="https://forms.gle/anMt3SBaLrmsjMAv7">this form</a> to get access.<p>
<p>Contact <a href="mailto:llmtest@terrydjony.com">llmtest@terrydjony.com</a> for any support.</p>
        "#.trim()))
    }
}
