use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::time::sleep;

use super::{ModelProvider, ModelRequest, ModelResponse, ProviderFuture, ToolCall};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            api_key,
            model,
            client,
        }
    }

    fn request_body(&self, request: ModelRequest, stream: bool) -> Value {
        let mut body = json!({
            "model": self.model,
            "input": request.input,
            "store": request.store,
            "tools": request.tools,
            "stream": stream,
            "parallel_tool_calls": request.parallel_tool_calls,
        });
        if let Some(instructions) = request.instructions {
            body["instructions"] = Value::String(instructions);
        }
        if !request.include.is_empty() {
            body["include"] = json!(request.include);
        }
        if let Some(format) = request.text_format {
            body["text"] = json!({ "format": format });
        }
        body
    }

    async fn post_json(&self, body: Value) -> Result<Value, String> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let send_result = self
                .client
                .post(RESPONSES_URL)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await;

            let response = match send_result {
                Ok(response) => response,
                Err(err) => {
                    if attempt < MAX_ATTEMPTS && is_transient(&err) {
                        sleep(backoff(attempt)).await;
                        continue;
                    }
                    return Err(format!("OpenAI request failed: {err}"));
                }
            };

            let status = response.status();
            if !status.is_success() {
                if attempt < MAX_ATTEMPTS && (status.as_u16() == 429 || status.is_server_error()) {
                    sleep(backoff(attempt)).await;
                    continue;
                }
                let value = response.json::<Value>().await.unwrap_or(Value::Null);
                return Err(format!(
                    "OpenAI returned {status}: {}",
                    compact_json(&value)
                ));
            }

            return response
                .json::<Value>()
                .await
                .map_err(|err| format!("OpenAI response was not JSON: {err}"));
        }
    }

    #[allow(dead_code)]
    async fn post_stream(&self, body: Value) -> Result<String, String> {
        let mut response = self
            .client
            .post(RESPONSES_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| format!("OpenAI stream request failed: {err}"))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".to_string());
            return Err(format!("OpenAI returned {status}: {error_text}"));
        }

        let mut buffer = String::new();
        let mut output = String::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|err| format!("OpenAI stream chunk failed: {err}"))?
        {
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find("\n\n") {
                let event = buffer[..index].to_string();
                buffer = buffer[index + 2..].to_string();
                apply_sse_event(&event, &mut output)?;
            }
        }
        if !buffer.trim().is_empty() {
            apply_sse_event(&buffer, &mut output)?;
        }
        Ok(output)
    }
}

impl ModelProvider for OpenAiProvider {
    fn respond<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, ModelResponse> {
        Box::pin(async move {
            let raw = self.post_json(self.request_body(request, false)).await?;
            parse_response(raw)
        })
    }

    fn stream_text<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, String> {
        Box::pin(async move { self.post_stream(self.request_body(request, true)).await })
    }
}

pub fn parse_response(raw: Value) -> Result<ModelResponse, String> {
    let id = raw
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let output_items = raw
        .get("output")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut output_text = String::new();
    let mut tool_calls = Vec::new();

    for item in &output_items {
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                for content in item
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if content.get("type").and_then(Value::as_str) == Some("output_text")
                        && let Some(text) = content.get("text").and_then(Value::as_str)
                    {
                        output_text.push_str(text);
                    }
                }
            }
            Some("function_call") => {
                tool_calls.push(ToolCall {
                    call_id: item
                        .get("call_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    arguments: item
                        .get("arguments")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(ModelResponse {
        id,
        output_text,
        tool_calls,
        output_items,
        raw,
    })
}

fn apply_sse_event(event: &str, output: &mut String) -> Result<(), String> {
    let data = event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap_or_default();
    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }
    let value: Value =
        serde_json::from_str(data).map_err(|err| format!("invalid SSE JSON: {err}"))?;
    if value.get("type").and_then(Value::as_str) == Some("response.output_text.delta")
        && let Some(delta) = value.get("delta").and_then(Value::as_str)
    {
        output.push_str(delta);
    }
    Ok(())
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unprintable json>".to_string())
}

/// Transient network failures worth retrying: connection setup and read
/// timeouts (including the half-open socket hang documented for this OS).
fn is_transient(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

/// Exponential backoff: 250ms, 500ms, 1s, ... bounded by the attempt count.
fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(250u64.saturating_mul(1u64 << attempt.min(5).saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_output_text_and_function_calls() {
        let raw = json!({
            "id": "resp_1",
            "output": [
                {
                    "type": "message",
                    "content": [
                        {"type": "output_text", "text": "hello"}
                    ]
                },
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "read",
                    "arguments": "{\"path\":\"README.md\"}"
                }
            ]
        });

        let parsed = parse_response(raw).unwrap();
        assert_eq!(parsed.output_text, "hello");
        assert_eq!(parsed.tool_calls[0].name, "read");
    }

    #[test]
    fn parses_output_text_delta_events() {
        let mut output = String::new();
        apply_sse_event(
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}",
            &mut output,
        )
        .unwrap();
        assert_eq!(output, "Hi");
    }
}
