use std::{fs, path::PathBuf};

use super::*;

#[test]
fn gate_passes_with_applied_comparative_improvement() {
    let root = temp_dir("gate-pass");
    let home = root.join(".greco");
    seed_gate_signal(&home);
    let mut state = LoopState {
        modifications_applied: 1,
        ..Default::default()
    };
    state.decisions.push(test_decision(
        LoopDecisionKind::Applied,
        Some(test_comparison(LoopComparisonOutcome::Apply, 100_000, 0)),
    ));
    save_state(&home, &state).unwrap();

    let report = gate(&home, "all").unwrap();

    assert_eq!(report.verdict, LoopGateVerdict::Pass);
    assert_eq!(report.decisions.applied_with_comparison, 1);
    assert!(report.reason.contains("met primary threshold"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn gate_fails_with_active_duplicate_payloads() {
    let root = temp_dir("gate-fail-duplicates");
    let home = root.join(".greco");
    seed_gate_signal(&home);
    seed_active_modification_with_id(&home, "layer-a-one");
    seed_active_modification_with_id(&home, "layer-a-two");
    save_state(&home, &LoopState::default()).unwrap();

    let report = gate(&home, "all").unwrap();

    assert_eq!(report.verdict, LoopGateVerdict::Fail);
    assert_eq!(report.active_duplicate_payloads, 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn gate_requires_comparative_window_data() {
    let root = temp_dir("gate-insufficient");
    let home = root.join(".greco");
    save_state(&home, &LoopState::default()).unwrap();

    let report = gate(&home, "all").unwrap();

    assert_eq!(report.verdict, LoopGateVerdict::NeedsMoreData);
    assert!(report.reason.contains("need at least 10 sessions"));
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn dry_run_validates_without_applying() {
    let root = temp_dir("dry-run");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_eval_baseline(&home, false, 1_000);
    seed_success_trace(&home);

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::DryRun,
        },
    )
    .await
    .unwrap();

    assert!(report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::WouldApply);
    let comparison = report.decision.comparison.as_ref().unwrap();
    assert_eq!(comparison.outcome, LoopComparisonOutcome::Apply);
    assert!(comparison.artifact_path.as_ref().unwrap().exists());
    assert!(modification::snapshot(&home).unwrap().active.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn apply_mode_applies_and_records_checkpoint() {
    let root = temp_dir("apply");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_eval_baseline(&home, false, 1_000);
    seed_success_trace(&home);

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();

    assert!(report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::Applied);
    assert_eq!(
        report.decision.comparison.as_ref().unwrap().outcome,
        LoopComparisonOutcome::Apply
    );
    assert_eq!(modification::snapshot(&home).unwrap().active.len(), 1);
    assert_eq!(status(&home).unwrap().state.checkpoints.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn candidate_without_primary_improvement_is_rejected() {
    let root = temp_dir("reject-no-delta");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_eval_baseline(&home, true, 0);
    seed_success_trace(&home);

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();
    let snapshot = modification::snapshot(&home).unwrap();

    assert!(!report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::Rejected);
    assert_eq!(
        report.decision.comparison.as_ref().unwrap().outcome,
        LoopComparisonOutcome::Reject
    );
    assert!(snapshot.active.is_empty());
    assert_eq!(snapshot.rejected.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn equivalent_pending_candidate_is_reused_and_duplicate_rejected() {
    let root = temp_dir("reuse-pending");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_eval_baseline(&home, false, 1_000);
    seed_success_trace(&home);
    let audit = audit::build_window_report(&home, "all").unwrap();
    let existing = modification::propose_from_audit(&home, &audit).unwrap();

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();
    let snapshot = modification::snapshot(&home).unwrap();

    assert!(report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::Applied);
    assert_eq!(report.applied.as_ref().unwrap().id, existing.id);
    assert_eq!(snapshot.active.len(), 1);
    assert_eq!(snapshot.rejected.len(), 1);
    assert!(
        report
            .decision
            .reason
            .contains("reused equivalent candidate")
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn duplicate_active_candidate_is_rejected_without_validation() {
    let root = temp_dir("duplicate");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_success_trace(&home);
    let report = audit::build_window_report(&home, "all").unwrap();
    let proposed = modification::propose_from_audit(&home, &report).unwrap();
    modification::validate(&home, &root, &proposed.id)
        .await
        .unwrap();
    modification::apply(&home, &proposed.id).unwrap();

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();
    let snapshot = modification::snapshot(&home).unwrap();

    assert!(report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::SkippedDuplicate);
    assert!(report.validation_runs.is_empty());
    assert_eq!(snapshot.active.len(), 1);
    assert_eq!(snapshot.rejected.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn budget_exhaustion_freezes_without_applying() {
    let root = temp_dir("budget");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_success_trace(&home);
    fs::create_dir_all(home.join("state")).unwrap();
    fs::write(
            home.join("state").join("budgets.json"),
            r#"{"max_tokens_per_window":100000,"max_modifications_per_window":0,"max_chained_modifications":2,"max_tokens_per_validation":50000,"max_wall_seconds_per_validation":300,"early_stop_on_first_regression":true}"#,
        )
        .unwrap();

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();

    assert!(!report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::FrozenBudget);
    assert!(report.state.frozen);
    assert!(modification::snapshot(&home).unwrap().active.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn regression_evidence_rolls_back_active_modification() {
    let root = temp_dir("rollback");
    let home = root.join(".greco");
    seed_active_modification(&home);
    seed_failure_trace(&home);

    let report = run(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
    )
    .await
    .unwrap();

    assert!(report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::RolledBackRegression);
    assert!(report.rollback.is_some());
    assert!(modification::snapshot(&home).unwrap().active.is_empty());
    assert_eq!(modification::snapshot(&home).unwrap().validated.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

struct NoopSolver;

impl crate::provider::ModelProvider for NoopSolver {
    fn respond<'a>(
        &'a self,
        _request: crate::provider::ModelRequest,
    ) -> crate::provider::ProviderFuture<'a, crate::provider::ModelResponse> {
        Box::pin(async move {
            Ok(crate::provider::ModelResponse {
                id: "r".to_string(),
                output_text: "done".to_string(),
                tool_calls: Vec::new(),
                output_items: vec![serde_json::json!({
                    "type": "message",
                    "content": [{"type": "output_text", "text": "done"}]
                })],
                raw: serde_json::json!({"id": "r"}),
            })
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
async fn apply_mode_with_solver_rejects_without_measured_improvement() {
    let root = temp_dir("apply-solver-gate");
    let home = root.join(".greco");
    seed_eval(&home);
    seed_eval_baseline(&home, false, 1_000);
    seed_success_trace(&home);
    let config = Config {
        provider: "openai".to_string(),
        model: "test".to_string(),
        api_key: None,
        api_key_source: None,
        home: home.clone(),
        workspace: root.clone(),
    };

    let report = run_with_solver(
        &home,
        &root,
        LoopRunOptions {
            since: "all".to_string(),
            mode: LoopMode::Apply,
        },
        &NoopSolver,
        &config,
    )
    .await
    .unwrap();
    let snapshot = modification::snapshot(&home).unwrap();

    // The deterministic comparison admitted the candidate, but the solver
    // measured no marginal improvement on the `true` criterion (delta 0), so
    // the gate blocks autonomous apply: nothing is activated and the
    // candidate is rejected with a solver-gate reason.
    assert!(!report.success);
    assert_eq!(report.decision.kind, LoopDecisionKind::Rejected);
    assert!(report.decision.reason.contains("solver gate"));
    assert!(snapshot.active.is_empty());
    assert_eq!(snapshot.rejected.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

fn seed_eval(home: &Path) {
    let eval_task = home.join("eval").join("demo");
    fs::create_dir_all(&eval_task).unwrap();
    fs::write(
            eval_task.join("task.json"),
            r#"{"id":"demo","title":"Demo","kind":"search","prompt":"Demo","criteria":[{"id":"ok","command":"true","timeout_seconds":5}]}"#,
        )
        .unwrap();
}

fn seed_eval_baseline(home: &Path, success: bool, wall_ms: u128) {
    let runs = home.join("eval").join("runs");
    fs::create_dir_all(&runs).unwrap();
    let exit_code = if success { 0 } else { 1 };
    fs::write(
            runs.join("1000-demo.json"),
            format!(
                r#"{{"task":{{"id":"demo","title":"Demo","kind":"search","criteria":1}},"success":{},"generated_at_unix_ms":1000,"wall_ms":{},"criteria":[{{"id":"ok","command":"true","success":{},"exit_code":{},"timed_out":false,"wall_ms":{},"output":""}}],"run_path":null}}"#,
                success, wall_ms, success, exit_code, wall_ms
            ),
        )
        .unwrap();
}

fn seed_gate_signal(home: &Path) {
    let sessions = home.join("traces").join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    for index in 0..10 {
        fs::write(
                sessions.join(format!("session-{index}.jsonl")),
                format!(
                    r#"{{"ts_unix_ms":{},"event":"friction_summary","data":{{"turns":2,"tool_calls":1,"tokens":1000,"repeated_errors":0,"retracements":0,"avoidable_prompts":0,"missing_tool_failures":0,"objective_success":true}}}}"#,
                    1_000 + index
                ),
            )
            .unwrap();
    }
    let runs = home.join("eval").join("runs");
    fs::create_dir_all(&runs).unwrap();
    for index in 0..5 {
        fs::write(
                runs.join(format!("{}-demo-{index}.json", 1_000 + index)),
                format!(
                    r#"{{"task":{{"id":"demo-{index}","title":"Demo","kind":"search","criteria":1}},"success":true,"generated_at_unix_ms":{},"wall_ms":100,"criteria":[{{"id":"ok","command":"true","success":true,"exit_code":0,"timed_out":false,"wall_ms":100,"output":""}}],"run_path":null}}"#,
                    1_000 + index
                ),
            )
            .unwrap();
    }
}

fn test_decision(kind: LoopDecisionKind, comparison: Option<LoopComparison>) -> LoopDecision {
    LoopDecision {
        id: format!("decision-{}", now_millis()),
        kind,
        at_unix_ms: now_millis(),
        since: "all".to_string(),
        modification_id: Some("layer-a-test".to_string()),
        reason: "test decision".to_string(),
        budget: LoopBudgetSnapshot {
            tokens_used: 0,
            max_tokens_per_window: 100_000,
            validation_wall_ms_used: 0,
            max_wall_seconds_per_validation: 300,
            modifications_applied: 1,
            max_modifications_per_window: 2,
            chained_modifications: 1,
            max_chained_modifications: 2,
        },
        comparison,
    }
}

fn test_comparison(
    outcome: LoopComparisonOutcome,
    primary_improvement_ppm: i64,
    max_regression_ppm: i64,
) -> LoopComparison {
    LoopComparison {
        id: format!("comparison-{}", now_millis()),
        artifact_path: Some(PathBuf::from(".greco/state/comparisons/test.json")),
        primary_metric: LoopPrimaryMetric::ObjectiveSuccessRate,
        baseline: LoopMetricSnapshot {
            run_count: 5,
            success_count: 4,
            failure_count: 1,
            total_wall_ms: 500,
            average_wall_ms: 100,
            objective_success_rate_ppm: 800_000,
            estimated_tokens: 0,
        },
        candidate: LoopMetricSnapshot {
            run_count: 5,
            success_count: 5,
            failure_count: 0,
            total_wall_ms: 400,
            average_wall_ms: 80,
            objective_success_rate_ppm: 1_000_000,
            estimated_tokens: 100,
        },
        primary_improvement_ppm,
        max_regression_ppm,
        min_relative_improvement_ppm: 50_000,
        regression_tolerance_ppm: 10_000,
        outcome,
        reason: "test comparison".to_string(),
    }
}

fn seed_success_trace(home: &Path) {
    seed_trace(
        home,
        r#"{"ts_unix_ms":1000,"event":"friction_summary","data":{"turns":2,"tool_calls":1,"tokens":5000,"repeated_errors":0,"retracements":0,"avoidable_prompts":0,"missing_tool_failures":0,"objective_success":true}}"#,
    );
}

fn seed_failure_trace(home: &Path) {
    seed_trace(
        home,
        r#"{"ts_unix_ms":1000,"event":"friction_summary","data":{"turns":2,"tool_calls":1,"tokens":5000,"repeated_errors":0,"retracements":0,"avoidable_prompts":0,"missing_tool_failures":0,"objective_success":false}}"#,
    );
}

fn seed_trace(home: &Path, content: &str) {
    let sessions = home.join("traces").join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(sessions.join("demo.jsonl"), content).unwrap();
}

fn seed_active_modification(home: &Path) {
    seed_active_modification_with_id(home, "layer-a-test");
}

fn seed_active_modification_with_id(home: &Path, id: &str) {
    let active_dir = home.join("modifications").join("active").join(id);
    fs::create_dir_all(&active_dir).unwrap();
    fs::write(
            active_dir.join("manifest.json"),
            format!(
                r#"{{"id":"{}","version":"0.1.0","layer":"a","state":"active","description":"test active cached procedure","friction_source":{{"since":"all","session_count":1,"eval_run_count":1,"dominant_signal":"high-token-use","turns":2,"tool_calls":1,"tokens":5000,"repeated_errors":0,"retracements":0,"avoidable_prompts":0,"missing_tool_failures":0}},"payload":{{"kind":"cached_procedure","title":"Reduce high token use","body":"Read small files first.","prompt_budget_chars":1200}},"validation":null,"lineage":{{"parent_id":null,"reason":"test fixture"}},"rollback":null,"created_at_unix_ms":1,"applied_at_unix_ms":2,"reverted_at_unix_ms":null}}"#,
                id
            ),
        )
        .unwrap();
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "greco-loop-{label}-{}-{}",
        std::process::id(),
        now_millis()
    ))
}
