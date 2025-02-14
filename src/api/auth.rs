use actix_web::{get, post, web::{self, Data, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use sea_orm::{Condition, DatabaseConnection, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use supabase_auth::models::{AuthClient, IdTokenCredentials, Provider};

use entity::public::prelude::*;

use crate::error::Result;

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
                
                key.to_vec()
            }
        };

    Ok(HttpResponse::Ok()
        .body(format!(r#"
Your Api Key:
{:02X?}
        "#, u128::from_be_bytes(key.try_into().unwrap()))))
}
