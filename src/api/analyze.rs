use std::sync::{Arc, Mutex};

use actix_web::{get, post, web::{self, Buf, Data, ServiceConfig}, HttpResponse, Responder};
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{ChatCompletionMessage, ChatCompletionRequest, Content, MessageRole}};
use sea_orm::{prelude::Uuid, sqlx::types::chrono, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use entity::prelude::*;

use crate::{error::Result, util};

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(get_analysis)
        .service(start_analyze);
}

#[derive(Debug, Clone, Deserialize)]
struct AnalyzeEntry {
    system_prompt: String,
    user_prompt: String,
    expected_ai_answer: String,
}

#[get("/{id}")]
async fn get_analysis(db: Data<DatabaseConnection>, path: web::Path<(Uuid,)>) -> Result<impl Responder> {
    let session = Session::find_by_id(path.into_inner().0)
        .one(db.get_ref()).await?;
    let Some(session) = session else {
        return Ok(HttpResponse::NotFound().finish());
    };

    Ok(HttpResponse::Ok().json(session))
}

#[derive(Debug, Serialize)]
struct OutputAnalyzeEntry {
    system_prompt: String,
    user_prompt: String,
    expected_ai_answer: String,
    base_url: String,
    model_name: String,
    actual_ai_answer: String,
    cosine_similarity: f64,
}

#[post("/")]
async fn start_analyze(db: Data<DatabaseConnection>, body: web::Payload) -> Result<impl Responder> {
    let buffer = body.to_bytes().await?;
    
    let mut entries = Vec::new();

    let mut reader = csv::Reader::from_reader(buffer.reader());
    for line in reader.deserialize() {
        let entry: AnalyzeEntry = line?;
        entries.push(entry);
    }
    
    let session = Session::insert(entity::session::ActiveModel {
        ..Default::default()
    }).exec(db.get_ref()).await?;
    
    actix_web::rt::spawn(async move {
        let pool = tokio_task_pool::Pool::bounded(std::thread::available_parallelism().unwrap().get());
        
        let models = Model::find()
            .all(db.get_ref()).await.unwrap()
            .into_iter().map(|model| Arc::new(model));
        
        let entries_len = entries.len();
        let completed_entries = Arc::new(Mutex::new(Vec::<OutputAnalyzeEntry>::new()));
        
        let mut tasks = Vec::new();
        
        for model in models.clone() {
            for entry in entries.clone() {
                let task = pool.spawn(analyze(db.clone(), session.last_insert_id, model.clone(), entry, completed_entries.clone(), entries_len)).await.unwrap();
                tasks.push(task);
            }
        }
        
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        
        let completed_entries = completed_entries.lock().unwrap();
        
        Session::update(entity::session::ActiveModel {
            id: Set(session.last_insert_id),
            finished_at: Set(Some(chrono::Utc::now().fixed_offset())),
            ..Default::default()
        }).exec(db.get_ref()).await.unwrap();
        
        println!("====== Output ======\n\n{:#?}\n\n====================", completed_entries);
    });
    
    #[derive(Debug, Serialize)]
    struct InnerResponse {
        id: Uuid,
    }

    Ok(web::Json(InnerResponse { id: session.last_insert_id }))
}

// TODO: put error into the resulting csv
async fn analyze(db: Data<DatabaseConnection>, session_id: Uuid, model: Arc<entity::model::Model>, entry: AnalyzeEntry, completed_entries: Arc<Mutex<Vec<OutputAnalyzeEntry>>>, entries_len: usize) {
    let client = OpenAIClient::builder()
        .with_endpoint(&model.base_url)
        .with_api_key(&model.api_key)
        .build().unwrap();
    
    let request = client.chat_completion(
        ChatCompletionRequest::new(
            model.model_name.clone(),
            vec![
                ChatCompletionMessage {
                    role: MessageRole::system,
                    content: Content::Text(entry.system_prompt.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatCompletionMessage {
                    role: MessageRole::user,
                    content: Content::Text(entry.user_prompt.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
        )
    ).await.unwrap();
    
    let response = request.choices.first().unwrap().message.content.as_ref().unwrap();
    
    let output = OutputAnalyzeEntry {
        cosine_similarity: util::calculate_similarity(&entry.expected_ai_answer, &response),

        system_prompt: entry.system_prompt,
        user_prompt: entry.user_prompt,
        expected_ai_answer: entry.expected_ai_answer,
        actual_ai_answer: response.clone(),

        base_url: model.base_url.clone(),
        model_name: model.model_name.clone(),
    };
    
    completed_entries.lock().unwrap().push(output);
    let len = completed_entries.lock().unwrap().len();
    
    Session::update(entity::session::ActiveModel {
        id: Set(session_id),
        progress: Set(((len as f32 / entries_len as f32) * 100.0) as i32),
        ..Default::default()
    })
        .exec(db.get_ref()).await.unwrap();
}
