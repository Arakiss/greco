use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{audit::AuditReport, eval};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModificationManifest {
    pub id: String,
    pub version: String,
    pub layer: ModificationLayer,
    pub state: ModificationState,
    pub description: String,
    pub friction_source: FrictionSource,
    pub payload: ModificationPayload,
    pub validation: Option<ModificationValidation>,
    pub lineage: ModificationLineage,
    pub rollback: Option<RollbackMetadata>,
    pub created_at_unix_ms: u128,
    pub applied_at_unix_ms: Option<u128>,
    pub reverted_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModificationLayer {
    A,
    S1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModificationState {
    Proposed,
    Validated,
    Active,
    Rejected,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrictionSource {
    pub since: String,
    pub session_count: usize,
    pub eval_run_count: usize,
    pub dominant_signal: String,
    pub turns: u64,
    pub tool_calls: u64,
    pub tokens: u64,
    pub repeated_errors: u64,
    pub retracements: u64,
    pub avoidable_prompts: u64,
    pub missing_tool_failures: u64,
    #[serde(default)]
    pub harness_artifacts_available: u64,
    #[serde(default)]
    pub harness_artifacts_loaded: u64,
    #[serde(default)]
    pub harness_activation_failures: u64,
    #[serde(default)]
    pub harness_adherence_checks: u64,
    #[serde(default)]
    pub harness_adherence_misses: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModificationPayload {
    CachedProcedure {
        title: String,
        body: String,
        prompt_budget_chars: usize,
    },
    SubagentPrompt {
        subagent_id: String,
        body: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModificationValidation {
    pub validated_at_unix_ms: u128,
    pub accepted: bool,
    pub eval_runs: Vec<PathBuf>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModificationLineage {
    pub parent_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackMetadata {
    pub previous_state: ModificationState,
    pub activated_prompt_chars: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModificationEntry {
    pub id: String,
    pub version: String,
    pub layer: ModificationLayer,
    pub state: ModificationState,
    pub description: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModificationSnapshot {
    pub proposed: Vec<ModificationEntry>,
    pub validated: Vec<ModificationEntry>,
    pub active: Vec<ModificationEntry>,
    pub rejected: Vec<ModificationEntry>,
    pub retired: Vec<ModificationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResult {
    pub id: String,
    pub path: PathBuf,
    pub manifest: ModificationManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleResult {
    pub id: String,
    pub from: ModificationState,
    pub to: ModificationState,
    pub path: PathBuf,
    pub manifest: ModificationManifest,
}

pub fn propose_from_audit(home: &Path, report: &AuditReport) -> Result<ProposalResult, String> {
    if report.session_count == 0 {
        return Err("cannot propose without session traces".to_string());
    }
    let source = FrictionSource {
        since: report.since.clone(),
        session_count: report.session_count,
        eval_run_count: report.eval_run_count,
        dominant_signal: dominant_signal(report),
        turns: report.metrics.turns,
        tool_calls: report.metrics.tool_calls,
        tokens: report.metrics.tokens,
        repeated_errors: report.metrics.repeated_errors,
        retracements: report.metrics.retracements,
        avoidable_prompts: report.metrics.avoidable_prompts,
        missing_tool_failures: report.metrics.missing_tool_failures,
        harness_artifacts_available: report.metrics.harness_artifacts_available,
        harness_artifacts_loaded: report.metrics.harness_artifacts_loaded,
        harness_activation_failures: report.metrics.harness_activation_failures,
        harness_adherence_checks: report.metrics.harness_adherence_checks,
        harness_adherence_misses: report.metrics.harness_adherence_misses,
    };
    let id = format!("layer-a-{}-{}", source.dominant_signal, now_millis());
    let manifest = ModificationManifest {
        id: id.clone(),
        version: "0.1.0".to_string(),
        layer: ModificationLayer::A,
        state: ModificationState::Proposed,
        description: format!(
            "Layer A cached procedure generated from `{}` friction over {} sessions",
            source.dominant_signal, source.session_count
        ),
        friction_source: source.clone(),
        payload: ModificationPayload::CachedProcedure {
            title: format!("Reduce {}", source.dominant_signal.replace('-', " ")),
            body: cached_procedure_body(&source),
            prompt_budget_chars: 1_200,
        },
        validation: None,
        lineage: ModificationLineage {
            parent_id: None,
            reason: "generated by greco propose from aggregate trace friction".to_string(),
        },
        rollback: None,
        created_at_unix_ms: now_millis(),
        applied_at_unix_ms: None,
        reverted_at_unix_ms: None,
    };
    let path = state_dir(home, &ModificationState::Proposed).join(&id);
    if path.exists() {
        return Err(format!("modification already exists: {id}"));
    }
    write_manifest_dir(&path, &manifest)?;
    Ok(ProposalResult { id, path, manifest })
}

pub fn snapshot(home: &Path) -> Result<ModificationSnapshot, String> {
    Ok(ModificationSnapshot {
        proposed: list_entries(home, ModificationState::Proposed)?,
        validated: list_entries(home, ModificationState::Validated)?,
        active: list_entries(home, ModificationState::Active)?,
        rejected: list_entries(home, ModificationState::Rejected)?,
        retired: list_entries(home, ModificationState::Retired)?,
    })
}

pub fn list_entries(
    home: &Path,
    state: ModificationState,
) -> Result<Vec<ModificationEntry>, String> {
    let dir = state_dir(home, &state);
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut modifications = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("modification read failed: {err}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let manifest = read_manifest(&entry.path())?;
        modifications.push(entry_from_manifest(&manifest, entry.path()));
    }
    modifications.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(modifications)
}

pub fn read_by_id(home: &Path, id: &str) -> Result<(PathBuf, ModificationManifest), String> {
    for state in all_states() {
        let path = state_dir(home, &state).join(id);
        if path.exists() {
            let manifest = read_manifest(&path)?;
            return Ok((path, manifest));
        }
    }
    Err(format!("modification `{id}` not found"))
}

pub async fn validate(home: &Path, workspace: &Path, id: &str) -> Result<LifecycleResult, String> {
    let (path, mut manifest) = read_by_id(home, id)?;
    if manifest.state != ModificationState::Proposed
        && manifest.state != ModificationState::Validated
    {
        return Err(format!(
            "modification `{id}` must be proposed or validated before validation, found {:?}",
            manifest.state
        ));
    }
    let from = manifest.state.clone();
    let tasks = eval::list_tasks(home)?;
    if tasks.is_empty() {
        return Err("cannot validate modification without eval tasks".to_string());
    }
    let validation_home = create_validation_sandbox(home, &manifest)?;
    let mut eval_runs = Vec::new();
    let mut accepted = true;
    for task in tasks {
        let report = eval::run_task(&validation_home, workspace, &task.id).await?;
        accepted &= report.success;
        if let Some(path) = report.run_path {
            eval_runs.push(path);
        }
    }
    manifest.state = if accepted {
        ModificationState::Validated
    } else {
        ModificationState::Rejected
    };
    manifest.validation = Some(ModificationValidation {
        validated_at_unix_ms: now_millis(),
        accepted,
        eval_runs,
        summary: if accepted {
            "all eval tasks passed with candidate available for manual application".to_string()
        } else {
            "one or more eval tasks failed; candidate rejected".to_string()
        },
    });
    let to = manifest.state.clone();
    let new_path = move_with_manifest(home, path, manifest)?;
    let manifest = read_manifest(&new_path)?;
    Ok(LifecycleResult {
        id: id.to_string(),
        from,
        to,
        path: new_path,
        manifest,
    })
}

fn create_validation_sandbox(
    home: &Path,
    manifest: &ModificationManifest,
) -> Result<PathBuf, String> {
    let sandbox = home
        .join("state")
        .join("validation-sandboxes")
        .join(format!(
            "{}-{}",
            sanitize_file_name(&manifest.id),
            now_millis()
        ));
    fs::create_dir_all(&sandbox)
        .map_err(|err| format!("cannot create validation sandbox: {err}"))?;

    let eval_source = home.join("eval");
    if eval_source.exists() {
        copy_dir(&eval_source, &sandbox.join("eval"))?;
    }

    let active_source = state_dir(home, &ModificationState::Active);
    if active_source.exists() {
        copy_dir(
            &active_source,
            &sandbox.join("modifications").join("active"),
        )?;
    }

    let mut candidate = manifest.clone();
    candidate.state = ModificationState::Active;
    candidate.applied_at_unix_ms = Some(now_millis());
    candidate.rollback = Some(RollbackMetadata {
        previous_state: manifest.state.clone(),
        activated_prompt_chars: match &manifest.payload {
            ModificationPayload::CachedProcedure { body, .. } => body.chars().count(),
            ModificationPayload::SubagentPrompt { body, .. } => body.chars().count(),
        },
        note: "validation sandbox activated candidate without touching live state".to_string(),
    });
    write_manifest_dir(
        &sandbox
            .join("modifications")
            .join("active")
            .join(&candidate.id),
        &candidate,
    )?;

    Ok(sandbox)
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| {
        format!(
            "cannot create copied directory {}: {err}",
            destination.display()
        )
    })?;
    for entry in fs::read_dir(source)
        .map_err(|err| format!("cannot read directory {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("cannot read directory entry: {err}"))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("cannot read directory entry type: {err}"))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|err| format!("cannot copy file {}: {err}", target.display()))?;
        }
    }
    Ok(())
}

pub fn apply(home: &Path, id: &str) -> Result<LifecycleResult, String> {
    let (path, mut manifest) = read_by_id(home, id)?;
    if manifest.state != ModificationState::Validated {
        return Err(format!(
            "modification `{id}` must be validated before apply, found {:?}",
            manifest.state
        ));
    }
    let activated_prompt_chars = match &manifest.payload {
        ModificationPayload::CachedProcedure { body, .. } => body.chars().count(),
        ModificationPayload::SubagentPrompt { body, .. } => body.chars().count(),
    };
    let from = manifest.state.clone();
    manifest.state = ModificationState::Active;
    manifest.applied_at_unix_ms = Some(now_millis());
    manifest.rollback = Some(RollbackMetadata {
        previous_state: from.clone(),
        activated_prompt_chars,
        note: "manual apply moved the modification into active harness state".to_string(),
    });
    let to = manifest.state.clone();
    let new_path = move_with_manifest(home, path, manifest)?;
    let manifest = read_manifest(&new_path)?;
    Ok(LifecycleResult {
        id: id.to_string(),
        from,
        to,
        path: new_path,
        manifest,
    })
}

pub fn reject(home: &Path, id: &str, reason: String) -> Result<LifecycleResult, String> {
    let (path, mut manifest) = read_by_id(home, id)?;
    if manifest.state != ModificationState::Proposed
        && manifest.state != ModificationState::Validated
    {
        return Err(format!(
            "modification `{id}` must be proposed or validated before reject, found {:?}",
            manifest.state
        ));
    }
    let from = manifest.state.clone();
    manifest.state = ModificationState::Rejected;
    manifest.validation = Some(ModificationValidation {
        validated_at_unix_ms: now_millis(),
        accepted: false,
        eval_runs: Vec::new(),
        summary: reason,
    });
    let to = manifest.state.clone();
    let new_path = move_with_manifest(home, path, manifest)?;
    let manifest = read_manifest(&new_path)?;
    Ok(LifecycleResult {
        id: id.to_string(),
        from,
        to,
        path: new_path,
        manifest,
    })
}

pub fn revert(home: &Path, id: &str) -> Result<LifecycleResult, String> {
    let (path, mut manifest) = read_by_id(home, id)?;
    if manifest.state != ModificationState::Active {
        return Err(format!(
            "modification `{id}` must be active before revert, found {:?}",
            manifest.state
        ));
    }
    let from = manifest.state.clone();
    manifest.state = ModificationState::Validated;
    manifest.reverted_at_unix_ms = Some(now_millis());
    manifest.rollback = Some(RollbackMetadata {
        previous_state: from.clone(),
        activated_prompt_chars: 0,
        note: "manual revert removed the modification from active harness state".to_string(),
    });
    let to = manifest.state.clone();
    let new_path = move_with_manifest(home, path, manifest)?;
    let manifest = read_manifest(&new_path)?;
    Ok(LifecycleResult {
        id: id.to_string(),
        from,
        to,
        path: new_path,
        manifest,
    })
}

pub fn active_layer_a_prompt(home: &Path) -> Result<String, String> {
    let active = list_entries(home, ModificationState::Active)?;
    let mut blocks = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in active {
        let (_, manifest) = read_by_id(home, &entry.id)?;
        if manifest.layer != ModificationLayer::A {
            continue;
        }
        if let ModificationPayload::CachedProcedure {
            title,
            body,
            prompt_budget_chars,
        } = manifest.payload
        {
            let mut body = body;
            if body.chars().count() > prompt_budget_chars {
                body = body.chars().take(prompt_budget_chars).collect::<String>();
                body.push_str("\n[greco: active procedure truncated]");
            }
            if seen.insert((title.clone(), body.clone())) {
                blocks.push(format!("Active procedure: {title}\n{body}"));
            }
        }
    }
    Ok(blocks.join("\n\n"))
}

pub fn active_layer_a_count(home: &Path) -> Result<usize, String> {
    let mut count = 0;
    for entry in list_entries(home, ModificationState::Active)? {
        let (_, manifest) = read_by_id(home, &entry.id)?;
        if manifest.layer == ModificationLayer::A {
            count += 1;
        }
    }
    Ok(count)
}

pub fn find_equivalent_in_states(
    home: &Path,
    manifest: &ModificationManifest,
    states: &[ModificationState],
    excluded_id: Option<&str>,
) -> Result<Option<ModificationEntry>, String> {
    for state in states {
        for entry in list_entries(home, state.clone())? {
            if excluded_id.is_some_and(|id| id == entry.id) {
                continue;
            }
            let (_, candidate) = read_by_id(home, &entry.id)?;
            if candidate.layer == manifest.layer && candidate.payload == manifest.payload {
                return Ok(Some(entry));
            }
        }
    }
    Ok(None)
}

pub fn render_entries(entries: &[ModificationEntry]) -> String {
    if entries.is_empty() {
        return "No modifications.".to_string();
    }
    entries
        .iter()
        .map(|entry| {
            format!(
                "{:?} {:?} {} {} - {}",
                entry.state, entry.layer, entry.id, entry.version, entry.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_diff(manifest: &ModificationManifest) -> String {
    match &manifest.payload {
        ModificationPayload::CachedProcedure { title, body, .. } => {
            format!(
                "--- inactive Layer A procedure\n+++ active Layer A procedure\n+title: {title}\n{}",
                plus_lines(body)
            )
        }
        ModificationPayload::SubagentPrompt { subagent_id, body } => {
            format!(
                "--- inactive S1 prompt\n+++ active S1 prompt for {subagent_id}\n{}",
                plus_lines(body)
            )
        }
    }
}

fn plus_lines(body: &str) -> String {
    body.lines()
        .map(|line| format!("+{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn cached_procedure_body(source: &FrictionSource) -> String {
    match source.dominant_signal.as_str() {
        "avoidable-prompts" => {
            "When the task has enough local context to proceed, do not ask permission or clarification. State the assumption in the final answer only if it materially affects the result.".to_string()
        }
        "repeated-errors" => {
            "Before retrying a failed tool call, inspect the exact error text and change one variable at a time. Do not repeat the same command with the same arguments after a failure.".to_string()
        }
        "retracements" => {
            "Before calling the same tool with the same arguments, check whether the prior result already answered the question. Reuse trace evidence instead of re-reading identical files.".to_string()
        }
        "missing-tool-failures" => {
            "When a requested tool is unavailable, immediately choose the closest local primitive and record the fallback in the trace-facing answer. Do not wait for a nonexistent tool.".to_string()
        }
        "high-token-use" => {
            "For read-only repository questions, read the smallest authoritative files first, summarize only the decision-relevant lines, and avoid broad scans after the answer is already grounded.".to_string()
        }
        _ => {
            "For each task, identify the smallest evidence source that can prove completion, use it first, and stop expanding context once the claim is verifiable.".to_string()
        }
    }
}

fn dominant_signal(report: &AuditReport) -> String {
    let metrics = &report.metrics;
    if metrics.avoidable_prompts > 0 {
        "avoidable-prompts".to_string()
    } else if metrics.repeated_errors > 0 {
        "repeated-errors".to_string()
    } else if metrics.retracements > 0 {
        "retracements".to_string()
    } else if metrics.missing_tool_failures > 0 {
        "missing-tool-failures".to_string()
    } else if metrics.harness_activation_failures > 0 {
        "harness-activation-failures".to_string()
    } else if metrics.harness_adherence_misses > 0 {
        "harness-adherence-misses".to_string()
    } else if report.session_count > 0 && metrics.tokens / report.session_count as u64 > 2_000 {
        "high-token-use".to_string()
    } else {
        "evidence-discipline".to_string()
    }
}

fn read_manifest(path: &Path) -> Result<ModificationManifest, String> {
    let content = fs::read_to_string(path.join("manifest.json"))
        .map_err(|err| format!("cannot read modification manifest: {err}"))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("cannot parse modification manifest: {err}"))
}

fn write_manifest_dir(path: &Path, manifest: &ModificationManifest) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| format!("cannot create modification dir: {err}"))?;
    write_manifest(path, manifest)
}

fn write_manifest(path: &Path, manifest: &ModificationManifest) -> Result<(), String> {
    let rendered = serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
    fs::write(path.join("manifest.json"), rendered)
        .map_err(|err| format!("cannot write modification manifest: {err}"))
}

fn move_with_manifest(
    home: &Path,
    old_path: PathBuf,
    manifest: ModificationManifest,
) -> Result<PathBuf, String> {
    let new_path = state_dir(home, &manifest.state).join(&manifest.id);
    if new_path != old_path {
        let parent = new_path
            .parent()
            .ok_or_else(|| "modification state path has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|err| format!("cannot create modification state dir: {err}"))?;
        if new_path.exists() {
            // Preserve the no-deletion invariant: a colliding destination is
            // retired (moved aside), never removed, so the archive stays a
            // complete memory substrate.
            let retired = state_dir(home, &ModificationState::Retired).join(format!(
                "{}-superseded-{}",
                manifest.id,
                now_millis()
            ));
            let retired_parent = retired
                .parent()
                .ok_or_else(|| "retired state path has no parent".to_string())?;
            fs::create_dir_all(retired_parent)
                .map_err(|err| format!("cannot create retired modification dir: {err}"))?;
            fs::rename(&new_path, &retired)
                .map_err(|err| format!("cannot retire superseded modification: {err}"))?;
        }
        fs::rename(&old_path, &new_path)
            .map_err(|err| format!("cannot move modification state: {err}"))?;
    }
    write_manifest(&new_path, &manifest)?;
    Ok(new_path)
}

fn entry_from_manifest(manifest: &ModificationManifest, path: PathBuf) -> ModificationEntry {
    ModificationEntry {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        layer: manifest.layer.clone(),
        state: manifest.state.clone(),
        description: manifest.description.clone(),
        path,
    }
}

fn state_dir(home: &Path, state: &ModificationState) -> PathBuf {
    home.join("modifications").join(match state {
        ModificationState::Proposed => "proposed",
        ModificationState::Validated => "validated",
        ModificationState::Active => "active",
        ModificationState::Rejected => "rejected",
        ModificationState::Retired => "retired",
    })
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

fn all_states() -> [ModificationState; 5] {
    [
        ModificationState::Proposed,
        ModificationState::Validated,
        ModificationState::Active,
        ModificationState::Rejected,
        ModificationState::Retired,
    ]
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[tokio::test]
    async fn validation_runs_against_candidate_sandbox() {
        let root = temp_dir("mod-validation-sandbox");
        let home = root.join(".greco");
        let eval_task = home.join("eval").join("candidate-active");
        fs::create_dir_all(&eval_task).unwrap();
        fs::write(
            eval_task.join("task.json"),
            r#"{"id":"candidate-active","title":"Candidate active","kind":"proof","prompt":"Proof","criteria":[{"id":"candidate-active","command":"find \"$GRECO_HOME/modifications/active\" -name manifest.json -print 2>/dev/null | grep -q '/layer-a-'","timeout_seconds":5}]}"#,
        )
        .unwrap();
        let report = AuditReport {
            generated_at_unix_ms: now_millis(),
            since: "all".to_string(),
            session_count: 1,
            eval_run_count: 1,
            metrics: crate::audit::AuditMetrics {
                tokens: 9_000,
                ..Default::default()
            },
            eval_runs: Vec::new(),
            modifications: Default::default(),
            loop_state: None,
            signal_assessment: "proof signal".to_string(),
            report_paths: None,
        };

        let proposed = propose_from_audit(&home, &report).unwrap();
        let validated = validate(&home, &root, &proposed.id).await.unwrap();
        let eval_runs = validated.manifest.validation.unwrap().eval_runs;

        assert_eq!(validated.to, ModificationState::Validated);
        assert_eq!(eval_runs.len(), 1);
        assert!(eval_runs[0].starts_with(home.join("state").join("validation-sandboxes")));
        assert!(!home.join("modifications").join("active").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn validates_applies_reverts_and_reapplies_layer_a() {
        let root = temp_dir("mod-lifecycle");
        let home = root.join(".greco");
        let eval_task = home.join("eval").join("demo");
        fs::create_dir_all(&eval_task).unwrap();
        fs::write(
            eval_task.join("task.json"),
            r#"{"id":"demo","title":"Demo","kind":"search","prompt":"Demo","criteria":[{"id":"ok","command":"true","timeout_seconds":5}]}"#,
        )
        .unwrap();
        let report = AuditReport {
            generated_at_unix_ms: now_millis(),
            since: "all".to_string(),
            session_count: 10,
            eval_run_count: 5,
            metrics: crate::audit::AuditMetrics {
                tokens: 30_000,
                ..Default::default()
            },
            eval_runs: Vec::new(),
            modifications: Default::default(),
            loop_state: None,
            signal_assessment: "baseline window has enough samples".to_string(),
            report_paths: None,
        };

        let proposed = propose_from_audit(&home, &report).unwrap();
        let validated = validate(&home, &root, &proposed.id).await.unwrap();
        let active = apply(&home, &proposed.id).unwrap();
        let prompt = active_layer_a_prompt(&home).unwrap();
        let reverted = revert(&home, &proposed.id).unwrap();
        let active_again = apply(&home, &proposed.id).unwrap();

        assert_eq!(validated.to, ModificationState::Validated);
        assert_eq!(active.to, ModificationState::Active);
        assert!(prompt.contains("Active procedure"));
        assert_eq!(reverted.to, ModificationState::Validated);
        assert_eq!(active_again.to, ModificationState::Active);
        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "greco-{label}-{}-{}",
            std::process::id(),
            now_millis()
        ))
    }
}
