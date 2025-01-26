use crate::Model;

pub struct LLMConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: Model,
}
