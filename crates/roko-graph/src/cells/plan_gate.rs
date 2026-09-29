//! Plan-topology gate Cell.
//!
//! `PlanGateCell` is the real "plan.gate" cell that replaces the
//! `PassthroughCell` stub in `ProductionPlanTopology`. It delegates to the
//! `SharedGateEvaluator` available through `CellContext.resources.gates`,
//! running each canonical rung (compile, lint, test) in order and
//! producing a `Kind::GateVerdict` Signal whose body is a serialized
//! `GateResult`.
//!
//! The cell gates the checkout of the exact attempt the task's executor
//! produced, named by the [`TaskAttempt`] stamped on the executor's output,
//! and sends that attempt's ordinal and key with every request. It fails
//! closed, with an error rather than a pass, when:
//!
//! - the `SharedGateEvaluator` is not injected (`resources.gates` is `None`);
//! - the input names no attempt, or an attempt with no isolated checkout: the
//!   only other tree is the operator's own, which a gate never judges;
//! - the checkout it names does not exist.
//!
//! A pipeline in which no rung ran (every rung skipped) fails.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use roko_core::{
    Body, GateResult, Kind, ProtocolId, RokoError, RungResult, SharedGateError, SharedGateRequest,
    Signal, error::Result,
};
use tracing::{info, warn};

use crate::cell::{Cell, CellContext, CellVersion};
use crate::cells::task_executor::TaskAttempt;

/// Canonical gate rungs executed in order.
///
/// These match the production gate pipeline's core rungs. Additional rungs
/// (diff, fmt, shell, judge) are available in the full `GatePipelineCell`
/// from `roko-gate`; this cell covers the three rungs that every plan task
/// must pass.
const CANONICAL_RUNGS: &[&str] = &["compile", "lint", "test"];

/// Output tag explaining a verdict that no rung produced.
const GATE_EVIDENCE_TAG: &str = "gate.evidence";

/// Plan-topology gate Cell.
///
/// Reads task metadata from its TOML node config and runs the canonical
/// gate rungs via `SharedGateEvaluator`. Produces a `Kind::GateVerdict`
/// Signal carrying a serialized `GateResult`.
pub struct PlanGateCell {
    /// Task identifier within the plan.
    task_id: String,
    /// Plan identifier.
    plan_id: String,
    /// Source plan directory.
    plan_dir: String,
    /// Task title, sent as gate context.
    title: String,
    /// Files expected in scope (for `changed_files` in gate requests).
    files: Vec<String>,
}

