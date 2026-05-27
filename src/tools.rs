use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{fs, process::Command, time};

use crate::provider::ToolCall;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

pub fn primitive_tool_specs() -> Vec<Value> {
    vec![
        function_spec(
            "read",
            "Read a UTF-8 file inside the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative file path."}
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        ),
        function_spec(
            "write",
            "Create or replace a UTF-8 file inside the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        ),
        function_spec(
            "edit",
            "Replace exact text in a UTF-8 file inside the workspace.",
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "find": {"type": "string"},
                    "replace": {"type": "string"}
                },
                "required": ["path", "find", "replace"],
                "additionalProperties": false
            }),
        ),
        function_spec(
            "bash",
            "Run a shell command in the workspace with a timeout.",
            json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"},
                    "timeout_seconds": {"type": "integer"}
                },
                "required": ["command", "timeout_seconds"],
                "additionalProperties": false
            }),
        ),
    ]
}

fn function_spec(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "name": name,
        "description": description,
        "parameters": parameters,
        "strict": true
    })
}

pub async fn read_file(workspace: &Path, path: &Path) -> Result<ToolResult, String> {
    let target = guarded_path(workspace, path)?;
    let output = fs::read_to_string(target)
        .await
        .map_err(|err| format!("read failed: {err}"))?;
    Ok(ToolResult {
        success: true,
        output,
    })
}

pub async fn write_file(
    workspace: &Path,
    path: &Path,
    content: &str,
) -> Result<ToolResult, String> {
    let target = guarded_path(workspace, path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("cannot create parent directory: {err}"))?;
    }
    fs::write(target, content)
        .await
        .map_err(|err| format!("write failed: {err}"))?;
    Ok(ToolResult {
        success: true,
        output: "written".to_string(),
    })
}

pub async fn edit_file(
    workspace: &Path,
    path: &Path,
    find: &str,
    replace: &str,
) -> Result<ToolResult, String> {
    let target = guarded_path(workspace, path)?;
    let content = fs::read_to_string(&target)
        .await
        .map_err(|err| format!("read before edit failed: {err}"))?;
    if !content.contains(find) {
        return Err("edit target text not found".to_string());
    }
    let updated = content.replacen(find, replace, 1);
    fs::write(target, updated)
        .await
        .map_err(|err| format!("write after edit failed: {err}"))?;
    Ok(ToolResult {
        success: true,
        output: "edited".to_string(),
    })
}

pub async fn run_bash(
    workspace: &Path,
    command: &str,
    timeout_seconds: u64,
) -> Result<ToolResult, String> {
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .current_dir(workspace)
        .kill_on_drop(true)
        .output();
    let output = time::timeout(Duration::from_secs(timeout_seconds), child)
        .await
        .map_err(|_| format!("command timed out after {timeout_seconds}s"))?
        .map_err(|err| format!("command failed to start: {err}"))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(ToolResult {
        success: output.status.success(),
        output: text,
    })
}

pub async fn execute_tool_call(workspace: &Path, call: &ToolCall) -> ToolResult {
    let result = match call.name.as_str() {
        "read" => match parse_args::<ReadArgs>(&call.arguments) {
            Ok(args) => read_file(workspace, &args.path).await,
            Err(err) => Err(err),
        },
        "write" => match parse_args::<WriteArgs>(&call.arguments) {
            Ok(args) => write_file(workspace, &args.path, &args.content).await,
            Err(err) => Err(err),
        },
        "edit" => match parse_args::<EditArgs>(&call.arguments) {
            Ok(args) => edit_file(workspace, &args.path, &args.find, &args.replace).await,
            Err(err) => Err(err),
        },
        "bash" => match parse_args::<BashArgs>(&call.arguments) {
            Ok(args) => run_bash(workspace, &args.command, args.timeout_seconds).await,
            Err(err) => Err(err),
        },
        _ => Err(format!("unknown tool `{}`", call.name)),
    };

    result.unwrap_or_else(|err| ToolResult {
        success: false,
        output: err,
    })
}

#[derive(Debug, Deserialize)]
struct ReadArgs {
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct WriteArgs {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Deserialize)]
struct EditArgs {
    path: PathBuf,
    find: String,
    replace: String,
}

#[derive(Debug, Deserialize)]
struct BashArgs {
    command: String,
    timeout_seconds: u64,
}

fn parse_args<T>(arguments: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(arguments).map_err(|err| format!("invalid tool arguments: {err}"))
}

fn guarded_path(workspace: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Err("absolute paths are not allowed".to_string());
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("parent path traversal is not allowed".to_string());
    }
    Ok(workspace.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ToolCall;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn rejects_parent_traversal() {
        assert!(guarded_path(Path::new("/tmp/work"), Path::new("../secret")).is_err());
    }

    #[tokio::test]
    async fn executes_tool_calls_against_workspace() {
        let workspace = temp_dir("execute");
        fs::create_dir_all(&workspace).unwrap();

        let write = execute_tool_call(
            &workspace,
            &ToolCall {
                call_id: "call_write".to_string(),
                name: "write".to_string(),
                arguments: "{\"path\":\"scratch.txt\",\"content\":\"hello\"}".to_string(),
            },
        )
        .await;
        assert!(write.success);

        let edit = execute_tool_call(
            &workspace,
            &ToolCall {
                call_id: "call_edit".to_string(),
                name: "edit".to_string(),
                arguments: "{\"path\":\"scratch.txt\",\"find\":\"hello\",\"replace\":\"goodbye\"}"
                    .to_string(),
            },
        )
        .await;
        assert!(edit.success);

        let bash = execute_tool_call(
            &workspace,
            &ToolCall {
                call_id: "call_bash".to_string(),
                name: "bash".to_string(),
                arguments:
                    "{\"command\":\"test \\\"$(cat scratch.txt)\\\" = \\\"goodbye\\\"\",\"timeout_seconds\":5}"
                        .to_string(),
            },
        )
        .await;
        assert!(bash.success);
        fs::remove_dir_all(workspace).unwrap();
    }

    #[tokio::test]
    async fn reports_edit_failure_when_text_is_absent() {
        let workspace = temp_dir("edit-miss");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("scratch.txt"), "hello").unwrap();

        let result = edit_file(&workspace, Path::new("scratch.txt"), "missing", "replace").await;
        assert_eq!(result.unwrap_err(), "edit target text not found");
        fs::remove_dir_all(workspace).unwrap();
    }

    #[tokio::test]
    async fn reports_bash_timeout() {
        let workspace = temp_dir("timeout");
        fs::create_dir_all(&workspace).unwrap();

        let result = run_bash(&workspace, "sleep 2", 1).await;
        assert_eq!(result.unwrap_err(), "command timed out after 1s");
        fs::remove_dir_all(workspace).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir().join(format!(
            "greco-tools-test-{label}-{millis}-{}",
            std::process::id()
        ))
    }
}
