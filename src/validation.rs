use std::{path::Path, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::{process::Command, time};

use crate::{catalog, config::Config};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub skill_path: String,
    pub manifest_id: Option<String>,
    pub accepted: bool,
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
            return Ok(report(path, None, checks));
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

    let traces_dir = config.home.join("traces/validation");
    if let Err(err) = tokio::fs::create_dir_all(&traces_dir).await {
        checks.push(fail(
            "trace_prepare",
            format!("cannot create trace dir: {err}"),
        ));
    }

    let accepted = checks.iter().all(|check| check.passed);
    Ok(ValidationReport {
        skill_path: path.display().to_string(),
        manifest_id: Some(manifest.id),
        accepted,
        checks,
    })
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
        Ok(Ok(output)) => fail(
            "validation_command",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
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