impl PlanGateCell {
    /// Construct from a TOML node config.
    ///
    /// Expected keys: `task_id`, `plan_id`, `plan_dir`, `title`, `files`.
    #[must_use]
    pub fn from_config(config: &toml::Value) -> Self {
        let table = config.as_table();
        let string = |key: &str| {
            table
                .and_then(|t| t.get(key))
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let files = table
            .and_then(|t| t.get("files"))
            .and_then(toml::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(toml::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        Self {
            task_id: string("task_id"),
            plan_id: string("plan_id"),
            plan_dir: string("plan_dir"),
            title: string("title"),
            files,
        }
    }

    /// Build a `SharedGateRequest` for one rung of `attempt`, run in
    /// `worktree`, the attempt's own checkout. The context carries the plan,
    /// the run, the attempt's key and the task title.
    fn build_request(
        &self,
        rung: &str,
        attempt: &TaskAttempt,
        worktree: &Path,
        run_id: Option<&str>,
    ) -> SharedGateRequest {
        let mut context = HashMap::from([("plan_id".to_owned(), self.plan_id.clone())]);
        let optional = [
            ("run_id", run_id),
            ("attempt_key", attempt.attempt_key.as_deref()),
            (
                "title",
                Some(self.title.as_str()).filter(|title| !title.is_empty()),
            ),
        ];
        for (key, value) in optional {
            if let Some(value) = value {
                context.insert(key.to_owned(), value.to_owned());
            }
        }
        SharedGateRequest {
            task_id: self.task_id.clone(),
            attempt_id: attempt.attempt,
            rung: rung.to_owned(),
            plan_dir: self.plan_dir.clone(),
            worktree_path: worktree.to_path_buf(),
            changed_files: self.files.clone(),
            context,
        }
    }

    /// The attempt named by the executor's output and its checkout. Fails
    /// closed: an unnamed attempt, an attempt that ran in the shared working
    /// tree, or a checkout that no longer exists is an error, never a
    /// fallback to some other tree.
    fn attempt_checkout(&self, input: &[Signal]) -> Result<(TaskAttempt, PathBuf)> {
        let attempt = TaskAttempt::from_signals(input).map_err(|reason| {
            RokoError::Invalid(format!(
                "PlanGateCell: task `{}`: {reason}; refusing to gate an unknown attempt",
                self.task_id
            ))
        })?;
        let Some(worktree) = attempt.workspace.clone() else {
            return Err(RokoError::Invalid(format!(
                "PlanGateCell: attempt {} of task `{}` ran in no isolated worktree; the plan \
                 gate judges an attempt's own checkout, never the operator's working tree \
                 (run the plan with --worktree-per-task)",
                attempt.attempt, self.task_id
            )));
        };
        if !worktree.is_dir() {
            return Err(RokoError::Invalid(format!(
                "PlanGateCell: the worktree of attempt {} of task `{}` does not exist: {}",
                attempt.attempt,
                self.task_id,
                worktree.display()
            )));
        }
        Ok((attempt, worktree))
    }

    /// Convert a `SharedGateError` into a failed `RungResult` with diagnostic
    /// evidence rather than propagating the error. This keeps the gate
    /// pipeline running for subsequent rungs (fail-open per rung, fail-closed
    /// at aggregate level).
    fn error_to_rung_result(rung: &str, err: &SharedGateError) -> RungResult {
        RungResult {
            rung_name: rung.to_owned(),
            passed: false,
            score: 0.0,
            evidence: Some(err.to_string()),
        }
    }
}

#[async_trait]
impl Cell for PlanGateCell {
    fn cell_id(&self) -> &'static str {
        "plan.gate"
    }

    fn cell_name(&self) -> &'static str {
        "PlanGateCell"
    }

    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }

