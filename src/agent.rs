use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{
    cli,
    config::Config,
    provider::{ModelProvider, ModelRequest, function_call_output, user_message},
    tools::{self, ToolResult},
    trajectory::Trajectory,
};

const REASONING_INCLUDE: &str = "reasoning.encrypted_content";
const MAX_TOOL_OUTPUT_CHARS: usize = 20_000;

#[derive(Debug, Clone, Copy)]
pub struct AgentOptions {
    pub max_turns: usize,
}

#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub output_text: String,
    pub turns: usize,
    pub tool_calls: usize,
    pub trace_path: PathBuf,
}

pub async fn run_agent<P: ModelProvider>(
    provider: &P,
    config: &Config,
    input: String,
    options: AgentOptions,
) -> Result<AgentOutcome, String> {
    if options.max_turns == 0 {
        return Err("max turns must be greater than zero".to_string());
    }

    let trace = Trajectory::start(
        &config.home,
        json!({
            "model": config.model,
            "provider": config.provider,
            "workspace": config.workspace.display().to_string(),
            "store": false
        }),
    )
    .await?;

    let mut input_items = vec![user_message(input)];
    let mut tool_call_count = 0;

    for turn in 1..=options.max_turns {
        let response = provider
            .respond(ModelRequest {
                instructions: Some(cli::SYSTEM_PROMPT.to_string()),
                input: input_items.clone(),
                tools: tools::primitive_tool_specs(),
                store: false,
                include: vec![REASONING_INCLUDE.to_string()],
                parallel_tool_calls: false,
                text_format: None,
            })
            .await?;

        trace
            .record(
                "model_response",
                json!({
                    "turn": turn,
                    "response_id": response.id.clone(),
                    "output_text": response.output_text.clone(),
                    "tool_calls": response.tool_calls.clone(),
                    "output_item_count": response.output_items.len(),
                    "usage": response.raw.get("usage").cloned().unwrap_or(Value::Null)
                }),
            )
            .await?;

        input_items.extend(response.output_items.clone());
        if response.tool_calls.is_empty() {
            trace
                .record(
                    "session_end",
                    json!({
                        "turns": turn,
                        "tool_calls": tool_call_count,
                        "completed": true
                    }),
                )
                .await?;
            return Ok(AgentOutcome {
                output_text: response.output_text,
                turns: turn,
                tool_calls: tool_call_count,
                trace_path: trace.path().to_path_buf(),
            });
        }

        for call in response.tool_calls {
            tool_call_count += 1;
            trace
                .record(
                    "tool_call",
                    json!({
                        "turn": turn,
                        "call_id": call.call_id.clone(),
                        "name": call.name.clone(),
                        "arguments": call.arguments.clone()
                    }),
                )
                .await?;
            let result = tools::execute_tool_call(&config.workspace, &call).await;
            let serialized = serialize_tool_result(&result)?;
            input_items.push(function_call_output(&call.call_id, serialized.clone()));
            trace
                .record(
                    "tool_result",
                    json!({
                        "turn": turn,
                        "call_id": call.call_id.clone(),
                        "success": result.success,
                        "output": result.output,
                        "model_output": serialized
                    }),
                )
                .await?;
        }
    }

    trace
        .record(
            "session_end",
            json!({
                "turns": options.max_turns,
                "tool_calls": tool_call_count,
                "completed": false
            }),
        )
        .await?;
    Err(format!(
        "model did not finish after {} turns; trace written to {}",
        options.max_turns,
        trace.path().display()
    ))
}

fn serialize_tool_result(result: &ToolResult) -> Result<String, String> {
    let mut output = result.output.clone();
    if output.chars().count() > MAX_TOOL_OUTPUT_CHARS {
        output = output
            .chars()
            .take(MAX_TOOL_OUTPUT_CHARS)
            .collect::<String>();
        output.push_str("\n[greco: tool output truncated]");
    }
    serde_json::to_string(&ToolResult {
        success: result.success,
        output,
    })
    .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::*;
    use crate::provider::{ModelResponse, ProviderFuture, ToolCall};

    struct FakeProvider {
        requests: Mutex<Vec<ModelRequest>>,
    }

    impl FakeProvider {
        fn new() -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl ModelProvider for FakeProvider {
        fn respond<'a>(&'a self, request: ModelRequest) -> ProviderFuture<'a, ModelResponse> {
            Box::pin(async move {
                let mut requests = self.requests.lock().unwrap();
                let index = requests.len();
                requests.push(request);
                if index == 0 {
                    let output_items = vec![
                        json!({
                            "id": "rs_1",
                            "type": "reasoning",
                            "summary": []
                        }),
                        json!({
                            "id": "fc_1",
                            "type": "function_call",
                            "call_id": "call_1",
                            "name": "read",
                            "arguments": "{\"path\":\"README.md\"}"
                        }),
                    ];
                    Ok(ModelResponse {
                        id: "resp_1".to_string(),
                        output_text: String::new(),
                        tool_calls: vec![ToolCall {
                            call_id: "call_1".to_string(),
                            name: "read".to_string(),
                            arguments: "{\"path\":\"README.md\"}".to_string(),
                        }],
                        output_items,
                        raw: json!({"id": "resp_1"}),
                    })
                } else {
                    assert!(requests[1].input.iter().any(|item| {
                        item.get("type").and_then(Value::as_str) == Some("reasoning")
                    }));
                    assert!(requests[1].input.iter().any(|item| {
                        item.get("type").and_then(Value::as_str) == Some("function_call_output")
                            && item.get("call_id").and_then(Value::as_str) == Some("call_1")
                    }));
                    Ok(ModelResponse {
                        id: "resp_2".to_string(),
                        output_text: "done".to_string(),
                        tool_calls: Vec::new(),
                        output_items: vec![json!({
                            "type": "message",
                            "content": [{"type": "output_text", "text": "done"}]
                        })],
                        raw: json!({"id": "resp_2"}),
                    })
                }
            })
        }

        fn stream_text<'a>(&'a self, _request: ModelRequest) -> ProviderFuture<'a, String> {
            Box::pin(async move { Ok(String::new()) })
        }
    }

    #[tokio::test]
    async fn preserves_output_items_and_returns_tool_outputs() {
        let workspace = temp_dir("workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("README.md"), "Greco").unwrap();
        let config = Config {
            provider: "openai".to_string(),
            model: "gpt-5.4".to_string(),
            api_key: None,
            api_key_source: None,
            home: workspace.join(".greco"),
            workspace: workspace.clone(),
        };
        let provider = FakeProvider::new();

        let outcome = run_agent(
            &provider,
            &config,
            "read README.md".to_string(),
            AgentOptions { max_turns: 3 },
        )
        .await
        .unwrap();

        assert_eq!(outcome.output_text, "done");
        assert_eq!(outcome.turns, 2);
        assert_eq!(outcome.tool_calls, 1);
        assert!(outcome.trace_path.exists());
        fs::remove_dir_all(workspace).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir().join(format!(
            "greco-agent-test-{label}-{millis}-{}",
            std::process::id()
        ))
    }
}
