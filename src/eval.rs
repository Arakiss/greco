use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, process::Command, time};

use crate::{agent, config::Config, provider::ModelProvider};

const COMMITTED_SUITE_DIR: &str = "fixtures/eval-suite";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalTask {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub prompt: String,
    #[serde(default)]
    pub budget: EvalBudget,
    #[serde(default)]
    pub criteria: Vec<EvalCriterion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalBudget {
    pub max_turns: usize,
    pub max_wall_seconds: u64,
}

impl Default for EvalBudget {
    fn default() -> Self {
        Self {
            max_turns: 8,
            max_wall_seconds: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvalCriterion {
    pub id: String,
    pub command: String,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTaskSummary {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub criteria: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunReport {
    pub task: EvalTaskSummary,
    pub success: bool,
    pub generated_at_unix_ms: u128,
    pub wall_ms: u128,
    pub criteria: Vec<EvalCriterionReport>,
    pub run_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCriterionReport {
    pub id: String,
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub wall_ms: u128,
    pub output: String,
}

pub fn list_tasks(home: &Path, workspace: &Path) -> Result<Vec<EvalTask>, String> {
    let suite = suite_dir(home, workspace);
    if !suite.exists() {
        return Ok(Vec::new());
    }

    let mut tasks = Vec::new();
    for entry in fs::read_dir(&suite).map_err(|err| format!("cannot read eval suite: {err}"))? {
        let entry = entry.map_err(|err| format!("cannot read eval entry: {err}"))?;
        if !entry
            .file_type()
            .map_err(|err| format!("cannot read eval entry type: {err}"))?
            .is_dir()
        {
            continue;
        }
        let task_path = entry.path().join("task.json");
        if task_path.exists() {
            tasks.push(load_task_file(&task_path)?);
        }
    }
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(tasks)
}

pub async fn run_task(
    home: &Path,
    workspace: &Path,
    task_id: &str,
) -> Result<EvalRunReport, String> {
    let task = load_task(home, workspace, task_id)?;
    let started = Instant::now();
    let generated_at_unix_ms = now_millis();
    let mut criteria = Vec::new();

    for criterion in &task.criteria {
        criteria.push(run_criterion(home, workspace, criterion).await?);
    }

    let success = criteria.iter().all(|criterion| criterion.success);
    let mut report = EvalRunReport {
        task: task_summary(&task),
        success,
        generated_at_unix_ms,
        wall_ms: started.elapsed().as_millis(),
        criteria,
        run_path: None,
    };
    let run_path = write_run_report(home, &report)?;
    report.run_path = Some(run_path.clone());
    let rendered = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
    fs::write(&run_path, rendered)
        .map_err(|err| format!("cannot update eval run report: {err}"))?;
    Ok(report)
}

pub fn task_summary(task: &EvalTask) -> EvalTaskSummary {
    EvalTaskSummary {
        id: task.id.clone(),
        title: task.title.clone(),
        kind: task.kind.clone(),
        criteria: task.criteria.len(),
    }
}

pub fn render_task_list(tasks: &[EvalTask]) -> String {
    if tasks.is_empty() {
        return "No eval tasks found under .greco/eval.".to_string();
    }

    let mut lines = vec!["eval tasks".to_string()];
    for task in tasks {
        lines.push(format!(
            "- {} [{}] {} (criteria: {})",
            task.id,
            task.kind,
            task.title,
            task.criteria.len()
        ));
    }
    lines.join("\n")
}

pub fn render_run_report(report: &EvalRunReport) -> String {
    let mut lines = vec![
        format!("eval task: {} - {}", report.task.id, report.task.title),
        format!("success: {}", report.success),
        format!("wall_ms: {}", report.wall_ms),
    ];
    if let Some(path) = &report.run_path {
        lines.push(format!("run_report: {}", path.display()));
    }
    for criterion in &report.criteria {
        lines.push(format!(
            "- {} success={} exit={:?} timeout={} wall_ms={}",
            criterion.id,
            criterion.success,
            criterion.exit_code,
            criterion.timed_out,
            criterion.wall_ms
        ));
    }
    lines.join("\n")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveReport {
    pub task: EvalTaskSummary,
    pub success: bool,
    pub solver_completed: bool,
    pub solver_turns: usize,
    pub solver_tool_calls: usize,
    pub solver_output: String,
    pub solver_trace: PathBuf,
    pub snapshot: PathBuf,
    pub criteria: Vec<EvalCriterionReport>,
    pub wall_ms: u128,
}

/// Run the *solver* (the model) on an eval task inside an isolated copy of the
/// workspace, then grade the copy with the task criteria. This is the missing
/// causal link the deterministic `run_task` never exercised: a Layer A
/// procedure only changes behavior through the model's prompt, so a meaningful
/// objective signal requires the model to actually attempt the task. `home`
/// selects the harness state the solver loads (a candidate sandbox or the live
/// `.greco`); the snapshot is a throwaway the solver may freely edit.
pub async fn solve_task(
    home: &Path,
    workspace: &Path,
    base_config: &Config,
    provider: &dyn ModelProvider,
    task_id: &str,
) -> Result<SolveReport, String> {
    let task = load_task(home, workspace, task_id)?;
    let started = Instant::now();

    let snapshot = snapshot_workspace(home, workspace, &task.id)?;
    let solver_config = base_config.for_solver(snapshot.clone(), home.to_path_buf());
    let outcome = agent::run_agent(
        provider,
        &solver_config,
        task.prompt.clone(),
        agent::AgentOptions {
            max_turns: task.budget.max_turns.max(1),
        },
    )
    .await?;

    let mut criteria = Vec::new();
    for criterion in &task.criteria {
        criteria.push(run_criterion(home, &snapshot, criterion).await?);
    }
    let success = criteria.iter().all(|criterion| criterion.success);

    Ok(SolveReport {
        task: task_summary(&task),
        success,
        solver_completed: outcome.completed,
        solver_turns: outcome.turns,
        solver_tool_calls: outcome.tool_calls,
        solver_output: outcome.output_text,
        solver_trace: outcome.trace_path,
        snapshot,
        criteria,
        wall_ms: started.elapsed().as_millis(),
    })
}

pub fn render_solve_report(report: &SolveReport) -> String {
    let mut lines = vec![
        format!("solve task: {} - {}", report.task.id, report.task.title),
        format!(
            "solver: turns={} tools={} completed={}",
            report.solver_turns, report.solver_tool_calls, report.solver_completed
        ),
        format!("success: {}", report.success),
        format!("snapshot: {}", report.snapshot.display()),
        format!("trace: {}", report.solver_trace.display()),
        format!("wall_ms: {}", report.wall_ms),
    ];
    for criterion in &report.criteria {
        lines.push(format!(
            "- {} success={} exit={:?} timeout={} wall_ms={}",
            criterion.id,
            criterion.success,
            criterion.exit_code,
            criterion.timed_out,
            criterion.wall_ms
        ));
    }
    lines.join("\n")
}

/// Copy the workspace into a throwaway directory the solver can edit, excluding
/// the build output and harness state. `target` is excluded because it can be
/// gigabytes; `.greco` is excluded because the harness state is provided
/// separately via `GRECO_HOME`. Everything else (sources, docs, `.git`) is
/// copied so file-, git-, and cargo-based criteria all resolve.
fn snapshot_workspace(home: &Path, workspace: &Path, task_id: &str) -> Result<PathBuf, String> {
    let dest = home.join("state").join("solve-snapshots").join(format!(
        "{}-{}",
        sanitize_file_name(task_id),
        now_millis()
    ));
    fs::create_dir_all(&dest).map_err(|err| format!("cannot create solve snapshot: {err}"))?;
    copy_tree_filtered(workspace, &dest)?;
    Ok(dest)
}

fn copy_tree_filtered(source: &Path, dest: &Path) -> Result<(), String> {
    const EXCLUDE_TOP_LEVEL: &[&str] = &["target", ".greco"];
    for entry in
        fs::read_dir(source).map_err(|err| format!("cannot read {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("cannot read snapshot entry: {err}"))?;
        let name = entry.file_name();
        // Only the workspace root carries `target`/`.greco`; excluding by name
        // at every level is fine because no nested dir legitimately uses them.
        if EXCLUDE_TOP_LEVEL.contains(&name.to_string_lossy().as_ref()) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|err| format!("cannot read snapshot entry type: {err}"))?;
        let from = entry.path();
        let to = dest.join(&name);
        if file_type.is_dir() {
            fs::create_dir_all(&to)
                .map_err(|err| format!("cannot create {}: {err}", to.display()))?;
            copy_tree_filtered(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|err| format!("cannot copy {}: {err}", to.display()))?;
        }
        // Symlinks are intentionally skipped so the snapshot cannot alias outside.
    }
    Ok(())
}

fn load_task(home: &Path, workspace: &Path, task_id: &str) -> Result<EvalTask, String> {
    let path = suite_dir(home, workspace).join(task_id).join("task.json");
    if !path.exists() {
        return Err(format!(
            "eval task `{task_id}` not found under {}",
            suite_dir(home, workspace).display()
        ));
    }
    let task = load_task_file(&path)?;
    if task.id != task_id {
        return Err(format!(
            "eval task id mismatch: path is `{task_id}`, task.json says `{}`",
            task.id
        ));
    }
    Ok(task)
}

fn load_task_file(path: &Path) -> Result<EvalTask, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("cannot read eval task {}: {err}", path.display()))?;
    let task: EvalTask = serde_json::from_str(&content)
        .map_err(|err| format!("cannot parse eval task {}: {err}", path.display()))?;
    if task.id.trim().is_empty() {
        return Err(format!("eval task {} has empty id", path.display()));
    }
    if task.criteria.is_empty() {
        return Err(format!("eval task {} has no criteria", path.display()));
    }
    Ok(task)
}

async fn run_criterion(
    home: &Path,
    workspace: &Path,
    criterion: &EvalCriterion,
) -> Result<EvalCriterionReport, String> {
    let started = Instant::now();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&criterion.command)
        .current_dir(workspace)
        .env_clear()
        .envs(crate::tools::sandbox_env())
        .env("GRECO_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("cannot run criterion `{}`: {err}", criterion.id))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cannot capture criterion stdout".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "cannot capture criterion stderr".to_string())?;
    let mut stdout_buffer = Vec::new();
    let mut stderr_buffer = Vec::new();
    let stdout_task = tokio::spawn(async move {
        let _ = stdout.read_to_end(&mut stdout_buffer).await;
        stdout_buffer
    });
    let stderr_task = tokio::spawn(async move {
        let _ = stderr.read_to_end(&mut stderr_buffer).await;
        stderr_buffer
    });

    let wait = time::timeout(
        std::time::Duration::from_secs(criterion.timeout_seconds),
        child.wait(),
    )
    .await;

    let (timed_out, exit_code) = match wait {
        Ok(Ok(status)) => (false, status.code()),
        Ok(Err(err)) => {
            return Err(format!(
                "cannot wait for criterion `{}`: {err}",
                criterion.id
            ));
        }
        Err(_) => {
            let _ = child.kill().await;
            (true, None)
        }
    };

    let stdout = stdout_task.await.unwrap_or_default();
    let stderr = stderr_task.await.unwrap_or_default();
    let mut output = String::new();
    output.push_str(&String::from_utf8_lossy(&stdout));
    output.push_str(&String::from_utf8_lossy(&stderr));
    if output.chars().count() > 8_000 {
        output = output.chars().take(8_000).collect();
        output.push_str("\n[greco: criterion output truncated]");
    }

    Ok(EvalCriterionReport {
        id: criterion.id.clone(),
        command: criterion.command.clone(),
        success: !timed_out && exit_code == Some(0),
        exit_code,
        timed_out,
        wall_ms: started.elapsed().as_millis(),
        output,
    })
}

fn write_run_report(home: &Path, report: &EvalRunReport) -> Result<PathBuf, String> {
    let dir = home.join("eval").join("runs");
    fs::create_dir_all(&dir).map_err(|err| format!("cannot create eval run directory: {err}"))?;
    let path = dir.join(format!(
        "{}-{}.json",
        report.generated_at_unix_ms,
        sanitize_file_name(&report.task.id)
    ));
    let rendered = serde_json::to_string_pretty(report).map_err(|err| err.to_string())?;
    fs::write(&path, rendered).map_err(|err| format!("cannot write eval run report: {err}"))?;
    Ok(path)
}

fn suite_dir(home: &Path, workspace: &Path) -> PathBuf {
    let committed = workspace.join(COMMITTED_SUITE_DIR);
    if committed.exists() {
        committed
    } else {
        home.join("eval")
    }
}

fn default_timeout_seconds() -> u64 {
    120
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_tasks_in_sorted_order() {
        let home = temp_dir("eval-list");
        let workspace = temp_dir("eval-list-workspace");
        fs::create_dir_all(home.join("eval").join("b-task")).unwrap();
        fs::create_dir_all(home.join("eval").join("a-task")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            home.join("eval").join("b-task").join("task.json"),
            r#"{"id":"b-task","title":"B","kind":"bug","prompt":"B","criteria":[{"id":"ok","command":"true"}]}"#,
        )
        .unwrap();
        fs::write(
            home.join("eval").join("a-task").join("task.json"),
            r#"{"id":"a-task","title":"A","kind":"refactor","prompt":"A","criteria":[{"id":"ok","command":"true"}]}"#,
        )
        .unwrap();

        let tasks = list_tasks(&home, &workspace).unwrap();

        assert_eq!(tasks[0].id, "a-task");
        assert_eq!(tasks[1].id, "b-task");
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn committed_fixture_suite_takes_precedence() {
        let home = temp_dir("eval-list-home-precedence");
        let workspace = temp_dir("eval-list-workspace-precedence");
        fs::create_dir_all(home.join("eval").join("local-task")).unwrap();
        fs::create_dir_all(workspace.join(COMMITTED_SUITE_DIR).join("committed-task")).unwrap();
        fs::write(
            home.join("eval").join("local-task").join("task.json"),
            r#"{"id":"local-task","title":"Local","kind":"bug","prompt":"Local","criteria":[{"id":"ok","command":"true"}]}"#,
        )
        .unwrap();
        fs::write(
            workspace
                .join(COMMITTED_SUITE_DIR)
                .join("committed-task")
                .join("task.json"),
            r#"{"id":"committed-task","title":"Committed","kind":"bug","prompt":"Committed","criteria":[{"id":"ok","command":"true"}]}"#,
        )
        .unwrap();

        let tasks = list_tasks(&home, &workspace).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "committed-task");
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[tokio::test]
    async fn runs_criteria_and_writes_report() {
        let home = temp_dir("eval-run-home");
        let workspace = temp_dir("eval-run-workspace");
        fs::create_dir_all(home.join("eval").join("demo")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(
            home.join("eval").join("demo").join("task.json"),
            r#"{"id":"demo","title":"Demo","kind":"search","prompt":"Demo","criteria":[{"id":"ok","command":"test -d .","timeout_seconds":5}]}"#,
        )
        .unwrap();

        let report = run_task(&home, &workspace, "demo").await.unwrap();

        assert!(report.success);
        assert!(report.run_path.unwrap().exists());
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[tokio::test]
    async fn criterion_runs_with_cleared_environment() {
        let home = temp_dir("eval-envclear-home");
        let workspace = temp_dir("eval-envclear-ws");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&workspace).unwrap();

        // env_clear contract: non-allowlisted parent variables (cargo sets
        // CARGO_PKG_NAME for the test process) must not reach the criterion,
        // while GRECO_HOME must. The probe can never fail spuriously: if the
        // variable is absent it reads CLEARED too; it only fails on a real leak.
        let criterion = EvalCriterion {
            id: "env".to_string(),
            command: "test \"${CARGO_PKG_NAME:-CLEARED}\" = CLEARED && test -n \"$GRECO_HOME\""
                .to_string(),
            timeout_seconds: 5,
        };
        let report = run_criterion(&home, &workspace, &criterion).await.unwrap();
        assert!(
            report.success,
            "criterion env not scrubbed or GRECO_HOME missing: {}",
            report.output
        );

        fs::remove_dir_all(&home).unwrap();
        fs::remove_dir_all(&workspace).unwrap();
    }

    struct SolveFakeProvider {
        calls: std::sync::Mutex<usize>,
    }

    impl crate::provider::ModelProvider for SolveFakeProvider {
        fn respond<'a>(
            &'a self,
            _request: crate::provider::ModelRequest,
        ) -> crate::provider::ProviderFuture<'a, crate::provider::ModelResponse> {
            Box::pin(async move {
                let mut calls = self.calls.lock().unwrap();
                let index = *calls;
                *calls += 1;
                if index == 0 {
                    // turn 1: write a file the criterion will grade
                    Ok(crate::provider::ModelResponse {
                        id: "r1".to_string(),
                        output_text: String::new(),
                        tool_calls: vec![crate::provider::ToolCall {
                            call_id: "c1".to_string(),
                            name: "write".to_string(),
                            arguments: "{\"path\":\"result.txt\",\"content\":\"solved\"}"
                                .to_string(),
                        }],
                        output_items: vec![serde_json::json!({
                            "id": "fc1",
                            "type": "function_call",
                            "call_id": "c1",
                            "name": "write",
                            "arguments": "{\"path\":\"result.txt\",\"content\":\"solved\"}"
                        })],
                        raw: serde_json::json!({"id": "r1"}),
                    })
                } else {
                    // turn 2: finish
                    Ok(crate::provider::ModelResponse {
                        id: "r2".to_string(),
                        output_text: "done".to_string(),
                        tool_calls: Vec::new(),
                        output_items: vec![serde_json::json!({
                            "type": "message",
                            "content": [{"type": "output_text", "text": "done"}]
                        })],
                        raw: serde_json::json!({"id": "r2"}),
                    })
                }
            })
        }

        fn stream_text<'a>(
            &'a self,
            _request: crate::provider::ModelRequest,
        ) -> crate::provider::ProviderFuture<'a, String> {
            Box::pin(async move { Ok(String::new()) })
        }
    }

    #[tokio::test]
    async fn solve_task_runs_solver_in_isolated_snapshot_and_grades_it() {
        let home = temp_dir("solve-home");
        let workspace = temp_dir("solve-ws");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("seed.txt"), "seed").unwrap();
        fs::create_dir_all(home.join("eval").join("solve-demo")).unwrap();
        fs::write(
            home.join("eval").join("solve-demo").join("task.json"),
            r#"{"id":"solve-demo","title":"Solve demo","kind":"bug_fix","prompt":"create result.txt","budget":{"max_turns":4,"max_wall_seconds":30},"criteria":[{"id":"made","command":"test -f result.txt","timeout_seconds":5}]}"#,
        )
        .unwrap();
        let base = Config {
            provider: "openai".to_string(),
            model: "test".to_string(),
            api_key: None,
            api_key_source: None,
            home: home.clone(),
            workspace: workspace.clone(),
        };
        let provider = SolveFakeProvider {
            calls: std::sync::Mutex::new(0),
        };

        let report = solve_task(&home, &workspace, &base, &provider, "solve-demo")
            .await
            .unwrap();

        assert!(report.success, "criteria failed: {:?}", report.criteria);
        assert_eq!(report.solver_tool_calls, 1);
        // the solver's edit lands in the snapshot...
        assert!(
            report.snapshot.join("result.txt").exists(),
            "solver edit missing from snapshot"
        );
        assert!(
            report.snapshot.join("seed.txt").exists(),
            "snapshot did not copy the workspace"
        );
        // ...and the real workspace is left untouched (isolation invariant).
        assert!(
            !workspace.join("result.txt").exists(),
            "solver leaked into the real workspace"
        );

        fs::remove_dir_all(&home).ok();
        fs::remove_dir_all(&workspace).ok();
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "greco-{label}-{}-{}",
            std::process::id(),
            now_millis()
        ))
    }
}
