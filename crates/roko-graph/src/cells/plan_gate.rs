//! Plan-topology gate Cell.
//!
//! `PlanGateCell` is the real "plan.gate" cell that replaces the
//! `PassthroughCell` stub in `ProductionPlanTopology`. It delegates to the
//! `SharedGateEvaluator` available through `CellContext.resources.gates`,
//! running each canonical rung (compile, lint, test) in order and
//! producing a `Kind::GateVerdict` Signal whose body is a serialized
//! `GateResult`.
//!
//! If the `SharedGateEvaluator` is not injected (resources.gates is `None`),
//! the cell fails closed with an error rather than silently passing.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use roko_core::{
    Body, GateResult, Kind, ProtocolId, RungResult, SharedGateError, SharedGateRequest, Signal,
    error::Result,
};
use tracing::{info, warn};

use crate::cell::{Cell, CellContext, CellVersion};

/// Canonical gate rungs executed in order.
///
/// These match the production gate pipeline's core rungs. Additional rungs
/// (diff, fmt, shell, judge) are available in the full `GatePipelineCell`
/// from `roko-gate`; this cell covers the three rungs that every plan task
/// must pass.
const CANONICAL_RUNGS: &[&str] = &["compile", "lint", "test"];

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
    /// Files expected in scope (for `changed_files` in gate requests).
    files: Vec<String>,
}

impl PlanGateCell {
    /// Construct from a TOML node config.
    ///
    /// Expected keys: `task_id`, `plan_id`, `plan_dir`, `files`.
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
            files,
        }
    }

    /// Build a `SharedGateRequest` for one rung.
    fn build_request(&self, rung: &str, worktree: PathBuf) -> SharedGateRequest {
        SharedGateRequest {
            task_id: self.task_id.clone(),
            attempt_id: 0,
            rung: rung.to_owned(),
            plan_dir: self.plan_dir.clone(),
            worktree_path: worktree,
            changed_files: self.files.clone(),
            context: std::iter::once(("plan_id".to_owned(), self.plan_id.clone())).collect(),
        }
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
            roko_core::RokoError::Invalid(
                "PlanGateCell: SharedGateEvaluator not injected (ctx.resources.gates is None); \
                 refusing to silently pass"
                    .to_owned(),
            )
        })?;

        // Resolve the workspace root. Prefer the worktree/workspace from the
        // CellContext or fall back to the current directory.
        let worktree = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        info!(
            task_id = %self.task_id,
            plan_id = %self.plan_id,
            rungs = ?CANONICAL_RUNGS,
            "PlanGateCell: starting gate pipeline"
        );

        let mut rung_results = Vec::with_capacity(CANONICAL_RUNGS.len());
        let mut all_passed = true;

        for &rung in CANONICAL_RUNGS {
            let request = self.build_request(rung, worktree.clone());

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

        let total = rung_results.len() as f64;
        let passed_count = rung_results.iter().filter(|r| r.passed).count() as f64;
        let overall_score = if total > 0.0 {
            passed_count / total
        } else {
            // All rungs were skipped. Treat as pass.
            1.0
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
            roko_core::RokoError::Invalid(format!(
                "PlanGateCell: failed to serialize GateResult: {e}"
            ))
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

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn make_config() -> toml::Value {
        let mut table = toml::map::Map::new();
        table.insert("task_id".into(), toml::Value::String("task-1".into()));
        table.insert("plan_id".into(), toml::Value::String("plan-1".into()));
        table.insert("plan_dir".into(), toml::Value::String("plans/test".into()));
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

    fn input_signals() -> Vec<Signal> {
        vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("task output"))
                .build(),
        ]
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
        let output = cell.execute(input_signals(), &ctx).await.unwrap();

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
    }

    #[tokio::test]
    async fn test_rung_fails() {
        let cell = make_cell();
        let ctx = ctx_with_gates(FailTestEvaluator);
        let output = cell.execute(input_signals(), &ctx).await.unwrap();

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
        let output = cell.execute(input_signals(), &ctx).await.unwrap();

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
        let result = cell.execute(input_signals(), &ctx).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("SharedGateEvaluator not injected"));
    }

    #[tokio::test]
    async fn all_skipped_treated_as_pass() {
        let cell = make_cell();
        let ctx = ctx_with_gates(SkipAllEvaluator);
        let output = cell.execute(input_signals(), &ctx).await.unwrap();

        let result = decode_gate_result(&output[0]);
        assert!(result.passed);
        assert!(result.rung_results.is_empty()); // skipped rungs excluded
        assert_eq!(result.overall_score, 1.0);
    }

    #[test]
    fn build_request_populates_fields() {
        let cell = make_cell();
        let req = cell.build_request("compile", PathBuf::from("/workspace"));

        assert_eq!(req.task_id, "task-1");
        assert_eq!(req.rung, "compile");
        assert_eq!(req.plan_dir, "plans/test");
        assert_eq!(req.worktree_path, PathBuf::from("/workspace"));
        assert_eq!(req.changed_files, vec!["src/lib.rs"]);
        assert_eq!(
            req.context.get("plan_id").map(String::as_str),
            Some("plan-1")
        );
    }
}
