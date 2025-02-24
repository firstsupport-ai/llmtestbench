use actix_web::{post, web::{self, ServiceConfig}, HttpResponse};
use sea_orm::{prelude::Uuid, DatabaseConnection, EntityTrait as _, QueryFilter, ColumnTrait as _};
use serde::{Deserialize, Serialize};

use entity::public::{prelude::*, session};

use crate::{api::analyze::OutputAnalyzeEntry, error::Result};

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(condition_test);
}

#[derive(Debug, Deserialize)]
struct ConditionTestRequest {
    analysis_id: Uuid,
    
    minimum_similarity: Option<f64>,
    minimum_judge: Option<f64>,
}

#[post("/")]
async fn condition_test(db: web::Data<DatabaseConnection>, aws_client: web::Data<aws_sdk_s3::Client>, payload: web::Form<ConditionTestRequest>) -> Result<HttpResponse> {
    let session = Session::find_by_id(payload.analysis_id)
        .filter(
            session::Column::FinishedAt.is_not_null()
        )
        .one(db.get_ref()).await?;
    let Some(session) = session else {
        return Ok(HttpResponse::NotFound().finish());
    };

    #[derive(Debug, Serialize)]
    struct InnerResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        success: bool,
    }
    
    let mut response = InnerResponse {
        message: None,
        success: true,
    };
    
    if payload.minimum_similarity.is_some() || payload.minimum_judge.is_some() {
        let file = aws_client
            .get_object()
            .bucket("testllm-poc")
            .key(session.id)
            .send().await.unwrap();
        
        let buffer = file.body.collect().await.unwrap().into_bytes().to_vec();
    
        let mut reader = csv::Reader::from_reader(buffer.as_slice());
        for (row, line) in reader.deserialize().enumerate() {
            let row = row + 2;
            let entry: OutputAnalyzeEntry = line?;
            
            if let Some(minimum_similarity) = payload.minimum_similarity {
                if entry.cosine_similarity < minimum_similarity {
                    response = InnerResponse {
                        message: Some(format!("Cosine similarity is lower than expected at row {row}")),
                        success: false,
                    };
    
                    break;
                }
            }
            
            if let (Some(judge_value), Some(minimum_judge)) = (entry.judge_value, payload.minimum_judge) {
                if judge_value < minimum_judge {
                    response = InnerResponse {
                        message: Some(format!("Judge value is lower than expected at row {row}")),
                        success: false,
                    };
    
                    break;
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(response))
}
