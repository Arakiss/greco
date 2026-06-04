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
    let target = guarded_write_path(workspace, path)?;
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
    let target = guarded_write_path(workspace, path)?;
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

/// Upper bound on a single shell invocation, even if the model requests more.
/// The per-call `timeout_seconds` is model-controlled, so an unbounded value
/// could pin a core indefinitely; clamp it to a generous operator ceiling.
pub const MAX_BASH_TIMEOUT_SECONDS: u64 = 900;

pub async fn run_bash(
    workspace: &Path,
    command: &str,
    timeout_seconds: u64,
) -> Result<ToolResult, String> {
    let timeout_seconds = timeout_seconds.clamp(1, MAX_BASH_TIMEOUT_SECONDS);
    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .env_clear()
        .envs(sandbox_env())
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

/// Curated environment for sandboxed shell execution.
///
/// We start from a *cleared* environment and re-admit only an allowlist of
/// non-secret variables. This keeps credentials (e.g. `OPENAI_API_KEY`) out of
/// model- and suite-issued commands while still letting the real toolchain
/// (`cargo`, `rustc`, `git`, `rg`) resolve. A hardcoded minimal `PATH` silently
/// broke `cargo`/`rustc` on Homebrew and rustup layouts, so we inherit the real
/// `PATH` (which is not a secret) and the toolchain home dirs instead.
pub(crate) fn sandbox_env() -> Vec<(String, String)> {
    const ALLOW: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "LANG",
        "LANGUAGE",
        "LC_ALL",
        "LC_CTYPE",
        "TERM",
        "TMPDIR",
        "TZ",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "CARGO_TARGET_DIR",
        "RUST_BACKTRACE",
    ];
    let mut env: Vec<(String, String)> = ALLOW
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_string(), value))
        })
        .collect();
    if !env.iter().any(|(key, _)| key == "PATH") {
        env.push((
            "PATH".to_string(),
            "/usr/bin:/bin:/usr/sbin:/sbin".to_string(),
        ));
    }
    env
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
    confine_to_workspace(workspace, &workspace.join(path))
}

/// Write/edit guard: a confined workspace path that additionally refuses to
/// touch the `.greco` harness state and eval suite. Three contract docs declare
/// the suite "read-only for the agent"; the primitive layer now enforces it
/// instead of trusting convention.
fn guarded_write_path(workspace: &Path, path: &Path) -> Result<PathBuf, String> {
    let resolved = guarded_path(workspace, path)?;
    let home = workspace.join(".greco");
    let home_root = home.canonicalize().unwrap_or(home);
    if resolved.starts_with(&home_root) {
        return Err(
            "the .greco harness state and eval suite are read-only for the agent".to_string(),
        );
    }
    Ok(resolved)
}

/// Resolve symlinks in the existing prefix of `target` and confirm the result
/// stays inside `workspace`. A lexical `..`/absolute check is not enough: an
/// in-workspace symlink pointing outside would otherwise be followed. The
/// not-yet-existing tail cannot contain symlinks, so canonicalizing the longest
/// existing ancestor and re-appending the tail is sufficient.
fn confine_to_workspace(workspace: &Path, target: &Path) -> Result<PathBuf, String> {
    let workspace_root = workspace
        .canonicalize()
        .map_err(|err| format!("cannot resolve workspace: {err}"))?;
    let mut existing = target;
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let resolved_prefix = loop {
        match existing.canonicalize() {
            Ok(resolved) => break resolved,
            Err(_) => match existing.parent() {
                Some(parent) => {
                    if let Some(name) = existing.file_name() {
                        tail.push(name.to_os_string());
                    }
                    existing = parent;
                }
                None => return Err("cannot resolve path within workspace".to_string()),
            },
        }
    };
    let mut resolved = resolved_prefix;
    for name in tail.into_iter().rev() {
        resolved.push(name);
    }
    if !resolved.starts_with(&workspace_root) {
        return Err("path escapes the workspace".to_string());
    }
    Ok(resolved)
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

    #[test]
    fn denies_writes_into_greco_state_but_allows_reads() {
        let workspace = temp_dir("greco-deny");
        fs::create_dir_all(workspace.join(".greco").join("eval")).unwrap();

        // write/edit must be refused inside the read-only harness state
        assert!(guarded_write_path(&workspace, Path::new(".greco/eval/task.json")).is_err());
        assert!(guarded_write_path(&workspace, Path::new(".greco/state/loop.json")).is_err());
        // ordinary workspace writes still pass
        assert!(guarded_write_path(&workspace, Path::new("src/main.rs")).is_ok());
        // reads of the suite remain allowed
        assert!(guarded_path(&workspace, Path::new(".greco/eval/task.json")).is_ok());

        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn confines_in_workspace_symlink_escape() {
        let workspace = temp_dir("greco-symlink-ws");
        let outside = temp_dir("greco-symlink-out");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&outside).unwrap();
        // a symlink inside the workspace pointing outside must not be writable through
        std::os::unix::fs::symlink(&outside, workspace.join("escape")).unwrap();

        let result = guarded_write_path(&workspace, Path::new("escape/evil.txt"));
        assert!(
            result.is_err(),
            "symlink escape was not confined: {result:?}"
        );

        fs::remove_dir_all(&workspace).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn sandbox_env_only_returns_allowlisted_non_secret_keys() {
        let allow: std::collections::HashSet<&str> = [
            "PATH",
            "HOME",
            "USER",
            "LOGNAME",
            "SHELL",
            "LANG",
            "LANGUAGE",
            "LC_ALL",
            "LC_CTYPE",
            "TERM",
            "TMPDIR",
            "TZ",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "CARGO_TARGET_DIR",
            "RUST_BACKTRACE",
        ]
        .into_iter()
        .collect();
        for (key, _) in sandbox_env() {
            assert!(
                allow.contains(key.as_str()),
                "sandbox_env leaked a non-allowlisted variable: {key}"
            );
        }
        // a credential variable can never be emitted because it is not allowlisted
        assert!(!allow.contains("OPENAI_API_KEY"));
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
