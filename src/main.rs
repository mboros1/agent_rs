use anyhow::bail;
use core::fmt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(JsonSchema, Serialize, Deserialize, Default)]
enum TemperatureUnit {
    #[serde(rename = "celsius")]
    Celsius,
    #[default]
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
}

#[derive(JsonSchema, Serialize)]
struct WeatherParams {
    #[schemars(description = "City and state")]
    location: String,

    #[schemars(description = "Temperature unit")]
    #[serde(default)]
    unit: TemperatureUnit,
}

async fn get_weather(location: &str, unit: &str) -> serde_json::Value {
    // Mock weather data - replace with real API call in production
    serde_json::json!({
        "location": location,
        "temperature": 72,
        "unit": unit,
        "forecast": ["sunny", "windy"],
        "humidity": 65
    })
}

fn weather_function_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "location": {
                "type": "string",
                "description": "City and state"
            },
            "unit": {
                "type": "string",
                "enum": ["celsius", "fahrenheit"],
                "default": "fahrenheit"
            }
        },
        "required": ["location"]
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let api_key = env::var("DEEPSEEK_API_KEY")?;
    let url = "https://api.deepseek.com/chat/completions";

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let weather_schema = weather_function_schema(); //schemars::schema_for!(WeatherParams);

    // Create the function definition
    let tools = vec![Tool {
        tool_type: ToolType::Function,
        function: Function {
            name: "get_current_weather".into(),
            description: "Get the current weather in a given location".into(),
            parameters: serde_json::to_value(weather_schema).unwrap(),
        },
    }];

    let request = ChatCompletionRequest {
        messages: vec![
            Message::System {
                content:
                    "You are a helpful weather assistant. Use functions when asked about weather"
                        .into(),
                name: None,
            },
            Message::User {
                content: "What's the weather like in San Francisco?".into(),
                name: None,
            },
        ],
        model: Model::DeepseekChat,
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
        tools: Some(tools),
        tool_choice: Some(ToolChoice::Auto),
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

    let status = response.status();
    println!("Status: {}", status);
    let response_text = response.text().await?;
    if !status.is_success() {
        match serde_json::from_str::<DeepSeekErrorResponse>(&response_text) {
            Ok(err) => bail!(
                "API Error [{}]: {} (code: {})",
                err.error.error_type,
                err.error.message,
                err.error.code
            ),
            Err(_) => bail!("HTTP {} Error: {}", status, response_text),
        }
    }

    let response_data: DeepSeekResponse = serde_json::from_str(&response_text)?;

    if let Some(choice) = response_data.choices.first() {
        match &choice.message {
            Message::Assistant { tool_calls, .. } => {
                if let Some(calls) = tool_calls {
                    for call in calls {
                        println!("Function call: {}", call.function.name);
                    }
                }
            }

            _ => eprintln!("Unexpected message type"),
        }
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeepSeekErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<serde_json::Value>, // Can be null or various types
    pub code: String,
}
