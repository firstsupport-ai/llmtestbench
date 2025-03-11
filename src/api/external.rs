use actix_web::{post, web::{Bytes, Data, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use openssl::{hash::MessageDigest, pkey::PKey, sign::Signer};
use sea_orm::{prelude::Uuid, sqlx::types::chrono::{self, Utc}, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Unchanged};
use serde::Deserialize;

use entity::public::prelude::*;

use crate::{bail, error::Result};

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(lemonsqueezy);
}

#[derive(Debug, Deserialize)]
struct LemonSqueezyMetaData {
    id: Uuid,
}

#[derive(Debug, Deserialize)]
struct LemonSqueezyMeta {
    custom_data: LemonSqueezyMetaData,
}

#[derive(Debug, Deserialize)]
struct LemonSqueezyDataAttributes {
    product_id: usize,
    status: String,
    updated_at: chrono::DateTime<Utc>,
    renews_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct LemonSqueezyData {
    r#type: String,
    attributes: LemonSqueezyDataAttributes
}

#[derive(Debug, Deserialize)]
struct LemonSqueezyBody {
    meta: LemonSqueezyMeta,
    data: LemonSqueezyData,
}

#[post("/lemonsqueezy")]
async fn lemonsqueezy(req: HttpRequest, db: Data<DatabaseConnection>, body_buffer: Bytes) -> Result<impl Responder> {
    let Some("application/json") = req.headers().get("Content-Type").map(|v| v.to_str().ok()).flatten() else {
        bail!(Unauthorized);
    };

    let Some(event_name) = req.headers().get("X-Event-Name").map(|v| v.to_str().ok()).flatten() else {
        bail!(Unauthorized);
    };
    let Some(signature) = req.headers().get("X-Signature").map(|v| v.to_str().ok()).flatten() else {
        bail!(Unauthorized);
    };
    
    let key = PKey::hmac("signature".as_bytes()).unwrap();

    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&body_buffer).unwrap();
    
    let hmac_result = signer.sign_to_vec().unwrap();
    let digest = hmac_result.iter()
        .fold(String::with_capacity(hmac_result.len() * 2), |mut acc, byte| {
            acc.push_str(&format!("{:02x}", byte));
            acc
        });
    
    if (digest.len() != signature.len()) || !openssl::memcmp::eq(digest.as_bytes(), signature.as_bytes()) {
        bail!(Unauthorized);
    }
    
    if !(event_name == "subscription_created" || event_name == "subscription_updated") {
        return Ok(HttpResponse::Ok().body("Non subscription payload"));
    }
    
    let payload: LemonSqueezyBody = serde_json::from_slice(&body_buffer).unwrap();
    
    if payload.data.r#type != "subscriptions" {
        return Ok(HttpResponse::Ok().body("Non subscription payload"));
    }

    if payload.data.attributes.status != "active" {
        return Ok(HttpResponse::Ok().body("Non active subscription"));
    }

    let Some(plan) = Plan::find()
        .filter(entity::public::plan::Column::ProductId.eq(payload.data.attributes.product_id.to_string()))
        .one(db.get_ref()).await? else {
        return Ok(HttpResponse::Ok().body("Product ID not found"));
    };
    
    ApiKey::update(entity::public::api_key::ActiveModel {
        id: Unchanged(payload.meta.custom_data.id),
        active_plan_id: Set(Some(plan.id)),
        active_plan_from: Set(Some(payload.data.attributes.updated_at.fixed_offset())),
        active_plan_to: Set(Some(payload.data.attributes.renews_at.fixed_offset())),
        ..Default::default()
    }).exec(db.get_ref()).await?;

    Ok(HttpResponse::Ok().body(""))
}
