use anyhow::{bail, Context};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;

mod function;
mod llm_config;
mod message;
mod request;
mod response;

// Re-export if needed
pub use function::*;
pub use llm_config::*;
pub use message::*;
pub use request::*;
pub use response::*;

#[derive(JsonSchema, Serialize, Deserialize, Default, Debug)]
enum TemperatureUnit {
    #[serde(rename = "celsius")]
    Celsius,
    #[default]
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
struct WeatherParams {
    #[schemars(description = "City and state")]
    location: String,

    #[schemars(description = "Temperature unit")]
    #[serde(default)]
    unit: TemperatureUnit,
}

async fn get_weather(location: &str, unit: TemperatureUnit) -> serde_json::Value {
    // Mock weather data - replace with real API call in production
    serde_json::json!({
        "location": location,
        "temperature": 72,
        "unit": unit,
        "forecast": ["sunny", "windy"],
        "humidity": 65
    })
}

fn build_headers(api_key: &str) -> anyhow::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    Ok(headers)
}

const DEEP_SEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const OPENAI_URL: &str = "";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    /*
    let config = LLMConfig {
        base_url: DEEP_SEEK_URL.into(),
        api_key: env::var("DEEPSEEK_API_KEY")?,
        model: Model::DeepseekChat,
    };
    */
    let config = LLMConfig {
        base_url: DEEP_SEEK_URL.into(),
        api_key: env::var("DEEPSEEK_API_KEY")?,
        model: Model::DeepseekChat,
    };

    let headers = build_headers(&config.api_key)?;

    let weather_schema = schemars::schema_for!(WeatherParams);

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
                    "You are a helpful weather assistant. Use functions when asked about weather, THEN SUMMARIZE THEM IN HUMAN READABLE FORM"
                        .into(),
                name: None,
            },
            Message::User {
                content: "What's the weather like in San Francisco?".into(),
                name: None,
            },
        ],
        model: config.model,
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
        .post(&config.base_url)
        .headers(headers.clone())
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

    let response_data: LLLMResponse = serde_json::from_str(&response_text)?;

    if let Some(choice) = response_data.choices.first() {
        match &choice.message {
            Message::Assistant { tool_calls, .. } => {
                let mut new_messages = request.messages.clone();
                if let Some(calls) = tool_calls {
                    new_messages.push(choice.message.clone());
                    for call in calls {
                        // execute function call
                        let result = match call.function.name.as_str() {
                            "get_current_weather" => {
                                let args: WeatherParams =
                                    serde_json::from_str(&call.function.arguments)?;

                                // Execute function
                                get_weather(&args.location, args.unit).await
                            }
                            _ => bail!("Unknown function: {}", call.function.name),
                        };
                        new_messages.push(Message::Tool {
                            content: serde_json::to_string(&result)?,
                            tool_call_id: call.id.clone(),
                        });
                    }
                    let follow_up_request = ChatCompletionRequest {
                        messages: new_messages,
                        tool_choice: None,
                        model: Model::DeepseekChat,
                        temperature: 0.7, // More focused response
                        ..request
                    };

                    let final_response = client
                        .post(&config.base_url)
                        .headers(headers)
                        .json(&follow_up_request)
                        .send()
                        .await
                        .context("Failed to send follow-up request")?;

                    let final_status = final_response.status();
                    println!("Status: {}", final_status);
                    let final_text = final_response.text().await?;
                    if !final_status.is_success() {
                        println!(
                            "Failed request: {}",
                            serde_json::to_string_pretty(&follow_up_request)?
                        );
                        match serde_json::from_str::<DeepSeekErrorResponse>(&response_text) {
                            Ok(err) => bail!(
                                "API Error [{}]: {} (code: {})",
                                err.error.error_type,
                                err.error.message,
                                err.error.code
                            ),
                            Err(_) => bail!("Http {} Error: {}", final_status, final_text),
                        }
                    }

                    // Process final response
                    let final_data: LLLMResponse = serde_json::from_str(&final_text)?;

                    if let Some(final_choice) = &final_data.choices.first() {
                        match &final_choice.message {
                            Message::Assistant { content, .. } => {
                                println!("Result after function call: {}", content);
                                println!("Full result: {:?}", final_data);
                            }
                            _ => bail!("Expected an assistant message"),
                        }
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
