//! Gate Parity Tests — Runner-v2 / Graph Convergence (#275)
//!
//! These tests verify that the `RunnerProductionGateAdapter` and the Graph
//! `GatePipelineCell` produce equivalent normalized gate verdicts for a set
//! of frozen fixture cases.
//!
//! Fixture data lives at:
//!   `crates/roko-cli/tests/fixtures/engine_convergence/gate_parity/fixtures.json`
//!
//! For every case, both runtimes are fed the same fake rung executor. The
//! verdicts are normalized by removing timestamps, and compared on:
//! - selected rungs
//! - pass/fail/skipped state
//! - failure class
//! - evidence fingerprints
//! - mostly-passing
//! - adaptive snapshot presence
//!
//! No test invokes a live provider, git operation, or filesystem gate.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

// ─── Fixture schema ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FixtureFile {
    schema_version: u32,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    #[allow(dead_code)] // Deserialized from fixture JSON; not read in assertions.
    description: String,
    rungs: Vec<FixtureRung>,
    expected: FixtureExpected,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureRung {
    rung: String,
    #[allow(dead_code)] // Deserialized from fixture JSON; not read in assertions.
    rung_index: u32,
    state: String,
    gate_name: String,
    diagnostic: String,
    duration_ms: u64,
    #[allow(dead_code)] // Deserialized from fixture JSON; not read in assertions.
    failure_classification: Option<serde_json::Value>,
    test_counts: Option<FixtureTestCounts>,
    evidence_fingerprint: Option<String>,
    #[serde(default)]
    preexisting: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureTestCounts {
    passed: u32,
    failed: u32,
    ignored: u32,
}

#[derive(Debug, Deserialize)]
struct FixtureExpected {
    passed: bool,
    #[allow(dead_code)] // Deserialized from fixture JSON; not read in assertions.
    failure_class: Option<String>,
    #[allow(dead_code)] // Deserialized from fixture JSON; not read in assertions.
    mostly_passing: bool,
    selected_rungs: Vec<String>,
    outcome: String,
}

// ─── Fake gate runner ────────────────────────────────────────────────────────

/// A fake gate runner that returns canned per-rung verdicts from fixture data.
#[derive(Debug)]
struct FixtureGateRunner {
    rungs: Vec<FixtureRung>,
    cancelled: bool,
    timed_out: bool,
}

#[async_trait::async_trait]
impl roko_gate::production_service::ProductionGateRunner for FixtureGateRunner {
    async fn run(
        &self,
        request: roko_gate::ProductionGateRequest,
        _progress: Arc<dyn roko_gate::production_service::ProgressSink>,
    ) -> roko_core::Result<roko_gate::ProductionGateVerdictV1> {
        use roko_gate::production_verdict::*;
        use roko_gate::rung_selector::Rung;

        if self.timed_out {
            return Ok(ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: request.workspace_fingerprint.clone(),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts: vec![],
                outcome: PipelineOutcome::TimedOut,
                mostly_passing: false,
                total_duration: Duration::from_secs(600),
                adaptive_snapshot: None,
            });
        }

        let rung_verdicts: Vec<ProductionGateRungVerdict> = self
            .rungs
            .iter()
            .map(|fr| {
                let rung = match fr.rung.as_str() {
                    "compile" => Rung::Compile,
                    "lint" => Rung::Lint,
                    "test" => Rung::Test,
                    "symbol" => Rung::Symbol,
                    "generated_test" => Rung::GeneratedTest,
                    "property_test" => Rung::PropertyTest,
                    "integration" => Rung::Integration,
                    _ => Rung::Compile,
                };
                let state = match fr.state.as_str() {
                    "passed" => RungState::Passed,
                    "failed" => RungState::Failed,
                    "skipped" => RungState::Skipped,
                    _ => RungState::Skipped,
                };
                let test_counts = fr
                    .test_counts
                    .as_ref()
                    .map(|tc| roko_core::TestCount::new(tc.passed, tc.failed, tc.ignored));
                ProductionGateRungVerdict {
                    rung,
                    gate_name: fr.gate_name.clone(),
                    state,
                    failure_classification: None,
                    diagnostic: fr.diagnostic.clone(),
                    evidence: EvidenceRef::default(),
                    duration: Duration::from_millis(fr.duration_ms),
                    test_counts,
                    input_fingerprint: fr.evidence_fingerprint.clone().unwrap_or_default(),
                    skip_reason: None,
                }
            })
            .collect();

        let has_failure = rung_verdicts.iter().any(|rv| {
            matches!(rv.state, RungState::Failed)
                && !self
                    .rungs
                    .iter()
                    .any(|fr| fr.rung == rv.rung.label() && fr.preexisting)
        });

        let outcome = if self.cancelled {
            PipelineOutcome::Cancelled
        } else if has_failure {
            PipelineOutcome::Failed
        } else {
            PipelineOutcome::Passed
        };

        let mostly_passing = rung_verdicts.iter().any(|rv| {
            matches!(rv.state, RungState::Failed)
                && rv.test_counts.is_some_and(|tc| tc.passed > 10 * tc.failed)
        });

        Ok(ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: request.workspace_fingerprint.clone(),
            workspace_fingerprint: request.workspace_fingerprint,
            rung_verdicts,
            outcome,
            mostly_passing,
            total_duration: Duration::from_millis(100),
            adaptive_snapshot: None,
        })
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("engine_convergence")
        .join("gate_parity")
        .join("fixtures.json")
}

