use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};

use actix_multipart::form::{tempfile::TempFile, MultipartForm, json::Json as MPJson};
use actix_web::{get, post, web::{self, Data, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use aws_sdk_s3::presigning::PresigningConfig;
use openai_api_rs::v1::{api::OpenAIClient, chat_completion::{ChatCompletionMessage, ChatCompletionRequest, Content, MessageRole}};
use resend_rs::types::{Attachment, ContentOrPath, CreateEmailBaseOptions};
use sea_orm::{prelude::Uuid, sqlx::types::chrono, DatabaseConnection, EntityTrait, QueryFilter, Set, ColumnTrait};
use serde::{Deserialize, Serialize};

use entity::public::prelude::*;
use entity::auth::prelude::Users;

use crate::{bail, error::Result, util};

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
async fn get_analysis(db: Data<DatabaseConnection>, aws_client: Data<aws_sdk_s3::Client>, path: web::Path<(Uuid,)>) -> Result<impl Responder> {
    let session = Session::find_by_id(path.into_inner().0)
        .one(db.get_ref()).await?;
    let Some(session) = session else {
        return Ok(HttpResponse::NotFound().finish());
    };

    let mut presigned_request = None;
    
    if session.finished_at.is_some() {
        presigned_request = Some(aws_client
            .get_object()
            .bucket("testllm-poc")
            .key(session.id)
            .response_content_disposition(format!("attachment;filename=Result {}.csv", session.id))
            .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap()).await.unwrap());
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "session": session,
        "url": presigned_request.map(|p| p.uri().to_owned()),
    })))
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OutputAnalyzeEntry {
    system_prompt: String,
    user_prompt: String,
    expected_ai_answer: String,
    base_url: String,
    model_name: String,
    actual_ai_answer: String,
    pub(super) cosine_similarity: f64,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) judge_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    judge_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Model {
    base_url: String,
    model_name: String,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeParameter {
    temperature: Option<f64>,
    top_p: Option<f64>,
    n: Option<i64>,
    response_format: Option<serde_json::Value>,
    stream: Option<bool>,
    stop: Option<Vec<String>>,
    max_tokens: Option<i64>,
    presence_penalty: Option<f64>,
    frequency_penalty: Option<f64>,
    logit_bias: Option<HashMap<String, i32>>,
    user: Option<String>,
    seed: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AnalyzeJudge {
    model: Model,
    prompt: String,
}

#[derive(Debug, MultipartForm)]
struct AnalyzeRequestPayload {
    #[multipart(limit = "50MB")]
    data: TempFile,
    models: MPJson<Vec<Model>>,
    parameter: Option<MPJson<AnalyzeParameter>>,
    judge: Option<MPJson<AnalyzeJudge>>,
}

#[post("")]
async fn start_analyze(db: Data<DatabaseConnection>, resend_client: Data<resend_rs::Resend>, aws_client: Data<aws_sdk_s3::Client>, payload: MultipartForm<AnalyzeRequestPayload>, req: HttpRequest) -> Result<impl Responder> {
    if payload.data.content_type.as_ref().map(|c| c.essence_str().to_string()).unwrap_or_default() != "text/csv" {
        bail!(BadRequest, "`data` must be `text/csv`");
    }
    
    let Some(auth) = req.headers().get("Authorization") else {
        bail!(Unauthorized);
    };
    
    let Some(api_key) = ApiKey::find()
        .filter(entity::public::api_key::Column::Key.eq(u128::from_str_radix(auth.to_str().unwrap(), 16).unwrap_or_default().to_be_bytes().to_vec()))
        .one(db.get_ref()).await? else {
            bail!(Unauthorized);
        };
    
    let Some(user) = Users::find()
        .filter(entity::auth::users::Column::Id.eq(api_key.user_id))
        .one(db.get_ref()).await? else {
            bail!(Unauthorized);
        };
    let Some(email) = user.email else {
        bail!(Unauthorized);
    };

    util::notify_slack(
        format!("User {} has hit `[Start Analyze]` endpoint",
            email,
        ),
    );
    
    let mut entries = Vec::new();

    let mut reader = csv::Reader::from_reader(payload.data.file.as_file());
    for line in reader.deserialize() {
        let entry: AnalyzeEntry = line?;
        entries.push(entry);
    }
    
    let session = Session::insert(entity::public::session::ActiveModel {
        ..Default::default()
    }).exec(db.get_ref()).await?;
    
    actix_web::rt::spawn(async move {
        let parallelism = std::thread::available_parallelism().unwrap().get() * 16;
        tracing::info!("Initializing analyzer with {parallelism} tasks");

        let pool = tokio_task_pool::Pool::bounded(parallelism);
        
        let MultipartForm(AnalyzeRequestPayload {
            data: _,
            models: MPJson(models),
            parameter,
            judge,
        }) = payload;
        
        let models = models.into_iter().map(|model| Arc::new(model));
        let parameter = Arc::new(parameter);
        let judge = Arc::new(judge);
        
        let entries_len = entries.len();
        let completed_entries = Arc::new(Mutex::new(Vec::<OutputAnalyzeEntry>::new()));
        
        let mut tasks = Vec::new();
        
        for model in models {
            for entry in entries.clone() {
                let task = pool.spawn(analyze(db.clone(), session.last_insert_id, model.clone(), parameter.clone(), judge.clone(), entry, completed_entries.clone(), entries_len)).await.unwrap();
                tasks.push(task);
            }
        }
        
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        
        let completed_entries = completed_entries.lock().unwrap();
        
        Session::update(entity::public::session::ActiveModel {
            id: Set(session.last_insert_id),
            finished_at: Set(Some(chrono::Utc::now().fixed_offset())),
            ..Default::default()
        }).exec(db.get_ref()).await.unwrap();
        
        let mut buffer = Vec::new();

        {
            let mut writer = csv::Writer::from_writer(&mut buffer);
            
            for entry in completed_entries.iter() {
                writer.serialize(entry).unwrap();
            }
        }
        
        aws_client
            .put_object()
            .bucket("testllm-poc")
            .key(session.last_insert_id)
            .body(buffer.clone().into())
            .send().await.unwrap();
        
        let email_payload = CreateEmailBaseOptions::new("TestLLM <testllm-noreply@firstsupport.ai>", [ email ], "LLM Analysis is completed!")
            .with_text("Hi there!\n\nYour request of LLM analysis is completed, you can download the result by the attached file!")
            .with_attachment(Attachment {
                filename: Some("Result.csv".to_owned()),
                content_type: Some("text/csv".to_owned()),
                content_or_path: ContentOrPath::Content(buffer),
            });
        
        resend_client.emails.send(email_payload).await.unwrap();
    });
    
    #[derive(Debug, Serialize)]
    struct InnerResponse {
        id: Uuid,
    }

    Ok(web::Json(InnerResponse { id: session.last_insert_id }))
}

// TODO: put error into the resulting csv
async fn analyze(db: Data<DatabaseConnection>, session_id: Uuid, model: Arc<Model>, parameter: Arc<Option<MPJson<AnalyzeParameter>>>, judge: Arc<Option<MPJson<AnalyzeJudge>>>, entry: AnalyzeEntry, completed_entries: Arc<Mutex<Vec<OutputAnalyzeEntry>>>, entries_len: usize) {
    let default_env_key = model.base_url
        .replace("https://", "")
        .replace("http://", "")
        .chars()
        .filter_map(|c|
            if c.is_ascii_alphanumeric() { Some(c) } else { None }
        ).collect::<String>();

    let client = OpenAIClient::builder()
        .with_endpoint(&model.base_url)
        .with_api_key(model.api_key.as_ref().unwrap_or(&std::env::var(default_env_key).unwrap()))
        .build().unwrap();
    
    let mut payload = ChatCompletionRequest::new(
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
    );
    
    if let Some(parameter) = parameter.as_ref() {
        payload.temperature = parameter.temperature;
        payload.top_p = parameter.top_p;
        payload.n = parameter.n;
        payload.response_format = parameter.response_format.clone();
        payload.stream = parameter.stream;
        payload.stop = parameter.stop.clone();
        payload.max_tokens = parameter.max_tokens;
        payload.presence_penalty = parameter.presence_penalty;
        payload.frequency_penalty = parameter.frequency_penalty;
        payload.logit_bias = parameter.logit_bias.clone();
        payload.user = parameter.user.clone();
        payload.seed = parameter.seed;
    }
    
    let request = client.chat_completion(payload).await.unwrap();
    let response = request.choices.first().unwrap().message.content.as_ref().unwrap();
    
    let mut judge_value = None;
    let mut judge_reason = None;

    if let Some(judge) = judge.as_ref() {
        let judge_client = OpenAIClient::builder()
            .with_endpoint(&judge.model.base_url)
            .with_api_key(judge.model.api_key.as_ref().unwrap_or(&util::get_default_api_key(&judge.model.base_url)))
            .build().unwrap();

        let judge_payload = ChatCompletionRequest::new(
            judge.model.model_name.clone(),
            vec![
                ChatCompletionMessage {
                    role: MessageRole::user,
                    content: Content::Text(judge.prompt.clone().replace("{{expected_response}}", &entry.expected_ai_answer).replace("{{actual_response}}", response)),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
        ).response_format(serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "response",
                "strict": true,
                "schema": {
                  "type": "object",
                  "properties": {
                    "value": {
                      "type": "number",
                      "description": "A score between 0 and 100 representing the similarity of the two given words. Higher values indicate greater similarity."
                    },
                    "reason": {
                      "type": "string",
                      "description": "Concise explanation for the score."
                    }
                  },
                  "required": [
                    "value",
                    "reason"
                  ],
                  "additionalProperties": false
                }
            }
        }));
        
        let judge_request = judge_client.chat_completion(judge_payload).await.unwrap();
        let judge_response = judge_request.choices.first().unwrap().message.content.as_ref().unwrap();
        let judge_response = serde_json::from_str::<serde_json::Value>(&judge_response).unwrap();
        
        judge_reason = Some(judge_response.get("reason").unwrap().as_str().unwrap().to_string());
        judge_value = Some(judge_response.get("value").unwrap().as_f64().unwrap() / 100.0);
    }
    
    let output = OutputAnalyzeEntry {
        cosine_similarity: util::calculate_similarity(&entry.expected_ai_answer, &response).await,

        system_prompt: entry.system_prompt,
        user_prompt: entry.user_prompt,
        expected_ai_answer: entry.expected_ai_answer,
        actual_ai_answer: response.clone(),

        base_url: model.base_url.clone(),
        model_name: model.model_name.clone(),
        
        judge_value,
        judge_reason,
    };
    
    completed_entries.lock().unwrap().push(output);
    let len = completed_entries.lock().unwrap().len();
    
    Session::update(entity::public::session::ActiveModel {
        id: Set(session_id),
        progress: Set(((len as f32 / entries_len as f32) * 100.0) as i32),
        ..Default::default()
    })
        .exec(db.get_ref()).await.unwrap();
}