    fn is_stub(&self) -> bool {
        false
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Verify]
    }

    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0)
    }

    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_mins(2))
    }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        let evaluator = ctx.resources.gates.as_ref().ok_or_else(|| {
            RokoError::Invalid(
                "PlanGateCell: SharedGateEvaluator not injected (ctx.resources.gates is None); \
                 refusing to silently pass"
                    .to_owned(),
            )
        })?;

        let (attempt, worktree) = self.attempt_checkout(&input)?;
        let worktree = worktree.as_path();
        let run_id = attempt.run_id.as_deref().or(ctx.run_id.as_deref());

        info!(
            task_id = %self.task_id,
            plan_id = %self.plan_id,
            attempt = attempt.attempt,
            worktree = %worktree.display(),
            rungs = ?CANONICAL_RUNGS,
            "PlanGateCell: starting gate pipeline"
        );

        let mut rung_results = Vec::with_capacity(CANONICAL_RUNGS.len());
        let mut all_passed = true;

        for &rung in CANONICAL_RUNGS {
            let request = self.build_request(rung, &attempt, worktree, run_id);

            match evaluator.verify_rung(&request).await {
                Ok(verdict) => {
                    if verdict.skipped {
                        info!(rung, "PlanGateCell: rung skipped");
                        // Skipped rungs do not count as pass or fail.
                        continue;
                    }

                    if !verdict.passed {
                        all_passed = false;
                        warn!(
                            rung,
                            reasons = ?verdict.failed_reasons,
                            "PlanGateCell: rung failed"
                        );
                    } else {
                        info!(rung, "PlanGateCell: rung passed");
                    }

                    rung_results.push(RungResult {
                        rung_name: verdict.rung,
                        passed: verdict.passed,
                        score: if verdict.passed { 1.0 } else { 0.0 },
                        evidence: verdict.evidence,
                    });
                }
                Err(err) => {
                    all_passed = false;
                    warn!(rung, error = %err, "PlanGateCell: rung evaluation error");
                    rung_results.push(Self::error_to_rung_result(rung, &err));
                }
            }
        }

        // A gate that checked nothing passes nothing: with every rung skipped
        // the attempt fails, with the reason as evidence.
        let nothing_ran = rung_results.is_empty();
        if nothing_ran {
            all_passed = false;
            warn!(
                task_id = %self.task_id,
                attempt = attempt.attempt,
                "PlanGateCell: no gate rung ran; failing closed"
            );
        }
        let total = rung_results.len() as f64;
        let passed_count = rung_results.iter().filter(|r| r.passed).count() as f64;
        let overall_score = if nothing_ran {
            0.0
        } else {
            passed_count / total
        };

        let gate_result = GateResult {
            passed: all_passed,
            rung_results,
            overall_score,
        };

        info!(
            task_id = %self.task_id,
            passed = gate_result.passed,
            score = gate_result.overall_score,
            "PlanGateCell: gate pipeline complete"
        );

        let body = Body::from_json(&gate_result).map_err(|e| {
            RokoError::Invalid(format!("PlanGateCell: failed to serialize GateResult: {e}"))
        })?;

        let mut output = Signal::builder(Kind::GateVerdict).body(body).build();

        // Propagate tags from the first input signal so downstream cells
        // retain task context (e.g. task_id, plan_id).
        if let Some(first) = input.first() {
            for (k, v) in &first.tags {
                output.tags.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        // Tag with gate outcome for conditional edges.
        output
            .tags
            .insert("gate.passed".to_owned(), all_passed.to_string());
        if nothing_ran {
            output.tags.insert(
                GATE_EVIDENCE_TAG.to_owned(),
                format!(
                    "no gate rung ran: all {} rungs were skipped",
                    CANONICAL_RUNGS.len()
                ),
            );
        }
        output.id = output.content_hash();

        Ok(vec![output])
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use roko_core::{SharedGateError, SharedGateEvaluator, SharedGateRequest, SharedGateVerdict};

    use super::*;
    use crate::cell::CellResources;

    // ── Mock evaluators ─────────────────────────────────────────────────────

    struct AllPassEvaluator;

    #[async_trait]
    impl SharedGateEvaluator for AllPassEvaluator {
        async fn verify_rung(
            &self,
            request: &SharedGateRequest,
        ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
            Ok(SharedGateVerdict::pass(&request.rung))
        }
    }

    struct FailTestEvaluator;

    #[async_trait]
    impl SharedGateEvaluator for FailTestEvaluator {
        async fn verify_rung(
            &self,
            request: &SharedGateRequest,
        ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
            if request.rung == "test" {
                Ok(SharedGateVerdict::fail(
                    "test",
                    vec!["3 test failures".into()],
                ))
            } else {
                Ok(SharedGateVerdict::pass(&request.rung))
            }
        }
    }

    struct ErrorEvaluator;

    #[async_trait]
    impl SharedGateEvaluator for ErrorEvaluator {
        async fn verify_rung(
            &self,
            _request: &SharedGateRequest,
        ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
            Err(SharedGateError::Internal {
                reason: "evaluator crash".into(),
            })
        }
    }

    struct SkipAllEvaluator;

    #[async_trait]
    impl SharedGateEvaluator for SkipAllEvaluator {
        async fn verify_rung(
            &self,
            request: &SharedGateRequest,
        ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
            Ok(SharedGateVerdict::skip(&request.rung))
        }
    }

    /// Records every request it is asked to evaluate, and passes it.
    #[derive(Clone, Default)]
    struct RecordingEvaluator {
        requests: Arc<parking_lot::Mutex<Vec<SharedGateRequest>>>,
    }

    #[async_trait]
    impl SharedGateEvaluator for RecordingEvaluator {
        async fn verify_rung(
            &self,
            request: &SharedGateRequest,
        ) -> std::result::Result<SharedGateVerdict, SharedGateError> {
            self.requests.lock().push(request.clone());
            Ok(SharedGateVerdict::pass(&request.rung))
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn make_config() -> toml::Value {
        let mut table = toml::map::Map::new();
        table.insert("task_id".into(), toml::Value::String("task-1".into()));
        table.insert("plan_id".into(), toml::Value::String("plan-1".into()));
        table.insert("plan_dir".into(), toml::Value::String("plans/test".into()));
        table.insert(
            "title".into(),
            toml::Value::String("Add the feature".into()),
        );
        table.insert(
            "files".into(),
            toml::Value::Array(vec![toml::Value::String("src/lib.rs".into())]),
        );
        toml::Value::Table(table)
    }

    fn make_cell() -> PlanGateCell {
        PlanGateCell::from_config(&make_config())
    }

    fn ctx_with_gates(evaluator: impl SharedGateEvaluator) -> CellContext {
        CellContext::new().with_resources(CellResources {
            gates: Some(Arc::new(evaluator)),
        })
    }

    /// Attempt 2 of `task-1`, run in `worktree`.
    fn attempt_in(worktree: Option<&Path>) -> TaskAttempt {
        TaskAttempt {
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            run_id: Some("run-1".to_string()),
            attempt_key: Some("run-1:plan-1:task-1:2".to_string()),
            attempt: 2,
            workspace: worktree.map(Path::to_path_buf),
            lease: None,
        }
    }

    /// The executor's output for `attempt`.
    fn executor_output(attempt: &TaskAttempt) -> Vec<Signal> {
        let mut output = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("task output"))
                .build(),
        ];
        attempt.stamp(&mut output);
        output
    }

    /// An attempt checkout on disk, and the executor output naming it.
    fn gated_attempt() -> (tempfile::TempDir, Vec<Signal>) {
        let worktree = tempfile::tempdir().expect("worktree");
        let input = executor_output(&attempt_in(Some(worktree.path())));
        (worktree, input)
    }

    fn decode_gate_result(signal: &Signal) -> GateResult {
        signal
            .body
            .as_json::<GateResult>()
            .expect("decode GateResult")
    }

    // ── Tests ───────────────────────────────────────────────────────────────

    #[test]
    fn from_config_parses_fields() {
        let cell = make_cell();
        assert_eq!(cell.task_id, "task-1");
        assert_eq!(cell.plan_id, "plan-1");
        assert_eq!(cell.plan_dir, "plans/test");
        assert_eq!(cell.title, "Add the feature");
        assert_eq!(cell.files, vec!["src/lib.rs"]);
    }

    #[test]
    fn cell_metadata() {
        let cell = make_cell();
        assert_eq!(cell.cell_id(), "plan.gate");
        assert_eq!(cell.cell_name(), "PlanGateCell");
        assert!(!cell.is_stub());
        assert_eq!(cell.protocols(), vec![ProtocolId::Verify]);
    }

    #[tokio::test]
    async fn all_rungs_pass() {
        let cell = make_cell();
        let ctx = ctx_with_gates(AllPassEvaluator);
        let (_worktree, input) = gated_attempt();
        let output = cell.execute(input, &ctx).await.unwrap();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0].kind, Kind::GateVerdict);

        let result = decode_gate_result(&output[0]);
        assert!(result.passed);
        assert_eq!(result.rung_results.len(), 3);
        assert_eq!(result.overall_score, 1.0);
        assert!(result.rung_results.iter().all(|r| r.passed));

        assert_eq!(
            output[0].tags.get("gate.passed").map(String::as_str),
            Some("true")
        );
        // The verdict keeps naming the attempt it judged.
        assert_eq!(output[0].tag("workspace.attempt"), Some("2"));
    }

    #[tokio::test]
    async fn test_rung_fails() {
        let cell = make_cell();
        let ctx = ctx_with_gates(FailTestEvaluator);
        let (_worktree, input) = gated_attempt();
        let output = cell.execute(input, &ctx).await.unwrap();

        let result = decode_gate_result(&output[0]);
        assert!(!result.passed);
        assert_eq!(result.failed_rung_names(), vec!["test"]);
        assert_eq!(result.passed_count(), 2);
        assert_eq!(result.failed_count(), 1);

        assert_eq!(
            output[0].tags.get("gate.passed").map(String::as_str),
            Some("false")
        );
    }

    #[tokio::test]
    async fn evaluator_error_produces_failed_rung() {
        let cell = make_cell();
        let ctx = ctx_with_gates(ErrorEvaluator);
        let (_worktree, input) = gated_attempt();
        let output = cell.execute(input, &ctx).await.unwrap();

        let result = decode_gate_result(&output[0]);
        assert!(!result.passed);
        // All 3 rungs should have error evidence.
        assert_eq!(result.rung_results.len(), 3);
        assert!(result.rung_results.iter().all(|r| !r.passed));
        assert!(result.rung_results.iter().all(|r| {
            r.evidence
                .as_deref()
                .unwrap_or("")
                .contains("evaluator crash")
        }));
    }

    #[tokio::test]
    async fn no_evaluator_fails_closed() {
        let cell = make_cell();
        let ctx = CellContext::new(); // no gates injected
        let (_worktree, input) = gated_attempt();
        let result = cell.execute(input, &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("SharedGateEvaluator not injected"));
    }

    /// bug-50caf2: a gate that checked nothing passes nothing.
    #[tokio::test]
    async fn all_skipped_fails_closed() {
        let cell = make_cell();
        let ctx = ctx_with_gates(SkipAllEvaluator);
        let (_worktree, input) = gated_attempt();
        let output = cell.execute(input, &ctx).await.unwrap();

        let result = decode_gate_result(&output[0]);
        assert!(!result.passed);
        assert!(result.rung_results.is_empty()); // skipped rungs excluded
        assert_eq!(result.overall_score, 0.0);
        assert_eq!(
            output[0].tags.get("gate.passed").map(String::as_str),
            Some("false")
        );
        let evidence = output[0].tag("gate.evidence").unwrap_or_default();
        assert!(evidence.contains("no gate rung ran"), "{evidence}");
    }

    /// bug-50caf2: every rung runs in the checkout the executor named, as
    /// that attempt, with the plan, run, attempt key and title as context.
    #[tokio::test]
    async fn plan_gate_uses_worktree_from_executor_output() {
        let cell = make_cell();
        let recorder = RecordingEvaluator::default();
        let ctx = ctx_with_gates(recorder.clone());
        let (worktree, input) = gated_attempt();
        let output = cell.execute(input, &ctx).await.unwrap();
        assert!(decode_gate_result(&output[0]).passed);

        let requests = recorder.requests.lock();
        let rungs: Vec<&str> = requests.iter().map(|r| r.rung.as_str()).collect();
        assert_eq!(rungs, CANONICAL_RUNGS);
        for request in requests.iter() {
            assert_eq!(request.worktree_path, worktree.path());
            assert_eq!(request.attempt_id, 2);
            assert_eq!(request.task_id, "task-1");
            let context = |key: &str| request.context.get(key).map(String::as_str);
            assert_eq!(context("plan_id"), Some("plan-1"));
            assert_eq!(context("run_id"), Some("run-1"));
            assert_eq!(context("attempt_key"), Some("run-1:plan-1:task-1:2"));
            assert_eq!(context("title"), Some("Add the feature"));
        }
    }

    /// bug-50caf2: without an isolated checkout of a named attempt the gate
    /// errors, and never falls back to another tree: not the process's
    /// working directory, not the shared (operator's) working tree, not a
    /// checkout that is gone.
    #[tokio::test]
    async fn plan_gate_fails_closed_without_worktree() {
        let cell = make_cell();
        let recorder = RecordingEvaluator::default();
        let ctx = ctx_with_gates(recorder.clone());
        let gone = tempfile::tempdir().expect("tempdir").path().join("removed");
        let cases = [
            (input_without_attempt(), "workspace.attempt"),
            (executor_output(&attempt_in(None)), "no isolated worktree"),
            (executor_output(&attempt_in(Some(&gone))), "does not exist"),
        ];
        for (input, reason) in cases {
            let error = cell.execute(input, &ctx).await.unwrap_err().to_string();
            assert!(error.contains(reason), "{reason}: {error}");
        }
        assert!(
            recorder.requests.lock().is_empty(),
            "no rung may run without the attempt's checkout"
        );
    }

    fn input_without_attempt() -> Vec<Signal> {
        vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("task output"))
                .build(),
        ]
    }

    #[test]
    fn build_request_populates_fields() {
        let cell = make_cell();
        let attempt = attempt_in(Some(Path::new("/workspace")));
        let req = cell.build_request("compile", &attempt, Path::new("/workspace"), None);

        assert_eq!(req.task_id, "task-1");
        assert_eq!(req.attempt_id, 2);
        assert_eq!(req.rung, "compile");
        assert_eq!(req.plan_dir, "plans/test");
        assert_eq!(req.worktree_path, PathBuf::from("/workspace"));
        assert_eq!(req.changed_files, vec!["src/lib.rs"]);
        assert_eq!(
            req.context.get("plan_id").map(String::as_str),
            Some("plan-1")
        );
        assert!(!req.context.contains_key("run_id"));
    }
}
