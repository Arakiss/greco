mod openai;

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use openai::OpenAiProvider;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

pub trait ModelProvider {
    fn respond<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, ModelResponse>;
    fn stream_text<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub instructions: Option<String>,
    pub input: Vec<ModelMessage>,
    pub tools: Vec<Value>,
    pub store: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

impl ModelMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelResponse {
    pub id: String,
    pub output_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub raw: Value,
}
