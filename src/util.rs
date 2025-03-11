use std::process::Command;

use openai_api_rs::v1::{api::OpenAIClient, embedding::EmbeddingRequest};
use serde_json::json;

pub async fn calculate_similarity(text1: &str, text2: &str) -> f64 {
    // Assuming get_embedding returns Vec<f64>
    let vec1 = get_embedding(text1).await;
    let vec2 = get_embedding(text2).await;

    let magnitude = |vec: &[f64]| -> f64 {
        (vec.iter()
            .map(|x| x * x)
            .sum::<f64>())
            .sqrt()
    };

    let mag1 = magnitude(vec1.as_slice());
    let mag2 = magnitude(vec2.as_slice());

    let dot_product: f64 = vec1.iter()
        .zip(vec2.iter())
        .map(|(x, y)| x * y)
        .sum();

    if mag1 == 0.0 || mag2 == 0.0 {
        0.0
    } else {
        dot_product / (mag1 * mag2)
    }
}

pub async fn get_embedding(text: &str) -> Vec<f64> {
    const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

    let openai_client = OpenAIClient::builder()
        .with_endpoint(OPENAI_BASE_URL)
        .with_api_key(get_default_api_key(OPENAI_BASE_URL))
        .build().unwrap();

    let embedding = openai_client.embedding(
        EmbeddingRequest::new(
            "text-embedding-3-large".to_owned(),
            vec![text.to_owned()],
        )
    ).await.unwrap();

    let result = embedding.data.first().unwrap();
    result.embedding.iter().map(|&f| f as f64).collect()
}

pub fn get_default_api_key(base_url: &str) -> String {
    let key = base_url
        .replace("https://", "")
        .replace("http://", "")
        .chars()
        .filter_map(|c| 
            if c.is_ascii_alphanumeric() { Some(c) } else { None }
        )
        .collect::<String>();

    std::env::var(key).unwrap()
}

pub fn notify_slack(message: impl AsRef<str>) {
    let message = message.as_ref().to_owned();

    std::thread::spawn({
        move || {
            Command::new("curl")
                .args(["-X", "POST"])
                .args(["-H", "Content-type: application/json"])
                .args(["--data", &json!({ "text": message }).to_string()])
                .arg("https://hooks.slack.com/services/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")
                .spawn().unwrap();
        }
    });
}

