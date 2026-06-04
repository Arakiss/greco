use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    audit::{self, AuditEvalRun, AuditReport},
    config::Config,
    eval::EvalRunReport,
    modification::{self, LifecycleResult, ModificationLayer, ModificationState, ProposalResult},
    provider::ModelProvider,
};

/// Wiring that lets the autonomous loop run the solver during validation. When
/// present, autonomous apply additionally requires a positive *measured*
/// marginal improvement (see `run_with_solver`). It is borrowed, not owned, so
/// it stays out of the serializable option/report types.
pub struct LoopSolver<'a> {
    pub provider: &'a dyn ModelProvider,
    pub config: &'a Config,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopBudgets {
    pub max_tokens_per_window: u64,
    pub max_modifications_per_window: u64,
    pub max_chained_modifications: u64,
    pub max_tokens_per_validation: u64,
    pub max_wall_seconds_per_validation: u64,
    pub early_stop_on_first_regression: bool,
}

impl Default for LoopBudgets {
    fn default() -> Self {
        Self {
            max_tokens_per_window: 100_000,
            max_modifications_per_window: 2,
            max_chained_modifications: 2,
            max_tokens_per_validation: 50_000,
            max_wall_seconds_per_validation: 300,
            early_stop_on_first_regression: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopThresholds {
    pub min_relative_improvement: f64,
    pub regression_tolerance: f64,
    pub validation_runs_required: usize,
    pub pareto_keep_when_uncomparable: bool,
}

impl Default for LoopThresholds {
    fn default() -> Self {
        Self {
            min_relative_improvement: 0.05,
            regression_tolerance: 0.01,
            validation_runs_required: 2,
            pareto_keep_when_uncomparable: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopPolicy {
    pub budgets: LoopBudgets,
    pub thresholds: LoopThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopState {
    pub frozen: bool,
    pub freeze_reason: Option<String>,
    pub window_started_at_unix_ms: u128,
    pub tokens_used: u64,
    pub validation_wall_ms_used: u128,
    pub modifications_applied: u64,
    pub chained_modifications: u64,
    pub checkpoints: Vec<LoopCheckpoint>,
    pub decisions: Vec<LoopDecision>,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            frozen: false,
            freeze_reason: None,
            window_started_at_unix_ms: now_millis(),
            tokens_used: 0,
            validation_wall_ms_used: 0,
            modifications_applied: 0,
            chained_modifications: 0,
            checkpoints: Vec::new(),
            decisions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopCheckpoint {
    pub id: String,
    pub created_at_unix_ms: u128,
    pub modification_id: String,
    pub active_before: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopDecision {
    pub id: String,
    pub kind: LoopDecisionKind,
    pub at_unix_ms: u128,
    pub since: String,
    pub modification_id: Option<String>,
    pub reason: String,
    pub budget: LoopBudgetSnapshot,
    pub comparison: Option<LoopComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopDecisionKind {
    WouldApply,
    Applied,
    Rejected,
    KeptPareto,
    SkippedDuplicate,
    RefusedFrozen,
    FrozenBudget,
    RolledBackRegression,
    OperatorFrozen,
    OperatorUnfrozen,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopBudgetSnapshot {
    pub tokens_used: u64,
    pub max_tokens_per_window: u64,
    pub validation_wall_ms_used: u128,
    pub max_wall_seconds_per_validation: u64,
    pub modifications_applied: u64,
    pub max_modifications_per_window: u64,
    pub chained_modifications: u64,
    pub max_chained_modifications: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopRunOptions {
    pub since: String,
    pub mode: LoopMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRunReport {
    pub success: bool,
    pub mode: LoopMode,
    pub decision: LoopDecision,
    pub proposed_id: Option<String>,
    pub validation_runs: Vec<LoopValidationSummary>,
    pub applied: Option<LifecycleResult>,
    pub rollback: Option<LifecycleResult>,
    pub policy: LoopPolicy,
    pub state: LoopState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopValidationSummary {
    pub run_index: usize,
    pub accepted: bool,
    pub result: LifecycleResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStatusReport {
    pub policy: LoopPolicy,
    pub state: LoopState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopGateReport {
    pub generated_at_unix_ms: u128,
    pub since: String,
    pub verdict: LoopGateVerdict,
    pub reason: String,
    pub signal: LoopGateSignal,
    pub decisions: LoopGateDecisions,
    pub comparisons: LoopGateComparisons,
    pub budget: LoopBudgetSnapshot,
    pub active_duplicate_payloads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopGateVerdict {
    Pass,
    Fail,
    NeedsMoreData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopGateSignal {
    pub session_count: usize,
    pub eval_run_count: usize,
    pub objective_failures: u64,
    pub repeated_errors: u64,
    pub missing_tool_failures: u64,
    pub harness_activation_failures: u64,
    pub harness_adherence_misses: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoopGateDecisions {
    pub considered: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub applied_with_comparison: usize,
    pub applied_without_comparison: usize,
    pub kept_pareto: usize,
    pub rejected: usize,
    pub skipped_duplicate: usize,
    pub frozen_budget: usize,
    pub refused_frozen: usize,
    pub rolled_back_regression: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoopGateComparisons {
    pub considered: usize,
    pub by_outcome: BTreeMap<String, usize>,
    pub best_primary_improvement_ppm: i64,
    pub max_regression_ppm: i64,
    pub latest_artifact_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopAuditSnapshot {
    pub frozen: bool,
    pub freeze_reason: Option<String>,
    pub tokens_used: u64,
    pub modifications_applied: u64,
    pub chained_modifications: u64,
    pub decision_count: usize,
    pub latest_decision: Option<LoopDecision>,
    pub recent_decisions: Vec<LoopDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopPrimaryMetric {
    ObjectiveSuccessRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopMetricSnapshot {
    pub run_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_wall_ms: u128,
    pub average_wall_ms: u128,
    pub objective_success_rate_ppm: u64,
    pub estimated_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopComparisonOutcome {
    Apply,
    KeepPareto,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopComparison {
    pub id: String,
    pub artifact_path: Option<PathBuf>,
    pub primary_metric: LoopPrimaryMetric,
    pub baseline: LoopMetricSnapshot,
    pub candidate: LoopMetricSnapshot,
    pub primary_improvement_ppm: i64,
    pub max_regression_ppm: i64,
    pub min_relative_improvement_ppm: u64,
    pub regression_tolerance_ppm: u64,
    pub outcome: LoopComparisonOutcome,
    pub reason: String,
}

pub async fn run(
    home: &Path,
    workspace: &Path,
    options: LoopRunOptions,
) -> Result<LoopRunReport, String> {
    run_inner(home, workspace, options, None).await
}

/// Run the loop with the solver in validation: autonomous apply now requires the
/// candidate to show a positive measured marginal improvement, not only that the
/// deterministic comparison admits it. This can only make admission stricter.
pub async fn run_with_solver(
    home: &Path,
    workspace: &Path,
    options: LoopRunOptions,
    provider: &dyn ModelProvider,
    config: &Config,
) -> Result<LoopRunReport, String> {
    run_inner(
        home,
        workspace,
        options,
        Some(LoopSolver { provider, config }),
    )
    .await
}

async fn run_inner(
    home: &Path,
    workspace: &Path,
    options: LoopRunOptions,
    solver: Option<LoopSolver<'_>>,
) -> Result<LoopRunReport, String> {
    let policy = load_policy(home)?;
    let mut state = load_state(home)?;
    if state.frozen {
        let reason = state
            .freeze_reason
            .clone()
            .unwrap_or_else(|| "loop is frozen".to_string());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::RefusedFrozen,
            &options.since,
            None,
            reason,
            None,
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: None,
            validation_runs: Vec::new(),
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    let audit_report = audit::build_window_report(home, &options.since)?;
    if options.mode == LoopMode::Apply
        && regression_detected(&audit_report)
        && let Some(result) = rollback_latest_active(home, &audit_report)?
    {
        state.frozen = true;
        state.freeze_reason = Some("regression evidence triggered rollback".to_string());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::RolledBackRegression,
            &options.since,
            Some(result.id.clone()),
            rollback_reason(&audit_report),
            None,
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: true,
            mode: options.mode,
            decision,
            proposed_id: None,
            validation_runs: Vec::new(),
            applied: None,
            rollback: Some(result),
            policy,
            state,
        }));
    }

    if let Some(reason) = budget_refusal_reason(&state, &policy) {
        state.frozen = true;
        state.freeze_reason = Some(reason.clone());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::FrozenBudget,
            &options.since,
            None,
            reason,
            None,
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: None,
            validation_runs: Vec::new(),
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    let proposed = modification::propose_from_audit(home, &audit_report)?;
    let (proposed, reuse_note) = match select_candidate(home, proposed)? {
        CandidateSelection::Use {
            proposed,
            reuse_note,
        } => (proposed, reuse_note),
        CandidateSelection::SkipActive { rejected } => {
            let decision = push_decision(
                &mut state,
                &policy,
                LoopDecisionKind::SkippedDuplicate,
                &options.since,
                Some(rejected.id.clone()),
                "candidate matches an active modification and was rejected".to_string(),
                None,
            );
            save_state(home, &state)?;
            return Ok(report(LoopRunReportDraft {
                success: true,
                mode: options.mode,
                decision,
                proposed_id: Some(rejected.id),
                validation_runs: Vec::new(),
                applied: None,
                rollback: None,
                policy,
                state,
            }));
        }
    };

    let validation_started = Instant::now();
    let required_runs = policy.thresholds.validation_runs_required.max(1);
    let max_wall =
        std::time::Duration::from_secs(policy.budgets.max_wall_seconds_per_validation.max(1));
    let runs_future = async {
        let mut runs = Vec::new();
        for index in 1..=required_runs {
            let result = modification::validate(home, workspace, &proposed.id).await?;
            let accepted = result
                .manifest
                .validation
                .as_ref()
                .is_some_and(|validation| validation.accepted);
            runs.push(LoopValidationSummary {
                run_index: index,
                accepted,
                result,
            });
            if !accepted && policy.budgets.early_stop_on_first_regression {
                break;
            }
        }
        Ok::<Vec<LoopValidationSummary>, String>(runs)
    };
    let validation_runs = match tokio::time::timeout(max_wall, runs_future).await {
        Ok(result) => result?,
        Err(_elapsed) => {
            // Real-time wall guard: abort a runaway validation instead of only
            // freezing the next cycle after the time was already spent.
            let reason = format!(
                "validation aborted: exceeded max_wall_seconds_per_validation {}s in real time",
                policy.budgets.max_wall_seconds_per_validation
            );
            state.frozen = true;
            state.freeze_reason = Some(reason.clone());
            state.validation_wall_ms_used += validation_started.elapsed().as_millis();
            let decision = push_decision(
                &mut state,
                &policy,
                LoopDecisionKind::FrozenBudget,
                &options.since,
                Some(proposed.id.clone()),
                reason,
                None,
            );
            save_state(home, &state)?;
            return Ok(report(LoopRunReportDraft {
                success: false,
                mode: options.mode,
                decision,
                proposed_id: Some(proposed.id),
                validation_runs: Vec::new(),
                applied: None,
                rollback: None,
                policy,
                state,
            }));
        }
    };
    let validation_wall_ms = validation_started.elapsed().as_millis();
    state.validation_wall_ms_used += validation_wall_ms;
    let mut comparison = compare_candidate(&audit_report, &validation_runs, &policy.thresholds)?;
    state.tokens_used = state
        .tokens_used
        .saturating_add(comparison.candidate.estimated_tokens);
    persist_comparison(home, &mut comparison)?;

    if validation_wall_ms > u128::from(policy.budgets.max_wall_seconds_per_validation) * 1_000 {
        let reason = format!(
            "validation wall time {}ms exceeded max_wall_seconds_per_validation {}s",
            validation_wall_ms, policy.budgets.max_wall_seconds_per_validation
        );
        state.frozen = true;
        state.freeze_reason = Some(reason.clone());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::FrozenBudget,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    if comparison.candidate.estimated_tokens > policy.budgets.max_tokens_per_validation {
        let reason = format!(
            "validation estimated tokens {} exceeded max_tokens_per_validation {}",
            comparison.candidate.estimated_tokens, policy.budgets.max_tokens_per_validation
        );
        state.frozen = true;
        state.freeze_reason = Some(reason.clone());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::FrozenBudget,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    if state.tokens_used > policy.budgets.max_tokens_per_window {
        let reason = format!(
            "window estimated tokens {} exceeded max_tokens_per_window {}",
            state.tokens_used, policy.budgets.max_tokens_per_window
        );
        state.frozen = true;
        state.freeze_reason = Some(reason.clone());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::FrozenBudget,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    let (_, manifest) = modification::read_by_id(home, &proposed.id)?;
    if !eligible_for_autonomous_apply(&manifest.layer) {
        let reason = format!(
            "layer {:?} is not enabled for autonomous application",
            manifest.layer
        );
        state.frozen = true;
        state.freeze_reason = Some(reason.clone());
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::FrozenBudget,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    if manifest.state == ModificationState::Rejected
        || validation_runs.iter().any(|run| !run.accepted)
    {
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::Rejected,
            &options.since,
            Some(proposed.id.clone()),
            "candidate failed validation or early regression guard".to_string(),
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    if comparison.outcome == LoopComparisonOutcome::Reject {
        let rejected = modification::reject(home, &proposed.id, comparison.reason.clone())?;
        let reason = format!(
            "candidate rejected by comparative evidence: {}",
            comparison.reason
        );
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::Rejected,
            &options.since,
            Some(rejected.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: false,
            mode: options.mode,
            decision,
            proposed_id: Some(rejected.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    if comparison.outcome == LoopComparisonOutcome::KeepPareto {
        let mut reason = format!("candidate kept on Pareto frontier: {}", comparison.reason);
        if let Some(note) = reuse_note {
            reason = format!("{reason}; {note}");
        }
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::KeptPareto,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: true,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    // Solver gate: with the solver wired in, a positive *measured* marginal
    // improvement is a hard precondition for autonomous apply. The deterministic
    // comparison having admitted the candidate is necessary but not sufficient.
    let solver_evidence = if let Some(ref solver) = solver {
        let measured = modification::solver_compare(
            home,
            workspace,
            &proposed.id,
            solver.config,
            solver.provider,
        )
        .await?;
        if measured.primary_improvement_ppm <= 0 {
            let reason = format!(
                "solver gate: no measured marginal improvement (primary_improvement_ppm={}, baseline_ppm={}, candidate_ppm={})",
                measured.primary_improvement_ppm,
                measured.baseline_success_ppm,
                measured.candidate_success_ppm
            );
            let rejected = modification::reject(home, &proposed.id, reason.clone())?;
            let decision = push_decision(
                &mut state,
                &policy,
                LoopDecisionKind::Rejected,
                &options.since,
                Some(rejected.id.clone()),
                reason,
                Some(comparison),
            );
            save_state(home, &state)?;
            return Ok(report(LoopRunReportDraft {
                success: false,
                mode: options.mode,
                decision,
                proposed_id: Some(rejected.id),
                validation_runs,
                applied: None,
                rollback: None,
                policy,
                state,
            }));
        }
        Some(format!(
            "solver gate passed: measured primary_improvement_ppm={}",
            measured.primary_improvement_ppm
        ))
    } else {
        None
    };

    if options.mode == LoopMode::DryRun {
        let mut reason =
            "dry-run comparative evidence passed and stopped before application".to_string();
        if let Some(note) = reuse_note {
            reason = format!("{reason}; {note}");
        }
        if let Some(evidence) = solver_evidence.as_ref() {
            reason = format!("{reason}; {evidence}");
        }
        let decision = push_decision(
            &mut state,
            &policy,
            LoopDecisionKind::WouldApply,
            &options.since,
            Some(proposed.id.clone()),
            reason,
            Some(comparison),
        );
        save_state(home, &state)?;
        return Ok(report(LoopRunReportDraft {
            success: true,
            mode: options.mode,
            decision,
            proposed_id: Some(proposed.id),
            validation_runs,
            applied: None,
            rollback: None,
            policy,
            state,
        }));
    }

    let active_before = modification::snapshot(home)?
        .active
        .into_iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    state.checkpoints.push(LoopCheckpoint {
        id: format!("checkpoint-{}", now_millis()),
        created_at_unix_ms: now_millis(),
        modification_id: proposed.id.clone(),
        active_before,
    });
    let applied = modification::apply(home, &proposed.id)?;
    state.modifications_applied += 1;
    state.chained_modifications += 1;
    let mut reason = "candidate passed comparative threshold and was applied".to_string();
    if let Some(note) = reuse_note {
        reason = format!("{reason}; {note}");
    }
    if let Some(evidence) = solver_evidence.as_ref() {
        reason = format!("{reason}; {evidence}");
    }
    let mut kind = LoopDecisionKind::Applied;
    if let Some(freeze_reason) = post_apply_freeze_reason(&state, &policy) {
        state.frozen = true;
        state.freeze_reason = Some(freeze_reason.clone());
        reason = format!("{reason}; {freeze_reason}");
        kind = LoopDecisionKind::FrozenBudget;
    }
    let decision = push_decision(
        &mut state,
        &policy,
        kind,
        &options.since,
        Some(applied.id.clone()),
        reason,
        Some(comparison),
    );
    save_state(home, &state)?;
    Ok(report(LoopRunReportDraft {
        success: true,
        mode: options.mode,
        decision,
        proposed_id: Some(proposed.id),
        validation_runs,
        applied: Some(applied),
        rollback: None,
        policy,
        state,
    }))
}

pub fn status(home: &Path) -> Result<LoopStatusReport, String> {
    Ok(LoopStatusReport {
        policy: load_policy(home)?,
        state: load_state(home)?,
    })
}

pub fn gate(home: &Path, since: &str) -> Result<LoopGateReport, String> {
    let audit_report = audit::build_window_report(home, since)?;
    gate_from_audit(home, &audit_report)
}

pub fn gate_from_audit(home: &Path, audit_report: &AuditReport) -> Result<LoopGateReport, String> {
    let policy = load_policy(home)?;
    let state = load_state(home)?;
    let generated_at_unix_ms = now_millis();
    let cutoff_ms = since_cutoff_ms(&audit_report.since, generated_at_unix_ms)?;
    let decisions = state
        .decisions
        .iter()
        .filter(|decision| cutoff_ms.is_none_or(|cutoff| decision.at_unix_ms >= cutoff))
        .cloned()
        .collect::<Vec<_>>();
    let decision_summary = summarize_gate_decisions(&decisions);
    let comparison_summary = summarize_gate_comparisons(&decisions);
    let active_duplicate_payloads = active_duplicate_payloads(home)?;
    let signal = LoopGateSignal {
        session_count: audit_report.session_count,
        eval_run_count: audit_report.eval_run_count,
        objective_failures: audit_report.metrics.objective_failures,
        repeated_errors: audit_report.metrics.repeated_errors,
        missing_tool_failures: audit_report.metrics.missing_tool_failures,
        harness_activation_failures: audit_report.metrics.harness_activation_failures,
        harness_adherence_misses: audit_report.metrics.harness_adherence_misses,
    };
    let budget = budget_snapshot(&state, &policy);
    let (verdict, reason) = gate_verdict(
        &policy,
        &signal,
        &decision_summary,
        &comparison_summary,
        active_duplicate_payloads,
    );

    Ok(LoopGateReport {
        generated_at_unix_ms,
        since: audit_report.since.clone(),
        verdict,
        reason,
        signal,
        decisions: decision_summary,
        comparisons: comparison_summary,
        budget,
        active_duplicate_payloads,
    })
}

pub fn freeze(home: &Path, reason: String) -> Result<LoopStatusReport, String> {
    let policy = load_policy(home)?;
    let mut state = load_state(home)?;
    state.frozen = true;
    state.freeze_reason = Some(reason.clone());
    push_decision(
        &mut state,
        &policy,
        LoopDecisionKind::OperatorFrozen,
        "manual",
        None,
        reason,
        None,
    );
    save_state(home, &state)?;
    Ok(LoopStatusReport { policy, state })
}

pub fn unfreeze(home: &Path) -> Result<LoopStatusReport, String> {
    let policy = load_policy(home)?;
    let mut state = load_state(home)?;
    state.frozen = false;
    state.freeze_reason = None;
    state.window_started_at_unix_ms = now_millis();
    state.tokens_used = 0;
    state.validation_wall_ms_used = 0;
    state.modifications_applied = 0;
    state.chained_modifications = 0;
    push_decision(
        &mut state,
        &policy,
        LoopDecisionKind::OperatorUnfrozen,
        "manual",
        None,
        "operator started a fresh autonomous window".to_string(),
        None,
    );
    save_state(home, &state)?;
    Ok(LoopStatusReport { policy, state })
}

pub fn audit_snapshot(home: &Path) -> Result<Option<LoopAuditSnapshot>, String> {
    let path = state_path(home);
    if !path.exists() {
        return Ok(None);
    }
    let state = load_state(home)?;
    let mut recent_decisions = state
        .decisions
        .iter()
        .rev()
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    recent_decisions.reverse();
    Ok(Some(LoopAuditSnapshot {
        frozen: state.frozen,
        freeze_reason: state.freeze_reason,
        tokens_used: state.tokens_used,
        modifications_applied: state.modifications_applied,
        chained_modifications: state.chained_modifications,
        decision_count: state.decisions.len(),
        latest_decision: state.decisions.last().cloned(),
        recent_decisions,
    }))
}

pub fn render_run_report(report: &LoopRunReport) -> String {
    let mut lines = vec![
        format!("loop mode: {:?}", report.mode),
        format!("success: {}", report.success),
        format!("decision: {:?}", report.decision.kind),
        format!("reason: {}", report.decision.reason),
        format!(
            "modification: {}",
            report.decision.modification_id.as_deref().unwrap_or("none")
        ),
        format!("frozen: {}", report.state.frozen),
    ];
    if let Some(comparison) = &report.decision.comparison {
        lines.push(format!("comparison: {:?}", comparison.outcome));
        lines.push(format!(
            "primary_improvement_ppm: {}",
            comparison.primary_improvement_ppm
        ));
        lines.push(format!(
            "max_regression_ppm: {}",
            comparison.max_regression_ppm
        ));
        if let Some(path) = &comparison.artifact_path {
            lines.push(format!("comparison_artifact: {}", path.display()));
        }
    }
    lines.join("\n")
}

pub fn render_status(report: &LoopStatusReport) -> String {
    let mut lines = vec![
        format!("frozen: {}", report.state.frozen),
        format!(
            "freeze_reason: {}",
            report.state.freeze_reason.as_deref().unwrap_or("none")
        ),
        format!(
            "budget: modifications {}/{} chained {}/{} validation_wall_ms {}",
            report.state.modifications_applied,
            report.policy.budgets.max_modifications_per_window,
            report.state.chained_modifications,
            report.policy.budgets.max_chained_modifications,
            report.state.validation_wall_ms_used
        ),
    ];
    if let Some(decision) = report.state.decisions.last() {
        lines.push(format!(
            "latest_decision: {:?} {}",
            decision.kind, decision.reason
        ));
        if let Some(comparison) = &decision.comparison {
            lines.push(format!(
                "latest_comparison: {:?} primary_improvement_ppm={} max_regression_ppm={}",
                comparison.outcome,
                comparison.primary_improvement_ppm,
                comparison.max_regression_ppm
            ));
        }
    }
    lines.join("\n")
}

struct LoopRunReportDraft {
    success: bool,
    mode: LoopMode,
    decision: LoopDecision,
    proposed_id: Option<String>,
    validation_runs: Vec<LoopValidationSummary>,
    applied: Option<LifecycleResult>,
    rollback: Option<LifecycleResult>,
    policy: LoopPolicy,
    state: LoopState,
}

fn report(draft: LoopRunReportDraft) -> LoopRunReport {
    LoopRunReport {
        success: draft.success,
        mode: draft.mode,
        decision: draft.decision,
        proposed_id: draft.proposed_id,
        validation_runs: draft.validation_runs,
        applied: draft.applied,
        rollback: draft.rollback,
        policy: draft.policy,
        state: draft.state,
    }
}

pub fn render_gate_report(report: &LoopGateReport) -> String {
    [
        format!("phase3_gate: {:?}", report.verdict),
        format!("reason: {}", report.reason),
        format!("since: {}", report.since),
        format!(
            "signal: sessions={} eval_runs={} objective_failures={} repeated_errors={} missing_tool_failures={} harness_activation_failures={} harness_adherence_misses={}",
            report.signal.session_count,
            report.signal.eval_run_count,
            report.signal.objective_failures,
            report.signal.repeated_errors,
            report.signal.missing_tool_failures,
            report.signal.harness_activation_failures,
            report.signal.harness_adherence_misses
        ),
        format!(
            "decisions: considered={} applied_with_comparison={} kept_pareto={} rejected={} skipped_duplicate={} frozen_budget={} refused_frozen={} rollback={}",
            report.decisions.considered,
            report.decisions.applied_with_comparison,
            report.decisions.kept_pareto,
            report.decisions.rejected,
            report.decisions.skipped_duplicate,
            report.decisions.frozen_budget,
            report.decisions.refused_frozen,
            report.decisions.rolled_back_regression
        ),
        format!(
            "comparisons: considered={} best_primary_improvement_ppm={} max_regression_ppm={}",
            report.comparisons.considered,
            report.comparisons.best_primary_improvement_ppm,
            report.comparisons.max_regression_ppm
        ),
        format!(
            "budget: tokens {}/{} modifications {}/{} chained {}/{}",
            report.budget.tokens_used,
            report.budget.max_tokens_per_window,
            report.budget.modifications_applied,
            report.budget.max_modifications_per_window,
            report.budget.chained_modifications,
            report.budget.max_chained_modifications
        ),
        format!("active_duplicate_payloads: {}", report.active_duplicate_payloads),
    ]
    .join("\n")
}

fn summarize_gate_decisions(decisions: &[LoopDecision]) -> LoopGateDecisions {
    let mut summary = LoopGateDecisions {
        considered: decisions.len(),
        ..Default::default()
    };
    for decision in decisions {
        *summary
            .by_kind
            .entry(decision_kind_key(&decision.kind).to_string())
            .or_insert(0) += 1;
        match decision.kind {
            LoopDecisionKind::Applied => {
                if decision.comparison.is_some() {
                    summary.applied_with_comparison += 1;
                } else {
                    summary.applied_without_comparison += 1;
                }
            }
            LoopDecisionKind::KeptPareto => summary.kept_pareto += 1,
            LoopDecisionKind::Rejected => summary.rejected += 1,
            LoopDecisionKind::SkippedDuplicate => summary.skipped_duplicate += 1,
            LoopDecisionKind::FrozenBudget => summary.frozen_budget += 1,
            LoopDecisionKind::RefusedFrozen => summary.refused_frozen += 1,
            LoopDecisionKind::RolledBackRegression => summary.rolled_back_regression += 1,
            LoopDecisionKind::WouldApply
            | LoopDecisionKind::OperatorFrozen
            | LoopDecisionKind::OperatorUnfrozen => {}
        }
    }
    summary
}

fn summarize_gate_comparisons(decisions: &[LoopDecision]) -> LoopGateComparisons {
    let mut summary = LoopGateComparisons::default();
    for comparison in decisions
        .iter()
        .filter_map(|decision| decision.comparison.as_ref())
    {
        summary.considered += 1;
        *summary
            .by_outcome
            .entry(comparison_outcome_key(&comparison.outcome).to_string())
            .or_insert(0) += 1;
        summary.best_primary_improvement_ppm = summary
            .best_primary_improvement_ppm
            .max(comparison.primary_improvement_ppm);
        summary.max_regression_ppm = summary
            .max_regression_ppm
            .max(comparison.max_regression_ppm);
        summary.latest_artifact_path = comparison.artifact_path.clone();
    }
    summary
}

fn gate_verdict(
    policy: &LoopPolicy,
    signal: &LoopGateSignal,
    decisions: &LoopGateDecisions,
    comparisons: &LoopGateComparisons,
    active_duplicate_payloads: usize,
) -> (LoopGateVerdict, String) {
    if signal.session_count < 10 || signal.eval_run_count < 5 {
        return (
            LoopGateVerdict::NeedsMoreData,
            format!(
                "need at least 10 sessions and 5 eval runs; observed {} sessions and {} eval runs",
                signal.session_count, signal.eval_run_count
            ),
        );
    }
    if active_duplicate_payloads > 0 {
        return (
            LoopGateVerdict::Fail,
            format!(
                "active modification catalog has {active_duplicate_payloads} duplicate payloads"
            ),
        );
    }
    if signal.objective_failures > 0
        || signal.repeated_errors > 0
        || signal.missing_tool_failures > 0
        || signal.harness_activation_failures > 0
        || signal.harness_adherence_misses > 0
    {
        return (
            LoopGateVerdict::Fail,
            format!(
                "protected regression signals present: objective_failures={} repeated_errors={} missing_tool_failures={} harness_activation_failures={} harness_adherence_misses={}",
                signal.objective_failures,
                signal.repeated_errors,
                signal.missing_tool_failures,
                signal.harness_activation_failures,
                signal.harness_adherence_misses
            ),
        );
    }
    if comparisons.considered == 0 {
        return (
            LoopGateVerdict::NeedsMoreData,
            "no comparative loop decisions recorded in this window".to_string(),
        );
    }
    if decisions.applied_without_comparison > 0 && decisions.applied_with_comparison == 0 {
        return (
            LoopGateVerdict::NeedsMoreData,
            "only legacy applied decisions exist; no applied decision carries comparison evidence"
                .to_string(),
        );
    }

    let improvement_threshold = ratio_to_ppm(policy.thresholds.min_relative_improvement) as i64;
    let regression_tolerance = ratio_to_ppm(policy.thresholds.regression_tolerance) as i64;
    if decisions.applied_with_comparison > 0
        && comparisons.best_primary_improvement_ppm >= improvement_threshold
        && comparisons.max_regression_ppm <= regression_tolerance
    {
        return (
            LoopGateVerdict::Pass,
            format!(
                "applied comparative decision met primary threshold {}ppm with max regression {}ppm",
                improvement_threshold, comparisons.max_regression_ppm
            ),
        );
    }

    (
        LoopGateVerdict::NeedsMoreData,
        format!(
            "no applied comparative decision met the primary threshold; best_primary_improvement_ppm={} threshold_ppm={} kept_pareto={} rejected={}",
            comparisons.best_primary_improvement_ppm,
            improvement_threshold,
            decisions.kept_pareto,
            decisions.rejected
        ),
    )
}

fn active_duplicate_payloads(home: &Path) -> Result<usize, String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = 0;
    for entry in modification::list_entries(home, ModificationState::Active)? {
        let (_, manifest) = modification::read_by_id(home, &entry.id)?;
        let key = serde_json::to_string(&manifest.payload).map_err(|err| err.to_string())?;
        if !seen.insert(key) {
            duplicates += 1;
        }
    }
    Ok(duplicates)
}

fn decision_kind_key(kind: &LoopDecisionKind) -> &'static str {
    match kind {
        LoopDecisionKind::WouldApply => "would_apply",
        LoopDecisionKind::Applied => "applied",
        LoopDecisionKind::Rejected => "rejected",
        LoopDecisionKind::KeptPareto => "kept_pareto",
        LoopDecisionKind::SkippedDuplicate => "skipped_duplicate",
        LoopDecisionKind::RefusedFrozen => "refused_frozen",
        LoopDecisionKind::FrozenBudget => "frozen_budget",
        LoopDecisionKind::RolledBackRegression => "rolled_back_regression",
        LoopDecisionKind::OperatorFrozen => "operator_frozen",
        LoopDecisionKind::OperatorUnfrozen => "operator_unfrozen",
    }
}

fn comparison_outcome_key(outcome: &LoopComparisonOutcome) -> &'static str {
    match outcome {
        LoopComparisonOutcome::Apply => "apply",
        LoopComparisonOutcome::KeepPareto => "keep_pareto",
        LoopComparisonOutcome::Reject => "reject",
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

enum CandidateSelection {
    Use {
        proposed: ProposalResult,
        reuse_note: Option<String>,
    },
    SkipActive {
        rejected: LifecycleResult,
    },
}

fn select_candidate(home: &Path, proposed: ProposalResult) -> Result<CandidateSelection, String> {
    if modification::find_equivalent_in_states(
        home,
        &proposed.manifest,
        &[ModificationState::Active],
        Some(&proposed.id),
    )?
    .is_some()
    {
        let rejected = modification::reject(
            home,
            &proposed.id,
            "equivalent active modification already exists".to_string(),
        )?;
        return Ok(CandidateSelection::SkipActive { rejected });
    }

    if let Some(existing) = modification::find_equivalent_in_states(
        home,
        &proposed.manifest,
        &[ModificationState::Proposed, ModificationState::Validated],
        Some(&proposed.id),
    )? {
        let duplicate = modification::reject(
            home,
            &proposed.id,
            format!("equivalent candidate already exists: {}", existing.id),
        )?;
        let (path, manifest) = modification::read_by_id(home, &existing.id)?;
        return Ok(CandidateSelection::Use {
            proposed: ProposalResult {
                id: existing.id.clone(),
                path,
                manifest,
            },
            reuse_note: Some(format!(
                "reused equivalent candidate {}; rejected duplicate {}",
                existing.id, duplicate.id
            )),
        });
    }

    Ok(CandidateSelection::Use {
        proposed,
        reuse_note: None,
    })
}

fn compare_candidate(
    audit_report: &AuditReport,
    validation_runs: &[LoopValidationSummary],
    thresholds: &LoopThresholds,
) -> Result<LoopComparison, String> {
    let baseline = metric_snapshot_from_audit_runs(&audit_report.eval_runs);
    let candidate_reports = read_validation_reports(validation_runs)?;
    let candidate = metric_snapshot_from_eval_reports(&candidate_reports);
    let min_relative_improvement_ppm = ratio_to_ppm(thresholds.min_relative_improvement);
    let regression_tolerance_ppm = ratio_to_ppm(thresholds.regression_tolerance);
    let primary_improvement_ppm =
        candidate.objective_success_rate_ppm as i64 - baseline.objective_success_rate_ppm as i64;
    let success_regression_ppm = (baseline.objective_success_rate_ppm as i64
        - candidate.objective_success_rate_ppm as i64)
        .max(0);
    let wall_regression_ppm = wall_regression_ppm(&baseline, &candidate);
    let max_regression_ppm = success_regression_ppm.max(wall_regression_ppm);
    let wall_improvement_ppm = wall_improvement_ppm(&baseline, &candidate);

    let (outcome, reason) = if baseline.run_count == 0 {
        (
            LoopComparisonOutcome::Reject,
            "no baseline eval runs available for comparative admission".to_string(),
        )
    } else if candidate.run_count == 0 {
        (
            LoopComparisonOutcome::Reject,
            "no candidate validation eval runs available for comparative admission".to_string(),
        )
    } else if primary_improvement_ppm >= min_relative_improvement_ppm as i64
        && max_regression_ppm <= regression_tolerance_ppm as i64
    {
        (
            LoopComparisonOutcome::Apply,
            format!(
                "objective success improved by {}ppm with max regression {}ppm",
                primary_improvement_ppm, max_regression_ppm
            ),
        )
    } else if thresholds.pareto_keep_when_uncomparable
        && max_regression_ppm <= regression_tolerance_ppm as i64
        && wall_improvement_ppm >= min_relative_improvement_ppm as i64
    {
        (
            LoopComparisonOutcome::KeepPareto,
            format!(
                "wall time improved by {}ppm but objective improvement {}ppm is below threshold {}ppm",
                wall_improvement_ppm, primary_improvement_ppm, min_relative_improvement_ppm
            ),
        )
    } else if max_regression_ppm > regression_tolerance_ppm as i64 {
        (
            LoopComparisonOutcome::Reject,
            format!(
                "candidate regressed a protected metric by {}ppm over tolerance {}ppm",
                max_regression_ppm, regression_tolerance_ppm
            ),
        )
    } else {
        (
            LoopComparisonOutcome::Reject,
            format!(
                "objective improvement {}ppm is below threshold {}ppm",
                primary_improvement_ppm, min_relative_improvement_ppm
            ),
        )
    };

    Ok(LoopComparison {
        id: format!("comparison-{}", now_millis()),
        artifact_path: None,
        primary_metric: LoopPrimaryMetric::ObjectiveSuccessRate,
        baseline,
        candidate,
        primary_improvement_ppm,
        max_regression_ppm,
        min_relative_improvement_ppm,
        regression_tolerance_ppm,
        outcome,
        reason,
    })
}

fn persist_comparison(home: &Path, comparison: &mut LoopComparison) -> Result<(), String> {
    let directory = home.join("state").join("comparisons");
    fs::create_dir_all(&directory)
        .map_err(|err| format!("cannot create comparison state dir: {err}"))?;
    let path = directory.join(format!("{}.json", comparison.id));
    comparison.artifact_path = Some(path.clone());
    let rendered = serde_json::to_string_pretty(comparison).map_err(|err| err.to_string())?;
    fs::write(&path, rendered).map_err(|err| format!("cannot write loop comparison: {err}"))
}

fn read_validation_reports(
    validation_runs: &[LoopValidationSummary],
) -> Result<Vec<EvalRunReport>, String> {
    let mut reports = Vec::new();
    for run in validation_runs {
        if let Some(validation) = &run.result.manifest.validation {
            for path in &validation.eval_runs {
                let content = fs::read_to_string(path)
                    .map_err(|err| format!("cannot read eval run {}: {err}", path.display()))?;
                let report: EvalRunReport = serde_json::from_str(&content)
                    .map_err(|err| format!("cannot parse eval run {}: {err}", path.display()))?;
                reports.push(report);
            }
        }
    }
    Ok(reports)
}

fn metric_snapshot_from_audit_runs(runs: &[AuditEvalRun]) -> LoopMetricSnapshot {
    let run_count = runs.len();
    let success_count = runs.iter().filter(|run| run.success).count();
    let total_wall_ms = runs.iter().map(|run| run.wall_ms).sum::<u128>();
    LoopMetricSnapshot {
        run_count,
        success_count,
        failure_count: run_count.saturating_sub(success_count),
        total_wall_ms,
        average_wall_ms: average_wall_ms(total_wall_ms, run_count),
        objective_success_rate_ppm: success_rate_ppm(success_count, run_count),
        estimated_tokens: 0,
    }
}

fn metric_snapshot_from_eval_reports(runs: &[EvalRunReport]) -> LoopMetricSnapshot {
    let run_count = runs.len();
    let success_count = runs.iter().filter(|run| run.success).count();
    let total_wall_ms = runs.iter().map(|run| run.wall_ms).sum::<u128>();
    let estimated_tokens = runs.iter().map(estimate_eval_tokens).sum::<u64>();
    LoopMetricSnapshot {
        run_count,
        success_count,
        failure_count: run_count.saturating_sub(success_count),
        total_wall_ms,
        average_wall_ms: average_wall_ms(total_wall_ms, run_count),
        objective_success_rate_ppm: success_rate_ppm(success_count, run_count),
        estimated_tokens,
    }
}

fn estimate_eval_tokens(report: &EvalRunReport) -> u64 {
    let mut chars = report.task.id.len() + report.task.title.len() + report.task.kind.len();
    for criterion in &report.criteria {
        chars += criterion.id.len() + criterion.command.len() + criterion.output.len();
    }
    ((chars as u64).saturating_add(3) / 4).max(1)
}

fn average_wall_ms(total_wall_ms: u128, run_count: usize) -> u128 {
    if run_count == 0 {
        0
    } else {
        total_wall_ms / run_count as u128
    }
}

fn success_rate_ppm(success_count: usize, run_count: usize) -> u64 {
    if run_count == 0 {
        0
    } else {
        ((success_count as u128 * 1_000_000) / run_count as u128) as u64
    }
}

fn ratio_to_ppm(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    (value * 1_000_000.0).round() as u64
}

fn wall_regression_ppm(baseline: &LoopMetricSnapshot, candidate: &LoopMetricSnapshot) -> i64 {
    if baseline.average_wall_ms == 0 || candidate.average_wall_ms <= baseline.average_wall_ms {
        return 0;
    }
    (((candidate.average_wall_ms - baseline.average_wall_ms) * 1_000_000)
        / baseline.average_wall_ms) as i64
}

fn wall_improvement_ppm(baseline: &LoopMetricSnapshot, candidate: &LoopMetricSnapshot) -> i64 {
    if baseline.average_wall_ms == 0 || candidate.average_wall_ms >= baseline.average_wall_ms {
        return 0;
    }
    (((baseline.average_wall_ms - candidate.average_wall_ms) * 1_000_000)
        / baseline.average_wall_ms) as i64
}

fn load_policy(home: &Path) -> Result<LoopPolicy, String> {
    Ok(LoopPolicy {
        budgets: load_or_default(&home.join("state").join("budgets.json"))?,
        thresholds: load_or_default(&home.join("state").join("thresholds.json"))?,
    })
}

fn load_state(home: &Path) -> Result<LoopState, String> {
    load_or_default(&state_path(home))
}

fn save_state(home: &Path, state: &LoopState) -> Result<(), String> {
    let path = state_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("cannot create state dir: {err}"))?;
    }
    let rendered = serde_json::to_string_pretty(state).map_err(|err| err.to_string())?;
    fs::write(path, rendered).map_err(|err| format!("cannot write loop state: {err}"))
}

fn load_or_default<T>(path: &Path) -> Result<T, String>
where
    T: Default + for<'de> Deserialize<'de>,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let content =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    serde_json::from_str(&content).map_err(|err| format!("cannot parse {}: {err}", path.display()))
}

fn state_path(home: &Path) -> PathBuf {
    home.join("state").join("loop-state.json")
}

fn push_decision(
    state: &mut LoopState,
    policy: &LoopPolicy,
    kind: LoopDecisionKind,
    since: &str,
    modification_id: Option<String>,
    reason: String,
    comparison: Option<LoopComparison>,
) -> LoopDecision {
    let decision = LoopDecision {
        id: format!("decision-{}", now_millis()),
        kind,
        at_unix_ms: now_millis(),
        since: since.to_string(),
        modification_id,
        reason,
        budget: budget_snapshot(state, policy),
        comparison,
    };
    state.decisions.push(decision.clone());
    if state.decisions.len() > 100 {
        state.decisions.remove(0);
    }
    decision
}

fn budget_snapshot(state: &LoopState, policy: &LoopPolicy) -> LoopBudgetSnapshot {
    LoopBudgetSnapshot {
        tokens_used: state.tokens_used,
        max_tokens_per_window: policy.budgets.max_tokens_per_window,
        validation_wall_ms_used: state.validation_wall_ms_used,
        max_wall_seconds_per_validation: policy.budgets.max_wall_seconds_per_validation,
        modifications_applied: state.modifications_applied,
        max_modifications_per_window: policy.budgets.max_modifications_per_window,
        chained_modifications: state.chained_modifications,
        max_chained_modifications: policy.budgets.max_chained_modifications,
    }
}

fn budget_refusal_reason(state: &LoopState, policy: &LoopPolicy) -> Option<String> {
    if state.tokens_used >= policy.budgets.max_tokens_per_window {
        return Some("max_tokens_per_window exhausted".to_string());
    }
    if state.modifications_applied >= policy.budgets.max_modifications_per_window {
        return Some("max_modifications_per_window exhausted".to_string());
    }
    if state.chained_modifications >= policy.budgets.max_chained_modifications {
        return Some("max_chained_modifications exhausted".to_string());
    }
    None
}

fn post_apply_freeze_reason(state: &LoopState, policy: &LoopPolicy) -> Option<String> {
    if state.modifications_applied >= policy.budgets.max_modifications_per_window {
        return Some("max_modifications_per_window reached; loop frozen until audit".to_string());
    }
    if state.chained_modifications >= policy.budgets.max_chained_modifications {
        return Some("max_chained_modifications reached; loop frozen until audit".to_string());
    }
    None
}

fn eligible_for_autonomous_apply(layer: &ModificationLayer) -> bool {
    matches!(layer, ModificationLayer::A | ModificationLayer::S1)
}

fn regression_detected(report: &AuditReport) -> bool {
    report.metrics.objective_failures > 0
        || report.metrics.repeated_errors > 0
        || report.metrics.missing_tool_failures > 0
        || report.metrics.harness_activation_failures > 0
        || report.metrics.harness_adherence_misses > 0
}

fn rollback_reason(report: &AuditReport) -> String {
    format!(
        "audit regression evidence: objective_failures={} repeated_errors={} missing_tool_failures={} harness_activation_failures={} harness_adherence_misses={}",
        report.metrics.objective_failures,
        report.metrics.repeated_errors,
        report.metrics.missing_tool_failures,
        report.metrics.harness_activation_failures,
        report.metrics.harness_adherence_misses
    )
}

fn rollback_latest_active(
    home: &Path,
    report: &AuditReport,
) -> Result<Option<LifecycleResult>, String> {
    let snapshot = modification::snapshot(home)?;
    let Some(entry) = snapshot.active.into_iter().last() else {
        return Ok(None);
    };
    let result = modification::revert(home, &entry.id)?;
    let _ = report;
    Ok(Some(result))
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests;
