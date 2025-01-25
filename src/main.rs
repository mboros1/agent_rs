use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
enum Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
    },
    Tool {
        content: String,
        tool_call_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Model {
    DeepseekChat,
    DeepseekReasoner,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: ResponseFormatType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResponseFormatType {
    Text,
    JsonObject,
}

#[derive(Debug, Serialize, Deserialize)]
struct Function {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    #[serde(rename = "type")]
    tool_type: ToolType,
    function: Function,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ToolType {
    Function,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ToolChoice {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "required")]
    Required,
    Named {
        #[serde(rename = "type")]
        tool_type: ToolType,
        function: FunctionChoice,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionChoice {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    messages: Vec<Message>,
    model: Model,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<serde_json::Value>,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_top_p")]
    top_p: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(default)]
    logprobs: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<i32>,
}

fn default_temperature() -> f32 {
    1.0
}
fn default_top_p() -> f32 {
    1.0
}

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
    let response_json: Value = serde_json::from_str(&response_text)?;
    println!(
        "Response: {}",
        serde_json::to_string_pretty(&response_json)?
    );

    Ok(())
}