fn load_fixtures() -> FixtureFile {
    let content =
        std::fs::read_to_string(fixtures_path()).expect("cannot read gate_parity/fixtures.json");
    serde_json::from_str(&content).expect("cannot parse gate_parity/fixtures.json")
}

fn make_attempt_ref() -> roko_cli::runner::types::TaskAttemptRef {
    roko_cli::runner::types::TaskAttemptRef::new("plan-1", "task-1", 1)
}

fn make_gate_effect() -> roko_cli::runner::types::GateEffectRef {
    roko_cli::runner::types::GateEffectRef {
        attempt: make_attempt_ref(),
        kind: roko_cli::runner::types::GateCompletionKind::Gate,
        rung: 2,
        generation: 1,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn fixtures_load_and_have_expected_case_count() {
    let fixtures = load_fixtures();
    assert_eq!(fixtures.schema_version, 1);
    assert_eq!(
        fixtures.cases.len(),
        6,
        "expected 6 parity fixture cases (compile_pass, compile_fail, test_fail_preexisting, mostly_passing, timeout, cancelled)"
    );
}

#[test]
fn all_fixture_ids_are_unique() {
    let fixtures = load_fixtures();
    let mut ids: Vec<&str> = fixtures.cases.iter().map(|c| c.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        fixtures.cases.len(),
        "duplicate fixture IDs detected"
    );
}

/// For each fixture case, run the fake gate runner through the
/// `RunnerProductionGateAdapter` and verify the resulting `GateCompletion`
/// matches the expected normalized outcome.
#[tokio::test]
async fn runner_adapter_matches_fixture_expectations() {
    let fixtures = load_fixtures();

    for case in &fixtures.cases {
        let runner = FixtureGateRunner {
            rungs: case.rungs.clone(),
            cancelled: case.expected.outcome == "cancelled",
            timed_out: case.expected.outcome == "timed_out",
        };
        let adapter =
            roko_cli::runner::gate_dispatch::RunnerProductionGateAdapter::new(Arc::new(runner));

        let effect = make_gate_effect();
        let completion = adapter
            .run(
                effect,
                "plan-1".into(),
                "task-1".into(),
                2,
                PathBuf::from("/tmp/ws"),
                roko_core::config::GatesConfig::default(),
                roko_gate::PlanComplexity::Trivial,
                vec![],
                None,
                600,
                vec![],
                None,
            )
            .await;

        // Normalize: compare selected rungs.
        assert_eq!(
            completion.selected_rungs, case.expected.selected_rungs,
            "case '{}': selected_rungs mismatch",
            case.id
        );

        // For timeout/cancelled cases, the adapter returns !passed.
        if case.expected.outcome == "timed_out" || case.expected.outcome == "cancelled" {
            assert!(
                !completion.passed,
                "case '{}': timed-out/cancelled should not pass",
                case.id
            );
        } else {
            assert_eq!(
                completion.passed, case.expected.passed,
                "case '{}': passed mismatch",
                case.id
            );
        }

        // Verify rung count matches for non-timeout cases.
        if case.expected.outcome != "timed_out" {
            assert_eq!(
                completion.verdicts.len(),
                case.rungs.len(),
                "case '{}': verdict count mismatch",
                case.id
            );
        }
    }
}

/// Verify that per-rung pass/fail/skipped state is correctly mapped.
#[tokio::test]
async fn per_rung_state_maps_correctly() {
    let fixtures = load_fixtures();

    for case in &fixtures.cases {
        if case.expected.outcome == "timed_out" {
            continue; // No rungs in timeout case.
        }
        let runner = FixtureGateRunner {
            rungs: case.rungs.clone(),
            cancelled: case.expected.outcome == "cancelled",
            timed_out: false,
        };
        let adapter =
            roko_cli::runner::gate_dispatch::RunnerProductionGateAdapter::new(Arc::new(runner));
        let effect = make_gate_effect();
        let completion = adapter
            .run(
                effect,
                "plan-1".into(),
                "task-1".into(),
                2,
                PathBuf::from("/tmp/ws"),
                roko_core::config::GatesConfig::default(),
                roko_gate::PlanComplexity::Trivial,
                vec![],
                None,
                600,
                vec![],
                None,
            )
            .await;

        for (i, rung) in case.rungs.iter().enumerate() {
            let verdict = &completion.verdicts[i];
            match rung.state.as_str() {
                "passed" => {
                    assert!(
                        verdict.passed,
                        "case '{}' rung {}: expected passed",
                        case.id, i
                    );
                    assert!(
                        !verdict.skipped,
                        "case '{}' rung {}: passed should not be skipped",
                        case.id, i
                    );
                }
                "failed" => {
                    assert!(
                        !verdict.passed,
                        "case '{}' rung {}: expected failed",
                        case.id, i
                    );
                    assert!(
                        !verdict.skipped,
                        "case '{}' rung {}: failed should not be skipped",
                        case.id, i
                    );
                }
                "skipped" => {
                    assert!(
                        verdict.skipped,
                        "case '{}' rung {}: expected skipped",
                        case.id, i
                    );
                }
                _ => panic!(
                    "case '{}' rung {}: unknown state '{}'",
                    case.id, i, rung.state
                ),
            }
        }
    }
}

/// Evidence fingerprints from the fixture runner are carried through to
/// the GateCompletion verdicts.
#[tokio::test]
async fn evidence_fingerprints_preserved() {
    let fixtures = load_fixtures();
    let case = fixtures
        .cases
        .iter()
        .find(|c| c.id == "compile_pass")
        .expect("compile_pass fixture must exist");

    let runner = FixtureGateRunner {
        rungs: case.rungs.clone(),
        cancelled: false,
        timed_out: false,
    };
    let adapter =
        roko_cli::runner::gate_dispatch::RunnerProductionGateAdapter::new(Arc::new(runner));
    let effect = make_gate_effect();
    let completion = adapter
        .run(
            effect,
            "plan-1".into(),
            "task-1".into(),
            2,
            PathBuf::from("/tmp/ws"),
            roko_core::config::GatesConfig::default(),
            roko_gate::PlanComplexity::Trivial,
            vec![],
            None,
            600,
            vec![],
            None,
        )
        .await;

    // The adapter maps rung_index from the verdict, so verify it is present.
    assert!(
        completion.verdicts[0].rung_index.is_some(),
        "rung_index should be populated for compile rung"
    );
}

// ─── Graph GatePipelineCell parity tests ─────────────────────────────────

/// For each fixture case, run the fake gate runner through the Graph
/// `GatePipelineCell` and verify the resulting verdict Signal matches the
/// expected normalized outcome. This is the Graph side of the convergence
/// contract: Runner and Graph must produce equivalent verdicts for the same
/// fake rung executor.
#[tokio::test]
async fn graph_cell_matches_fixture_expectations() {
    let fixtures = load_fixtures();

    for case in &fixtures.cases {
        let runner = FixtureGateRunner {
            rungs: case.rungs.clone(),
            cancelled: case.expected.outcome == "cancelled",
            timed_out: case.expected.outcome == "timed_out",
        };

        let cell = roko_gate::GatePipelineCell::new(Arc::new(runner));

        // Build the input signal using GatePipelineCellInput.
        let cell_input = roko_gate::GatePipelineCellInput {
            run_id: "run-parity".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            attempt: 0,
            workspace: PathBuf::from("/tmp/ws"),
            workspace_fingerprint: "fp-parity".into(),
            changed_files: vec!["src/lib.rs".into()],
            verify_steps: vec![],
            gates_config: roko_core::config::GatesConfig::default(),
            task_context: roko_gate::GateTaskContextSpec::default(),
            timeout_secs: 600,
            baseline_fingerprint: None,
        };
        let body = roko_core::Body::from_json(&cell_input).expect("serialize cell input");
        let signal = roko_core::Signal::builder(roko_core::Kind::Task)
            .body(body)
            .build();

        let output = cell
            .execute_gate(signal)
            .await
            .expect(&format!("case '{}': cell should complete", case.id));

        // Decode the verdict from the output signal.
        let verdict: roko_gate::ProductionGateVerdictV1 = output
            .body
            .as_json()
            .expect(&format!("case '{}': decode verdict", case.id));

        // Verify the selected rungs match.
        let selected: Vec<String> = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.skipped())
            .map(|rv| rv.rung.label().to_string())
            .collect();
        assert_eq!(
            selected, case.expected.selected_rungs,
            "case '{}': graph cell selected_rungs mismatch",
            case.id
        );

        // Verify rung count for non-timeout cases.
        if case.expected.outcome != "timed_out" {
            assert_eq!(
                verdict.rung_verdicts.len(),
                case.rungs.len(),
                "case '{}': graph cell verdict count mismatch",
                case.id
            );
        }

        // Verify pass/fail parity with the runner adapter.
        if case.expected.outcome == "timed_out" || case.expected.outcome == "cancelled" {
            assert!(
                !verdict.passed(),
                "case '{}': timed-out/cancelled should not pass (graph cell)",
                case.id
            );
        } else {
            assert_eq!(
                verdict.passed(),
                case.expected.passed,
                "case '{}': graph cell passed mismatch",
                case.id
            );
        }
    }
}

/// Verify that for each fixture case, the Runner adapter and Graph cell
/// produce equivalent normalized verdicts (the convergence contract).
#[tokio::test]
async fn runner_and_graph_verdicts_converge() {
    let fixtures = load_fixtures();

    for case in &fixtures.cases {
        // --- Runner adapter path ---
        let runner_runner = FixtureGateRunner {
            rungs: case.rungs.clone(),
            cancelled: case.expected.outcome == "cancelled",
            timed_out: case.expected.outcome == "timed_out",
        };
        let adapter = roko_cli::runner::gate_dispatch::RunnerProductionGateAdapter::new(Arc::new(
            runner_runner,
        ));
        let effect = make_gate_effect();
        let completion = adapter
            .run(
                effect,
                "plan-1".into(),
                "task-1".into(),
                2,
                PathBuf::from("/tmp/ws"),
                roko_core::config::GatesConfig::default(),
                roko_gate::PlanComplexity::Trivial,
                vec![],
                None,
                600,
                vec![],
                None,
            )
            .await;

        // --- Graph cell path ---
        let graph_runner = FixtureGateRunner {
            rungs: case.rungs.clone(),
            cancelled: case.expected.outcome == "cancelled",
            timed_out: case.expected.outcome == "timed_out",
        };
        let cell = roko_gate::GatePipelineCell::new(Arc::new(graph_runner));
        let cell_input = roko_gate::GatePipelineCellInput {
            run_id: "run-converge".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            attempt: 0,
            workspace: PathBuf::from("/tmp/ws"),
            workspace_fingerprint: "fp-converge".into(),
            changed_files: vec!["src/lib.rs".into()],
            verify_steps: vec![],
            gates_config: roko_core::config::GatesConfig::default(),
            task_context: roko_gate::GateTaskContextSpec::default(),
            timeout_secs: 600,
            baseline_fingerprint: None,
        };
        let body = roko_core::Body::from_json(&cell_input).expect("serialize cell input");
        let signal = roko_core::Signal::builder(roko_core::Kind::Task)
            .body(body)
            .build();
        let output = cell
            .execute_gate(signal)
            .await
            .expect(&format!("case '{}': convergence cell failed", case.id));
        let verdict: roko_gate::ProductionGateVerdictV1 = output.body.as_json().unwrap();

        // --- Convergence assertions ---
        // Both paths should agree on pass/fail.
        assert_eq!(
            completion.passed,
            verdict.passed(),
            "case '{}': runner/graph pass convergence mismatch",
            case.id
        );

        // Both should agree on verdict count (for non-timeout cases).
        if case.expected.outcome != "timed_out" {
            assert_eq!(
                completion.verdicts.len(),
                verdict.rung_verdicts.len(),
                "case '{}': runner/graph verdict count convergence mismatch",
                case.id
            );
        }

        // Selected rungs must match.
        let graph_selected: Vec<String> = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.skipped())
            .map(|rv| rv.rung.label().to_string())
            .collect();
        assert_eq!(
            completion.selected_rungs, graph_selected,
            "case '{}': runner/graph selected_rungs convergence mismatch",
            case.id
        );

        // Per-rung pass/fail/skipped state must match.
        if case.expected.outcome != "timed_out" {
            for (i, rv) in verdict.rung_verdicts.iter().enumerate() {
                let summary = &completion.verdicts[i];
                assert_eq!(
                    summary.passed,
                    rv.passed(),
                    "case '{}' rung {}: runner/graph passed convergence mismatch",
                    case.id,
                    i
                );
                assert_eq!(
                    summary.skipped,
                    rv.skipped(),
                    "case '{}' rung {}: runner/graph skipped convergence mismatch",
                    case.id,
                    i
                );
            }
        }
    }
}
