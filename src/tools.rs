use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{fs, process::Command, time};

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
                    "timeout_seconds": {"type": "number"}
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

    #[test]
    fn rejects_parent_traversal() {
        assert!(guarded_path(Path::new("/tmp/work"), Path::new("../secret")).is_err());
    }
}
