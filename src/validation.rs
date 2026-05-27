use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{fs, io::AsyncWriteExt, process::Command, time};

use crate::{catalog, config::Config};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub skill_path: String,
    pub manifest_id: Option<String>,
    pub accepted: bool,
    pub trace_path: Option<String>,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub async fn validate_skill(path: &Path, config: &Config) -> Result<ValidationReport, String> {
    let mut checks = Vec::new();
    let manifest = match catalog::read_manifest(path) {
        Ok(manifest) => {
            checks.push(pass("manifest_parse", "manifest.json parsed"));
            manifest
        }
        Err(err) => {
            checks.push(fail("manifest_parse", err));
            let mut report = report(path, None, checks);
            write_trace(&mut report, config).await;
            return Ok(report);
        }
    };

    match catalog::validate_manifest(&manifest, path) {
        Ok(()) => checks.push(pass(
            "manifest_static",
            "manifest fields and entrypoint are valid",
        )),
        Err(err) => checks.push(fail("manifest_static", err)),
    }

    if let Some(spec) = &manifest.validation
        && let Some(command) = &spec.command
    {
        let timeout_seconds = spec.timeout_seconds.unwrap_or(60);
        checks.push(run_validation_command(path, command, timeout_seconds).await);
    }

    let accepted = checks.iter().all(|check| check.passed);
    if let Err(err) = catalog::record_validation(&config.home, &manifest, accepted) {
        checks.push(fail("score_update", err));
    }

    let accepted = checks.iter().all(|check| check.passed);
    let mut report = ValidationReport {
        skill_path: path.display().to_string(),
        manifest_id: Some(manifest.id),
        accepted,
        trace_path: None,
        checks,
    };
    write_trace(&mut report, config).await;
    Ok(report)
}

pub fn render_report(report: &ValidationReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} {}",
        if report.accepted {
            "accepted"
        } else {
            "rejected"
        },
        report.skill_path
    ));
    for check in &report.checks {
        lines.push(format!(
            "- {}: {} ({})",
            check.name,
            if check.passed { "pass" } else { "fail" },
            check.detail
        ));
    }
    lines.join("\n")
}

async fn run_validation_command(
    path: &Path,
    command: &str,
    timeout_seconds: u64,
) -> ValidationCheck {
    let output = time::timeout(
        Duration::from_secs(timeout_seconds),
        Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .current_dir(path)
            .output(),
    )
    .await;

    match output {
        Err(_) => fail(
            "validation_command",
            format!("timed out after {timeout_seconds}s: {command}"),
        ),
        Ok(Err(err)) => fail(
            "validation_command",
            format!("could not start command: {err}"),
        ),
        Ok(Ok(output)) if output.status.success() => pass("validation_command", "command passed"),
        Ok(Ok(output)) => fail("validation_command", command_failure_detail(&output)),
    }
}

fn command_failure_detail(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("command exited with {}", output.status),
        (false, true) => format!("command exited with {}; stdout: {stdout}", output.status),
        (true, false) => format!("command exited with {}; stderr: {stderr}", output.status),
        (false, false) => format!(
            "command exited with {}; stdout: {stdout}; stderr: {stderr}",
            output.status
        ),
    }
}

fn report(
    path: &Path,
    manifest_id: Option<String>,
    checks: Vec<ValidationCheck>,
) -> ValidationReport {
    ValidationReport {
        skill_path: path.display().to_string(),
        manifest_id,
        accepted: checks.iter().all(|check| check.passed),
        trace_path: None,
        checks,
    }
}

fn pass(name: &str, detail: impl Into<String>) -> ValidationCheck {
    ValidationCheck {
        name: name.to_string(),
        passed: true,
        detail: detail.into(),
    }
}

fn fail(name: &str, detail: impl Into<String>) -> ValidationCheck {
    ValidationCheck {
        name: name.to_string(),
        passed: false,
        detail: detail.into(),
    }
}

async fn write_trace(report: &mut ValidationReport, config: &Config) {
    let traces_dir = config.home.join("traces/validation");
    if let Err(err) = fs::create_dir_all(&traces_dir).await {
        report.checks.push(fail(
            "trace_prepare",
            format!("cannot create trace dir: {err}"),
        ));
        report.accepted = false;
        return;
    }
    let trace_path = traces_dir.join(format!("{}.jsonl", now_nanos()));
    let row = json!({
        "ts_unix_ms": now_millis(),
        "event": "validation_report",
        "data": report
    });
    let line = match serde_json::to_string(&row) {
        Ok(mut line) => {
            line.push('\n');
            line
        }
        Err(err) => {
            report.checks.push(fail("trace_serialize", err.to_string()));
            report.accepted = false;
            return;
        }
    };
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&trace_path)
        .await
    {
        Ok(mut file) => {
            if let Err(err) = file.write_all(line.as_bytes()).await {
                report
                    .checks
                    .push(fail("trace_write", format!("cannot write trace: {err}")));
                report.accepted = false;
            } else {
                report.trace_path = Some(trace_path.display().to_string());
            }
        }
        Err(err) => {
            report
                .checks
                .push(fail("trace_open", format!("cannot open trace: {err}")));
            report.accepted = false;
        }
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::catalog::{self, CandidateDraft};

    #[tokio::test]
    async fn writes_validation_trace_and_score() {
        let workspace = temp_dir("validation");
        let home = workspace.join(".greco");
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        let created = catalog::create_candidate(
            &home,
            &CandidateDraft {
                id: "trace_skill".to_string(),
                version: "0.1.0".to_string(),
                description: "Trace skill".to_string(),
                entrypoint: "run.sh".to_string(),
                script: "#!/bin/sh\nset -eu\nprintf '%s\\n' ok\n".to_string(),
                validation_command: "sh run.sh".to_string(),
                timeout_seconds: 5,
                lineage: None,
                overwrite: false,
            },
        )
        .unwrap();
        let config = Config {
            provider: "openai".to_string(),
            model: "gpt-5.4".to_string(),
            api_key: None,
            api_key_source: None,
            home: home.clone(),
            workspace: workspace.clone(),
        };

        let report = validate_skill(&created.path, &config).await.unwrap();

        assert!(report.accepted);
        let trace_path = report.trace_path.unwrap();
        assert!(PathBuf::from(trace_path).exists());
        let snapshot = catalog::snapshot(&home).unwrap();
        let score = &snapshot.scores["trace_skill@0.1.0"];
        assert_eq!(score.attempts, 1);
        assert_eq!(score.passes, 1);
        tokio::fs::remove_dir_all(workspace).await.unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "greco-validation-test-{label}-{nanos}-{}",
            std::process::id()
        ))
    }
}
