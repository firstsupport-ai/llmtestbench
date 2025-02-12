use actix_web::{get, patch, web::{self, Data, ServiceConfig}, HttpResponse, Responder};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

use entity::prelude::*;

use crate::error::Result;

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(get_model)
        .service(update_model);
}

#[get("/")]
async fn get_model(db: Data<DatabaseConnection>) -> Result<impl Responder> {
    let models = Model::find()
        .all(db.get_ref()).await?;

    Ok(web::Json(models))
}

#[derive(Deserialize)]
struct UpdateModelBody {
    base_url: String,
    model_name: String,
    api_key: String,
}

#[patch("/")]
async fn update_model(db: Data<DatabaseConnection>, web::Json(models): web::Json<Vec<UpdateModelBody>>) -> Result<impl Responder> {
    Model::delete_many()
        .exec(db.get_ref()).await?;

    Model::insert_many(models.into_iter().map(|model| entity::model::ActiveModel {
        base_url: Set(model.base_url),
        model_name: Set(model.model_name),
        api_key: Set(model.api_key),
        ..Default::default()
    })).on_empty_do_nothing()
    .exec(db.get_ref()).await?;
    
    Ok(HttpResponse::NoContent())
}
