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
//! - the checkout it names does not exist;
//! - the executor handed the checkout on for the gate to settle, and no
//!   workspace provider is injected (`resources.workspaces` is `None`).
//!
//! A pipeline in which no rung ran (every rung skipped) fails. A failed gate
//! is an error, as every cell that fails reports it, so the task fails: the
//! gate's `Success` edge fires only for a gate that passed.
//!
//! A checkout handed on is the gate's to settle (gap-3b5361): an attempt that
//! passes, and whose settled verdict lets its work land, is accepted onto its
//! plan's branch; either way the checkout is kept, for review or after a
//! failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use roko_core::{
    Body, GateResult, Kind, ProtocolId, RokoError, RungResult, SharedGateError,
    SharedGateEvaluator, SharedGateRequest, Signal, error::Result,
};
use tracing::{info, warn};

use crate::cell::{Cell, CellContext, CellVersion};
use crate::cells::task_executor::{TaskAttempt, TaskGateVerdict, truncate_utf8};
use crate::workspace::{
    ExecutionWorkspaceProvider, WorkspaceAcceptRequest, WorkspaceAcceptance, WorkspaceLease,
    WorkspaceReleasePolicy,
};

/// Canonical gate rungs executed in order.
///
/// These match the production gate pipeline's core rungs. Additional rungs
/// (diff, fmt, shell, judge) are available in the full `GatePipelineCell`
/// from `roko-gate`; this cell covers the three rungs that every plan task
/// must pass.
const CANONICAL_RUNGS: &[&str] = &["compile", "lint", "test"];

