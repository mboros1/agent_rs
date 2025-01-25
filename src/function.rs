use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    tool_type: ToolType,
    function: Function,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Function,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
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
pub struct FunctionChoice {
    name: String,
}
