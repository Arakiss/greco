mod openai;

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use openai::OpenAiProvider;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

pub trait ModelProvider {
    fn respond<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, ModelResponse>;
    #[allow(dead_code)]
    fn stream_text<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub instructions: Option<String>,
    pub input: Vec<Value>,
    pub tools: Vec<Value>,
    pub store: bool,
    pub include: Vec<String>,
    pub parallel_tool_calls: bool,
}

pub fn user_message(content: impl Into<String>) -> Value {
    json!({
        "type": "message",
        "role": "user",
        "content": [
            {
                "type": "input_text",
                "text": content.into()
            }
        ]
    })
}

pub fn function_call_output(call_id: &str, output: impl Into<String>) -> Value {
    json!({
        "type": "function_call_output",
        "call_id": call_id,
        "output": output.into()
    })
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
    pub output_items: Vec<Value>,
    pub raw: Value,
}