/// Bytes of each failed rung's evidence kept in a failed gate's error.
const FAILURE_EVIDENCE_MAX_BYTES: usize = 400;

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
        if let Some(lease) = attempt
            .lease
            .as_ref()
            .filter(|lease| lease.path != worktree)
        {
            return Err(RokoError::Invalid(format!(
                "PlanGateCell: attempt {} of task `{}` ran in {} but handed on the lease of {}",
                attempt.attempt,
                self.task_id,
                worktree.display(),
                lease.path.display()
            )));
        }
        Ok((attempt, worktree))
    }

    /// Settle the checkout the executor handed on: accept the attempt when it
    /// `passed` the gate and its settled verdict (stamped on `input`) lets
    /// its work land, then keep the checkout, for review after a pass or for
    /// post-mortem after a failure. An acceptance that fails is an error.
    async fn settle_handed_on(
        &self,
        attempt: &TaskAttempt,
        lease: &WorkspaceLease,
        workspaces: &Arc<dyn ExecutionWorkspaceProvider>,
        passed: bool,
        input: &[Signal],
        run_id: Option<&str>,
    ) -> Result<Option<WorkspaceAcceptance>> {
        let verdict = TaskGateVerdict::from_signals(input).filter(|v| v.is_replayable());
        let accepted = match verdict {
            Some(verdict) if passed => {
                let request = WorkspaceAcceptRequest {
                    run_id: run_id.unwrap_or_default().to_owned(),
                    attempt_key: attempt.attempt_key.clone().unwrap_or_default(),
                    verdict: verdict.as_str().to_owned(),
                    title: self.title.clone(),
                };
                Some(workspaces.accept(lease, &request).await)
            }
            _ => None,
        };
        let policy = if passed {
            WorkspaceReleasePolicy::RetainForReview
        } else {
            WorkspaceReleasePolicy::RetainForFailure
        };
        if let Err(error) = workspaces.release(lease, policy).await {
            warn!(
                task_id = %self.task_id,
                attempt = attempt.attempt,
                %error,
                "PlanGateCell: could not release the attempt's worktree"
            );
        }
        match accepted {
            Some(Ok(acceptance)) => {
                info!(
                    task_id = %self.task_id,
                    attempt = attempt.attempt,
                    plan_branch = %acceptance.plan_branch,
                    accepted_commit = %acceptance.accepted_commit,
                    "PlanGateCell: accepted the attempt onto its plan branch"
                );
                Ok(Some(acceptance))
            }
            Some(Err(error)) => Err(RokoError::Rejected(format!(
                "PlanGateCell: attempt {} of task `{}` passed the plan gate but was not \
                 accepted onto its plan branch: {error}; its worktree is kept at {}",
                attempt.attempt,
                self.task_id,
                lease.path.display()
            ))),
            None => Ok(None),
        }
    }

    /// Run every canonical rung for `attempt` in `worktree` and aggregate
    /// the verdict. A pipeline in which no rung ran fails.
    async fn run_rungs(
        &self,
        evaluator: &dyn SharedGateEvaluator,
        attempt: &TaskAttempt,
        worktree: &Path,
        run_id: Option<&str>,
    ) -> GateResult {
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
            let request = self.build_request(rung, attempt, worktree, run_id);

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

                    // A failed rung without evidence keeps its reasons.
                    let evidence = verdict.evidence.or_else(|| {
                        (!verdict.failed_reasons.is_empty())
                            .then(|| verdict.failed_reasons.join("; "))
                    });
                    rung_results.push(RungResult {
                        rung_name: verdict.rung,
                        passed: verdict.passed,
                        score: if verdict.passed { 1.0 } else { 0.0 },
                        evidence,
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

        GateResult {
            passed: all_passed,
            rung_results,
            overall_score,
        }
    }

    /// Why `attempt` failed the gate `result` describes: the failed rungs and
    /// their evidence, or that no rung ran.
    fn failure_summary(&self, attempt: &TaskAttempt, result: &GateResult) -> String {
        let reasons = if result.rung_results.is_empty() {
            format!(
                "no gate rung ran: all {} rungs were skipped",
                CANONICAL_RUNGS.len()
            )
        } else {
            result
                .rung_results
                .iter()
                .filter(|rung| !rung.passed)
                .map(|rung| match rung.evidence.as_deref() {
                    Some(evidence) => format!(
                        "{}: {}",
                        rung.rung_name,
                        truncate_utf8(evidence, FAILURE_EVIDENCE_MAX_BYTES)
                    ),
                    None => rung.rung_name.clone(),
                })
                .collect::<Vec<_>>()
                .join("; ")
        };
        format!(
            "attempt {} of task `{}` failed the plan gate: {reasons}",
            attempt.attempt, self.task_id
        )
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
        // A checkout handed on is this gate's to settle, so it needs the
        // provider that holds it before any rung runs.
        let handed_on = match &attempt.lease {
            Some(lease) => Some((
                lease,
                ctx.resources.workspaces.as_ref().ok_or_else(|| {
                    RokoError::Invalid(format!(
                        "PlanGateCell: attempt {} of task `{}` handed its worktree on for the \
                         gate to settle, but no workspace provider is injected \
                         (ctx.resources.workspaces is None)",
                        attempt.attempt, self.task_id
                    ))
                })?,
            )),
            None => None,
        };

        let gate_result = self
            .run_rungs(evaluator.as_ref(), &attempt, worktree, run_id)
            .await;
        let passed = gate_result.passed;

        info!(
            task_id = %self.task_id,
            passed,
            score = gate_result.overall_score,
            "PlanGateCell: gate pipeline complete"
        );

        // A handed-on checkout is settled either way: accepted after a pass,
        // kept for post-mortem after a failure.
        let accepted = match handed_on {
            Some((lease, workspaces)) => {
                self.settle_handed_on(&attempt, lease, workspaces, passed, &input, run_id)
                    .await?
            }
            None => None,
        };
        // A failed gate fails its task: the cell errors, as every cell that
        // fails does, so the gate's `Success` edge does not fire (bug-8835bc).
        if !passed {
            return Err(RokoError::Verify {
                gate: self.cell_id().to_owned(),
                message: self.failure_summary(&attempt, &gate_result),
            });
        }

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

        // The attempt as settled here: a handed-on checkout is accepted, and
        // no longer handed on.
        if attempt.lease.is_some() {
            TaskAttempt {
                lease: None,
                accepted,
                ..attempt.clone()
            }
            .stamp(std::slice::from_mut(&mut output));
        }

        output
            .tags
            .insert("gate.passed".to_owned(), passed.to_string());
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
    use crate::workspace::WorkspaceReconcileResult;
    use crate::workspace::fake::InMemoryWorkspaceProvider;

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
            workspaces: None,
        })
    }

    fn ctx_with_gates_and_workspaces(
        evaluator: impl SharedGateEvaluator,
        workspaces: &Arc<InMemoryWorkspaceProvider>,
    ) -> CellContext {
        CellContext::new().with_resources(CellResources {
            gates: Some(Arc::new(evaluator)),
            workspaces: Some(Arc::clone(workspaces) as Arc<dyn ExecutionWorkspaceProvider>),
        })
    }

    /// A provider holding the checkout of attempt 2 of `task-1`, handed on
    /// by an executor whose verify steps passed, and that executor's output.
    async fn handed_on_attempt(
        worktrees: &Path,
    ) -> (Arc<InMemoryWorkspaceProvider>, WorkspaceLease, Vec<Signal>) {
        let provider = Arc::new(InMemoryWorkspaceProvider::new(
            PathBuf::from("/repo"),
            worktrees.to_path_buf(),
        ));
        let lease = provider
            .acquire(&crate::workspace::WorkspaceAttemptId {
                plan_id: "plan-1".to_string(),
                task_id: "task-1".to_string(),
                attempt: 0,
            })
            .await
            .expect("lease");
        std::fs::create_dir_all(&lease.path).expect("checkout");
        let attempt = TaskAttempt {
            lease: Some(lease.clone()),
            ..attempt_in(Some(lease.path.as_path()))
        };
        let mut input = executor_output(&attempt);
        TaskGateVerdict::Passed.stamp(&mut input);
        (provider, lease, input)
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
            accepted: None,
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

    /// bug-9c5973: one gate asks for each canonical rung once, every request
    /// for the same attempt and checkout. An evaluator that runs its
    /// pipeline on an attempt's first request (the CLI's gate adapter) then
    /// answers the other rungs from that run instead of compiling again.
    #[tokio::test]
    async fn a_gate_runs_each_rung_once() {
        let cell = make_cell();
        let evaluator = RecordingEvaluator::default();
        let ctx = ctx_with_gates(evaluator.clone());
        let (_worktree, input) = gated_attempt();
        cell.execute(input, &ctx).await.expect("the gate passes");

        let requests = evaluator.requests.lock();
        let rungs: Vec<&str> = requests
            .iter()
            .map(|request| request.rung.as_str())
            .collect();
        assert_eq!(rungs, CANONICAL_RUNGS);
        let attempts: std::collections::BTreeSet<_> = requests
            .iter()
            .map(|request| {
                (
                    request.attempt_id,
                    request.worktree_path.clone(),
                    request.context.get("attempt_key").cloned(),
                )
            })
            .collect();
        assert_eq!(attempts.len(), 1, "{attempts:?}");
    }

    /// bug-8835bc: a failed rung fails the gate with an error that names
    /// the rung and its reasons.
    #[tokio::test]
    async fn test_rung_fails() {
        let cell = make_cell();
        let ctx = ctx_with_gates(FailTestEvaluator);
        let (_worktree, input) = gated_attempt();
        let error = cell.execute(input, &ctx).await.unwrap_err();

        assert!(
            matches!(&error, RokoError::Verify { gate, .. } if gate == "plan.gate"),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("attempt 2 of task `task-1`"), "{message}");
        assert!(message.contains("test: 3 test failures"), "{message}");
        assert!(!message.contains("compile"), "{message}");
    }

    #[tokio::test]
    async fn evaluator_error_produces_failed_rung() {
        let cell = make_cell();
        let ctx = ctx_with_gates(ErrorEvaluator);
        let (_worktree, input) = gated_attempt();
        let message = cell.execute(input, &ctx).await.unwrap_err().to_string();

        // All 3 rungs failed with the evaluator's error as evidence.
        for rung in CANONICAL_RUNGS {
            let evidence = format!("{rung}: gate evaluation error: evaluator crash");
            assert!(message.contains(&evidence), "{message}");
        }
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
        let message = cell.execute(input, &ctx).await.unwrap_err().to_string();

        assert!(message.contains("no gate rung ran"), "{message}");
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

    /// gap-3b5361: a handed-on checkout that passes is accepted onto its
    /// plan's branch and kept for review; the verdict names where the work
    /// landed and no longer hands the checkout on.
    #[tokio::test]
    async fn plan_gate_accepts_a_handed_on_worktree_that_passes() {
        let worktrees = tempfile::tempdir().expect("worktrees");
        let (provider, lease, input) = handed_on_attempt(worktrees.path()).await;
        let ctx = ctx_with_gates_and_workspaces(AllPassEvaluator, &provider);

        let output = make_cell().execute(input, &ctx).await.unwrap();

        assert_eq!(output[0].tag("gate.passed"), Some("true"));
        let settled = TaskAttempt::from_signals(&output).expect("attempt");
        assert_eq!(settled.lease, None);
        let accepted = settled.accepted.expect("accepted onto the plan branch");
        assert_eq!(accepted.plan_branch, "roko/plan/plan-1");
        assert!(matches!(
            provider.reconcile(&lease).await.unwrap(),
            WorkspaceReconcileResult::Orphaned(_)
        ));
    }

    /// gap-3b5361: a handed-on checkout that fails the gate is kept for
    /// post-mortem and never accepted.
    #[tokio::test]
    async fn plan_gate_keeps_a_failed_worktree_without_accepting_it() {
        let worktrees = tempfile::tempdir().expect("worktrees");
        let (provider, lease, input) = handed_on_attempt(worktrees.path()).await;
        let ctx = ctx_with_gates_and_workspaces(FailTestEvaluator, &provider);

        let error = make_cell().execute(input, &ctx).await.unwrap_err();

        assert!(
            error.to_string().contains("failed the plan gate"),
            "{error}"
        );
        assert!(provider.active_leases().is_empty());
        assert!(matches!(
            provider.reconcile(&lease).await.unwrap(),
            WorkspaceReconcileResult::Orphaned(_)
        ));
    }

    /// gap-3b5361: a checkout handed on to a gate that cannot settle it is
    /// refused before any rung runs.
    #[tokio::test]
    async fn plan_gate_needs_a_workspace_provider_for_a_handed_on_worktree() {
        let worktrees = tempfile::tempdir().expect("worktrees");
        let (_provider, _lease, input) = handed_on_attempt(worktrees.path()).await;
        let recorder = RecordingEvaluator::default();
        let ctx = ctx_with_gates(recorder.clone());

        let error = make_cell().execute(input, &ctx).await.unwrap_err();

        assert!(error.to_string().contains("workspaces is None"), "{error}");
        assert!(recorder.requests.lock().is_empty());
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
