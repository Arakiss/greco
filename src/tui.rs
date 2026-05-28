use std::fs;

use crate::{catalog::Catalog, config::Config, eval, loop_control, modification};

pub fn render_status(config: &Config) -> String {
    [
        format!("Greco {}", env!("CARGO_PKG_VERSION")),
        format!("provider: {}", config.provider),
        format!("model: {}", config.model),
        format!("workspace: {}", config.workspace.display()),
        format!("home: {}", config.home.display()),
        format!(
            "api key: {}",
            if config.api_key.is_some() {
                "present"
            } else {
                "missing"
            }
        ),
    ]
    .join("\n")
}

pub fn render_snapshot(config: &Config) -> Result<String, String> {
    let catalog = Catalog::load(&config.home)?;
    let mut lines = vec![
        "greco tui snapshot".to_string(),
        "==================".to_string(),
        format!("version: {}", env!("CARGO_PKG_VERSION")),
        format!("provider: {} / {}", config.provider, config.model),
        format!("workspace: {}", config.workspace.display()),
        format!("archive: {}", config.home.display()),
        format!(
            "credential: {}",
            config.api_key_source.as_deref().unwrap_or("missing")
        ),
        String::new(),
        "catalog".to_string(),
        "-------".to_string(),
    ];

    if catalog.active.is_empty() {
        lines.push("active skills: 0".to_string());
    } else {
        lines.push(format!("active skills: {}", catalog.active.len()));
        for skill in catalog.active {
            lines.push(format!(
                "- {} {} :: {}",
                skill.id, skill.version, skill.description
            ));
        }
    }

    lines.extend([String::new(), "phase 2".to_string(), "-------".to_string()]);
    let tasks = eval::list_tasks(&config.home)?;
    let modifications = modification::snapshot(&config.home)?;
    lines.push(format!("eval tasks: {}", tasks.len()));
    lines.push(format!(
        "proposed modifications: {}",
        modifications.proposed.len()
    ));
    lines.push(format!(
        "validated modifications: {}",
        modifications.validated.len()
    ));
    lines.push(format!(
        "active modifications: {}",
        modifications.active.len()
    ));
    lines.push(format!(
        "rejected modifications: {}",
        modifications.rejected.len()
    ));
    lines.push(format!("latest audit: {}", latest_audit(&config.home)));
    if modifications.active.is_empty() {
        lines.push("runtime Layer A procedures: 0".to_string());
    } else {
        lines.push("runtime Layer A procedures: active".to_string());
        for entry in modifications.active {
            lines.push(format!("- {} :: {}", entry.id, entry.description));
        }
    }
    let loop_status = loop_control::status(&config.home)?;
    lines.extend([
        String::new(),
        "phase 3 loop".to_string(),
        "------------".to_string(),
        format!("frozen: {}", loop_status.state.frozen),
        format!(
            "freeze reason: {}",
            loop_status.state.freeze_reason.as_deref().unwrap_or("none")
        ),
        format!(
            "budget modifications: {}/{}",
            loop_status.state.modifications_applied,
            loop_status.policy.budgets.max_modifications_per_window
        ),
        format!(
            "chained modifications: {}/{}",
            loop_status.state.chained_modifications,
            loop_status.policy.budgets.max_chained_modifications
        ),
        format!("loop decisions: {}", loop_status.state.decisions.len()),
    ]);
    if let Some(decision) = loop_status.state.decisions.last() {
        lines.push(format!(
            "latest loop decision: {:?} {}",
            decision.kind, decision.reason
        ));
        if let Some(comparison) = &decision.comparison {
            lines.push(format!(
                "latest comparison: {:?} primary_improvement_ppm={} max_regression_ppm={}",
                comparison.outcome,
                comparison.primary_improvement_ppm,
                comparison.max_regression_ppm
            ));
            if let Some(path) = &comparison.artifact_path {
                lines.push(format!("comparison artifact: {}", path.display()));
            }
        }
    } else {
        lines.push("latest loop decision: none".to_string());
    }
    lines.extend([
        String::new(),
        "next".to_string(),
        "----".to_string(),
        "1. greco loop run --since all --dry-run --json".to_string(),
        "2. greco loop run --since all --apply --json".to_string(),
        "3. greco loop status --json".to_string(),
        "4. greco audit --since all".to_string(),
    ]);

    Ok(lines.join("\n"))
}

fn latest_audit(home: &std::path::Path) -> String {
    let dir = home.join("audit");
    let Ok(entries) = fs::read_dir(dir) else {
        return "missing".to_string();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .max()
        .unwrap_or_else(|| "missing".to_string())
}
