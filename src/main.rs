use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;

mod function;
mod message;
mod request;
mod response;

// Re-export if needed
pub use function::*;
pub use message::*;
pub use request::*;
pub use response::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let api_key = env::var("DEEPSEEK_API_KEY")?;
    let url = "https://api.deepseek.com/chat/completions";

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let request = ChatCompletionRequest {
        messages: vec![
            Message::System {
                content: "You are a helpful assistant".into(),
                name: None,
            },
            Message::User {
                content: "Hi".into(),
                name: None,
            },
        ],
        model: Model::DeepseekReasoner,
        frequency_penalty: Some(0.0),
        max_tokens: Some(2048),
        presence_penalty: Some(0.0),
        response_format: Some(ResponseFormat {
            format_type: ResponseFormatType::Text,
        }),
        stop: None,
        stream: false,
        stream_options: None,
        temperature: 1.0,
        top_p: 1.0,
        tools: None,
        tool_choice: None, // Changed to None since we're not using tools
        logprobs: false,
        top_logprobs: None,
    };

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .headers(headers)
        .json(&request)
        .send()
        .await?;

    println!("Status: {}", response.status());
    let response_text = response.text().await?;

    // Try to parse successful response first
    match serde_json::from_str::<DeepSeekResponse>(&response_text) {
        Ok(parsed) => {
            println!("Successfully parsed response:");
            println!("{}", serde_json::to_string_pretty(&parsed)?);
        }
        Err(_) => {
            // Fallback to error parsing
            #[derive(Debug, Serialize, Deserialize)]
            struct ApiError {
                error: ErrorDetail,
            }

            #[derive(Debug, Serialize, Deserialize)]
            struct ErrorDetail {
                message: String,
                #[serde(rename = "type")]
                error_type: String,
                param: Option<String>,
                code: String,
            }

            match serde_json::from_str::<ApiError>(&response_text) {
                Ok(error) => {
                    eprintln!("API Error:");
                    eprintln!("{}", serde_json::to_string_pretty(&error)?);
                }
                Err(e) => {
                    eprintln!("Failed to parse response ({} bytes):", response_text.len());
                    eprintln!("Raw response: {}", response_text);
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
