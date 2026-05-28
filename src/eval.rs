use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, process::Command, time};

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

pub fn list_tasks(home: &Path) -> Result<Vec<EvalTask>, String> {
    let suite = suite_dir(home);
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
    let task = load_task(home, task_id)?;
    let started = Instant::now();
    let generated_at_unix_ms = now_millis();
    let mut criteria = Vec::new();

    for criterion in &task.criteria {
        criteria.push(run_criterion(workspace, criterion).await?);
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

fn load_task(home: &Path, task_id: &str) -> Result<EvalTask, String> {
    let path = suite_dir(home).join(task_id).join("task.json");
    if !path.exists() {
        return Err(format!("eval task `{task_id}` not found under .greco/eval"));
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
    workspace: &Path,
    criterion: &EvalCriterion,
) -> Result<EvalCriterionReport, String> {
    let started = Instant::now();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&criterion.command)
        .current_dir(workspace)
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

fn suite_dir(home: &Path) -> PathBuf {
    home.join("eval")
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
        fs::create_dir_all(home.join("eval").join("b-task")).unwrap();
        fs::create_dir_all(home.join("eval").join("a-task")).unwrap();
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

        let tasks = list_tasks(&home).unwrap();

        assert_eq!(tasks[0].id, "a-task");
        assert_eq!(tasks[1].id, "b-task");
        fs::remove_dir_all(home).unwrap();
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

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "greco-{label}-{}-{}",
            std::process::id(),
            now_millis()
        ))
    }
}
