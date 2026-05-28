use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    eval::EvalRunReport,
    loop_control::{self, LoopAuditSnapshot},
    modification::{self, ModificationEntry},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub generated_at_unix_ms: u128,
    pub since: String,
    pub session_count: usize,
    pub eval_run_count: usize,
    pub metrics: AuditMetrics,
    pub eval_runs: Vec<AuditEvalRun>,
    pub modifications: AuditModifications,
    pub loop_state: Option<LoopAuditSnapshot>,
    pub signal_assessment: String,
    pub report_paths: Option<AuditReportPaths>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditMetrics {
    pub turns: u64,
    pub tool_calls: u64,
    pub tokens: u64,
    pub repeated_errors: u64,
    pub retracements: u64,
    pub avoidable_prompts: u64,
    pub missing_tool_failures: u64,
    pub objective_successes: u64,
    pub objective_failures: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvalRun {
    pub task_id: String,
    pub task_kind: String,
    pub success: bool,
    pub wall_ms: u128,
    pub generated_at_unix_ms: u128,
    pub criteria: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReportPaths {
    pub markdown: PathBuf,
    pub json: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditModifications {
    pub proposed: Vec<ModificationEntry>,
    pub validated: Vec<ModificationEntry>,
    pub active: Vec<ModificationEntry>,
    pub rejected: Vec<ModificationEntry>,
    pub retired: Vec<ModificationEntry>,
}

pub fn write_report(home: &Path, since: &str) -> Result<AuditReport, String> {
    let generated_at_unix_ms = now_millis();
    let mut report = build_window_report_at(home, since, generated_at_unix_ms)?;
    let directory = home.join("audit");
    fs::create_dir_all(&directory)
        .map_err(|err| format!("cannot create audit directory: {err}"))?;
    let stem = format!("{}-{}", generated_at_unix_ms, sanitize_file_name(since));
    let markdown = directory.join(format!("{stem}.md"));
    let json = directory.join(format!("{stem}.json"));
    fs::write(&markdown, render_markdown(&report))
        .map_err(|err| format!("cannot write audit markdown: {err}"))?;
    report.report_paths = Some(AuditReportPaths {
        markdown: markdown.clone(),
        json: json.clone(),
    });
    let serialized = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
    fs::write(&json, serialized).map_err(|err| format!("cannot write audit json: {err}"))?;
    Ok(report)
}

pub fn build_window_report(home: &Path, since: &str) -> Result<AuditReport, String> {
    build_window_report_at(home, since, now_millis())
}

pub fn render_markdown(report: &AuditReport) -> String {
    let mut lines = vec![
        "# Greco Audit".to_string(),
        String::new(),
        format!("- Since: `{}`", report.since),
        format!("- Sessions: {}", report.session_count),
        format!("- Eval runs: {}", report.eval_run_count),
        format!("- Signal: {}", report.signal_assessment),
        String::new(),
        "## Friction Metrics".to_string(),
        String::new(),
        format!("- Turns: {}", report.metrics.turns),
        format!("- Tool calls: {}", report.metrics.tool_calls),
        format!("- Tokens: {}", report.metrics.tokens),
        format!("- Repeated errors: {}", report.metrics.repeated_errors),
        format!("- Retracements: {}", report.metrics.retracements),
        format!("- Avoidable prompts: {}", report.metrics.avoidable_prompts),
        format!(
            "- Missing-tool failures: {}",
            report.metrics.missing_tool_failures
        ),
        format!(
            "- Objective successes: {}",
            report.metrics.objective_successes
        ),
        format!(
            "- Objective failures: {}",
            report.metrics.objective_failures
        ),
        String::new(),
        "## Modification Lifecycle".to_string(),
        String::new(),
        format!("- Proposed: {}", report.modifications.proposed.len()),
        format!("- Validated: {}", report.modifications.validated.len()),
        format!("- Active: {}", report.modifications.active.len()),
        format!("- Rejected: {}", report.modifications.rejected.len()),
        format!("- Retired: {}", report.modifications.retired.len()),
        String::new(),
        "## Autonomous Loop".to_string(),
        String::new(),
    ];

    if let Some(loop_state) = &report.loop_state {
        lines.extend([
            format!("- Frozen: {}", loop_state.frozen),
            format!(
                "- Freeze reason: {}",
                loop_state.freeze_reason.as_deref().unwrap_or("none")
            ),
            format!("- Tokens used: {}", loop_state.tokens_used),
            format!(
                "- Modifications applied: {}",
                loop_state.modifications_applied
            ),
            format!(
                "- Chained modifications: {}",
                loop_state.chained_modifications
            ),
            format!("- Decisions recorded: {}", loop_state.decision_count),
        ]);
        if let Some(decision) = &loop_state.latest_decision {
            lines.push(format!(
                "- Latest decision: {:?} {}",
                decision.kind, decision.reason
            ));
        }
        if !loop_state.recent_decisions.is_empty() {
            lines.push(String::new());
            lines.push("Recent loop decisions:".to_string());
            for decision in &loop_state.recent_decisions {
                lines.push(format!(
                    "- {:?} {} ({})",
                    decision.kind, decision.id, decision.reason
                ));
            }
        }
    } else {
        lines.push("No autonomous loop state found.".to_string());
    }

    lines.extend([String::new(), "## Eval Baseline".to_string(), String::new()]);

    if report.eval_runs.is_empty() {
        lines.push("No eval runs found in this window.".to_string());
    } else {
        for run in &report.eval_runs {
            lines.push(format!(
                "- `{}` [{}] success={} wall_ms={} criteria={}",
                run.task_id, run.task_kind, run.success, run.wall_ms, run.criteria
            ));
        }
    }

    if let Some(paths) = &report.report_paths {
        lines.push(String::new());
        lines.push("## Artifacts".to_string());
        lines.push(String::new());
        lines.push(format!("- Markdown: `{}`", paths.markdown.display()));
        lines.push(format!("- JSON: `{}`", paths.json.display()));
    }

    lines.join("\n")
}

fn build_window_report_at(
    home: &Path,
    since: &str,
    generated_at_unix_ms: u128,
) -> Result<AuditReport, String> {
    let since_ms = since_cutoff_ms(since, generated_at_unix_ms)?;
    let mut metrics = AuditMetrics::default();
    let mut session_count = 0;
    for summary in read_session_summaries(home, since_ms)? {
        session_count += 1;
        metrics.turns += summary.turns;
        metrics.tool_calls += summary.tool_calls;
        metrics.tokens += summary.tokens;
        metrics.repeated_errors += summary.repeated_errors;
        metrics.retracements += summary.retracements;
        metrics.avoidable_prompts += summary.avoidable_prompts;
        metrics.missing_tool_failures += summary.missing_tool_failures;
        if summary.objective_success {
            metrics.objective_successes += 1;
        } else {
            metrics.objective_failures += 1;
        }
    }

    let eval_runs = read_eval_runs(home, since_ms)?;
    let modification_snapshot = modification::snapshot(home)?;
    let modifications = AuditModifications {
        proposed: modification_snapshot.proposed,
        validated: modification_snapshot.validated,
        active: modification_snapshot.active,
        rejected: modification_snapshot.rejected,
        retired: modification_snapshot.retired,
    };
    let loop_state = loop_control::audit_snapshot(home)?;
    let signal_assessment = assess_signal(session_count, eval_runs.len());
    Ok(AuditReport {
        generated_at_unix_ms,
        since: since.to_string(),
        session_count,
        eval_run_count: eval_runs.len(),
        metrics,
        eval_runs,
        modifications,
        loop_state,
        signal_assessment,
        report_paths: None,
    })
}

#[derive(Debug, Clone, Default)]
struct SessionSummary {
    ts_unix_ms: u128,
    turns: u64,
    tool_calls: u64,
    tokens: u64,
    repeated_errors: u64,
    retracements: u64,
    avoidable_prompts: u64,
    missing_tool_failures: u64,
    objective_success: bool,
}

fn read_session_summaries(
    home: &Path,
    since_ms: Option<u128>,
) -> Result<Vec<SessionSummary>, String> {
    let directory = home.join("traces").join("sessions");
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|err| format!("cannot read sessions: {err}"))? {
        let entry = entry.map_err(|err| format!("cannot read session entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(summary) = read_session_summary(&path)?
            && since_ms.is_none_or(|cutoff| summary.ts_unix_ms >= cutoff)
        {
            summaries.push(summary);
        }
    }
    Ok(summaries)
}

fn read_session_summary(path: &Path) -> Result<Option<SessionSummary>, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("cannot read session trace {}: {err}", path.display()))?;
    let mut summary = SessionSummary::default();
    let mut saw_end = false;

    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let row: Value = serde_json::from_str(line)
            .map_err(|err| format!("cannot parse trace row in {}: {err}", path.display()))?;
        let event = row.get("event").and_then(Value::as_str).unwrap_or_default();
        let ts = row
            .get("ts_unix_ms")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u128;
        if ts > 0 {
            summary.ts_unix_ms = ts;
        }
        let data = row.get("data").unwrap_or(&Value::Null);
        match event {
            "friction_summary" => {
                summary.turns = data
                    .get("turns")
                    .and_then(Value::as_u64)
                    .unwrap_or(summary.turns);
                summary.tool_calls = data
                    .get("tool_calls")
                    .and_then(Value::as_u64)
                    .unwrap_or(summary.tool_calls);
                summary.tokens = data
                    .get("tokens")
                    .and_then(Value::as_u64)
                    .unwrap_or(summary.tokens);
                summary.repeated_errors = data
                    .get("repeated_errors")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                summary.retracements = data
                    .get("retracements")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                summary.avoidable_prompts = data
                    .get("avoidable_prompts")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                summary.missing_tool_failures = data
                    .get("missing_tool_failures")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                summary.objective_success = data
                    .get("objective_success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                saw_end = true;
            }
            "model_response" => {
                summary.tokens += data
                    .get("usage")
                    .and_then(|usage| usage.get("total_tokens"))
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
            }
            "session_end" => {
                summary.turns = data
                    .get("turns")
                    .and_then(Value::as_u64)
                    .unwrap_or(summary.turns);
                summary.tool_calls = data
                    .get("tool_calls")
                    .and_then(Value::as_u64)
                    .unwrap_or(summary.tool_calls);
                summary.objective_success = data
                    .get("completed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                saw_end = true;
            }
            _ => {}
        }
    }

    Ok(saw_end.then_some(summary))
}

fn read_eval_runs(home: &Path, since_ms: Option<u128>) -> Result<Vec<AuditEvalRun>, String> {
    let directory = home.join("eval").join("runs");
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|err| format!("cannot read eval runs: {err}"))? {
        let entry = entry.map_err(|err| format!("cannot read eval run entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|err| format!("cannot read eval run {}: {err}", path.display()))?;
        let report: EvalRunReport = serde_json::from_str(&content)
            .map_err(|err| format!("cannot parse eval run {}: {err}", path.display()))?;
        if since_ms.is_some_and(|cutoff| report.generated_at_unix_ms < cutoff) {
            continue;
        }
        runs.push(AuditEvalRun {
            task_id: report.task.id,
            task_kind: report.task.kind,
            success: report.success,
            wall_ms: report.wall_ms,
            generated_at_unix_ms: report.generated_at_unix_ms,
            criteria: report.criteria.len(),
        });
    }
    runs.sort_by(|left, right| {
        left.generated_at_unix_ms
            .cmp(&right.generated_at_unix_ms)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    Ok(runs)
}

fn assess_signal(session_count: usize, eval_run_count: usize) -> String {
    if session_count >= 10 && eval_run_count >= 5 {
        "baseline window has enough samples for an initial variance review".to_string()
    } else {
        format!(
            "insufficient Phase 1 signal for 5% delta detection yet; need at least 10 sessions and 5 eval runs, observed {session_count} sessions and {eval_run_count} eval runs"
        )
    }
}

fn since_cutoff_ms(since: &str, now: u128) -> Result<Option<u128>, String> {
    let trimmed = since.trim();
    if trimmed.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    if trimmed.len() < 2 {
        return Err("since must be all, or a duration like 24h, 7d, 30m".to_string());
    }
    let (number, unit) = trimmed.split_at(trimmed.len() - 1);
    let value: u128 = number
        .parse()
        .map_err(|_| "since duration must start with an integer".to_string())?;
    let millis = match unit {
        "m" => value * 60 * 1_000,
        "h" => value * 60 * 60 * 1_000,
        "d" => value * 24 * 60 * 60 * 1_000,
        _ => return Err("since unit must be m, h, d, or all".to_string()),
    };
    Ok(Some(now.saturating_sub(millis)))
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
    fn parses_since_windows() {
        assert_eq!(since_cutoff_ms("all", 1_000).unwrap(), None);
        assert_eq!(since_cutoff_ms("1m", 61_000).unwrap(), Some(1_000));
        assert_eq!(since_cutoff_ms("1h", 3_601_000).unwrap(), Some(1_000));
        assert_eq!(since_cutoff_ms("1d", 86_401_000).unwrap(), Some(1_000));
    }

    #[test]
    fn reads_friction_summary_trace() {
        let home = temp_dir("audit");
        let sessions = home.join("traces").join("sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("demo.jsonl"),
            r#"{"ts_unix_ms":1000,"event":"friction_summary","data":{"turns":2,"tool_calls":1,"tokens":42,"repeated_errors":1,"retracements":0,"avoidable_prompts":0,"missing_tool_failures":1,"objective_success":true}}"#,
        )
        .unwrap();

        let report = build_window_report_at(&home, "all", 2_000).unwrap();

        assert_eq!(report.session_count, 1);
        assert_eq!(report.metrics.turns, 2);
        assert_eq!(report.metrics.tokens, 42);
        assert_eq!(report.metrics.missing_tool_failures, 1);
        assert_eq!(report.metrics.objective_successes, 1);
        fs::remove_dir_all(home).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "greco-{label}-{}-{}",
            std::process::id(),
            now_millis()
        ))
    }
}
