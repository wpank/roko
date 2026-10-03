//! RunnerProductionGateAdapter — bridges Runner-v2 gate parameters to the
//! shared [`ProductionGateRunner`] service.
//!
//! Extracted from `gate_dispatch.rs` to keep the adapter layer separate from
//! the inline gate execution pipeline.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_core::config::GatesConfig;
use roko_gate::PlanComplexity;
use roko_gate::generated_test_gate::ArtifactStore as GeneratedArtifactStore;
use tracing::error;

use crate::task_parser::VerifyStep;

use super::gate_dispatch::{GateTaskContext, failed_gate_completion};
use super::types::{GateCompletion, GateEffectRef, GateVerdictSummary, RunnerFailureKind};

// ═══════════════════════════════════════════════════════════════════════════
// RunnerProductionGateAdapter (#275)
// ═══════════════════════════════════════════════════════════════════════════

/// Adapter that converts Runner-v2 gate parameters into a
/// [`ProductionGateRequest`], calls the injected
/// [`ProductionGateRunner`], and converts the
/// [`ProductionGateVerdictV1`] back into a [`GateCompletion`].
///
/// This is the single point of conversion between the Runner-v2 types
/// (which own event-loop integration, attempt ownership, and TUI events)
/// and the shared production gate service (which owns rung selection,
/// execution, and verdict normalization).
///
/// ## Call sites
///
/// The rich-topology path of `plan run` gates through it (`verify_rung`);
/// [`Self::run`] converts one Runner-v2 rung request. The Runner-v2 spawn
/// helpers that called it are gone (7129).
pub struct RunnerProductionGateAdapter {
    /// The injected shared gate service.
    service: Arc<dyn roko_gate::production_service::ProductionGateRunner>,
    /// The `[gates]` config shared-gate requests run with
    /// ([`Self::with_gates_config`]).
    gates_config: GatesConfig,
    /// Pipeline verdicts shared-gate requests of an attempt read after its
    /// first, by attempt (see [`SHARED_GATE_LOOKAHEAD`]).
    shared_runs: parking_lot::Mutex<HashMap<String, SharedPipelineRun>>,
}

/// The rung a shared-gate request runs the pipeline up to, at least: the
/// last of the rungs the Graph plan gate asks about in turn (compile, lint,
/// test). Its first request of an attempt runs them all, and the others
/// read that verdict instead of compiling again.
const SHARED_GATE_LOOKAHEAD: roko_gate::rung_selector::Rung = roko_gate::rung_selector::Rung::Test;

/// Attempts whose pipeline verdicts are kept at most.
const SHARED_RUNS_MAX: usize = 64;

/// An attempt's pipeline run: the rung it ran up to, and its verdict.
struct SharedPipelineRun {
    ceiling: u32,
    verdict: Arc<roko_gate::ProductionGateVerdictV1>,
}

impl std::fmt::Debug for RunnerProductionGateAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerProductionGateAdapter")
            .finish_non_exhaustive()
    }
}

impl RunnerProductionGateAdapter {
    /// Create an adapter wrapping the given shared service.
    pub fn new(service: Arc<dyn roko_gate::production_service::ProductionGateRunner>) -> Self {
        Self {
            service,
            gates_config: GatesConfig::default(),
            shared_runs: parking_lot::Mutex::default(),
        }
    }

    /// Run shared-gate requests with the run's `[gates]` config instead of
    /// the defaults.
    #[must_use]
    pub fn with_gates_config(mut self, gates_config: GatesConfig) -> Self {
        self.gates_config = gates_config;
        self
    }

    /// Convert Runner-v2 parameters into a `ProductionGateRequest`.
    pub(crate) fn build_request(
        effect: &GateEffectRef,
        plan_id: &str,
        task_id: &str,
        workdir: &Path,
        gates_config: &GatesConfig,
        verify_steps: &[VerifyStep],
        timeout_secs: u64,
        target_crates: &[String],
        task_context: Option<&GateTaskContext>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> roko_gate::ProductionGateRequest {
        // Convert CLI VerifyStep -> neutral VerifyStepSpec.
        let verify_step_specs: Vec<roko_gate::VerifyStepSpec> = verify_steps
            .iter()
            .map(|step| {
                roko_gate::VerifyStepSpec::from_command(&step.command)
                    .with_phase(&step.phase)
                    .with_timeout_ms(step.timeout_ms)
            })
            .collect();

        // Convert GateTaskContext -> GateTaskContextSpec.
        let task_context_spec = task_context
            .map(|ctx| roko_gate::GateTaskContextSpec {
                title: ctx.task_title.clone(),
                description: ctx.task_description.clone(),
                symbols: ctx.symbols.clone(),
                acceptance: ctx.acceptance.clone(),
            })
            .unwrap_or_default();

        // Compute workspace fingerprint synchronously from the workdir.
        let workspace_fingerprint = format!("{}:{}:{}", plan_id, task_id, effect.generation);

        roko_gate::ProductionGateRequest {
            run_id: format!("{}:{}", plan_id, effect.generation),
            plan_id: plan_id.to_string(),
            task_id: task_id.to_string(),
            attempt: effect.attempt.attempt,
            workspace: workdir.to_path_buf(),
            workspace_fingerprint,
            changed_files: target_crates.to_vec(),
            verify_steps: verify_step_specs,
            gates_config: gates_config.clone(),
            task_context: task_context_spec,
            timeout_secs,
            cancel,
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        }
    }

    /// Convert a `ProductionGateVerdictV1` back into a `GateCompletion`.
    pub(crate) fn verdict_to_completion(
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        verdict: &roko_gate::ProductionGateVerdictV1,
    ) -> GateCompletion {
        let passed = verdict.passed();

        // Map per-rung verdicts to GateVerdictSummary.
        let summaries: Vec<GateVerdictSummary> = verdict
            .rung_verdicts
            .iter()
            .map(|rv| {
                let failure_kind = if rv.skipped() || rv.passed() {
                    None
                } else {
                    rv.failure_classification
                        .as_ref()
                        .map(|fc| match fc.recommended_action {
                            roko_gate::GateFailureAction::Blocked => RunnerFailureKind::Resource,
                            roko_gate::GateFailureAction::NeedsHuman => {
                                RunnerFailureKind::Permanent
                            }
                            roko_gate::GateFailureAction::NeedsReplan => {
                                RunnerFailureKind::Structural
                            }
                            roko_gate::GateFailureAction::Retry => RunnerFailureKind::Transient,
                        })
                        .or(Some(RunnerFailureKind::Unknown))
                };
                GateVerdictSummary {
                    gate_name: rv.gate_name.clone(),
                    passed: rv.passed(),
                    skipped: rv.skipped(),
                    summary: rv.diagnostic.chars().take(500).collect(),
                    error_digest: rv
                        .failure_classification
                        .as_ref()
                        .map(|fc| format!("{:?}", fc.primary)),
                    failure_kind,
                    rung_index: Some(rv.rung.as_index()),
                }
            })
            .collect();

        let selected_rungs: Vec<String> = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.skipped())
            .map(|rv| rv.rung.label().to_string())
            .collect();

        let failure_kind = if !passed {
            summaries
                .iter()
                .find_map(|s| s.failure_kind)
                .or(Some(RunnerFailureKind::Unknown))
        } else {
            None
        };

        // Collect output from rung diagnostics.
        let output: String = verdict
            .rung_verdicts
            .iter()
            .filter(|rv| !rv.diagnostic.is_empty())
            .map(|rv| format!("{}: {}", rv.gate_name, rv.diagnostic))
            .collect::<Vec<_>>()
            .join("; ");

        GateCompletion {
            kind: effect.kind,
            attempt: Some(effect.attempt.clone()),
            effect: Some(effect),
            plan_id,
            task_id,
            rung,
            passed,
            failure_kind,
            verdicts: summaries,
            output,
            duration_ms: verdict.total_duration.as_millis() as u64,
            selected_rungs,
        }
    }

    /// Run the production gate pipeline through the shared service and return
    /// a `GateCompletion` compatible with the Runner-v2 event loop.
    ///
    /// It stands in for the inline execution of `run_gate_once`.
    pub async fn run(
        &self,
        effect: GateEffectRef,
        plan_id: String,
        task_id: String,
        rung: u32,
        workdir: PathBuf,
        gates_config: GatesConfig,
        _complexity: PlanComplexity,
        verify_steps: Vec<VerifyStep>,
        _baseline_failed_gates: Option<Vec<GateVerdictSummary>>,
        timeout_secs: u64,
        target_crates: Vec<String>,
        task_context: Option<GateTaskContext>,
    ) -> GateCompletion {
        let cancel = tokio_util::sync::CancellationToken::new();
        let request = Self::build_request(
            &effect,
            &plan_id,
            &task_id,
            &workdir,
            &gates_config,
            &verify_steps,
            timeout_secs,
            &target_crates,
            task_context.as_ref(),
            cancel,
        );

        let progress = Arc::new(roko_gate::production_service::NoopProgressSink);
        match self.service.run(request, progress).await {
            Ok(verdict) => Self::verdict_to_completion(effect, plan_id, task_id, rung, &verdict),
            Err(err) => {
                error!(%err, "production gate service error");
                failed_gate_completion(
                    effect,
                    plan_id,
                    task_id,
                    rung,
                    format!("production gate service error: {err}"),
                )
            }
        }
    }
}

// ── Generated-test artifact store ───────────────────────────────────────

/// Filesystem-backed store for generated test artifacts, keyed by plan.
#[derive(Clone, Debug)]
pub(crate) struct FsGeneratedArtifactStore {
    root: PathBuf,
}

impl FsGeneratedArtifactStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn artifact_dir(&self) -> PathBuf {
        self.root.join("generated-tests")
    }

    pub(crate) fn matching_entries(&self, prefix: &str) -> Vec<String> {
        let dir = self.artifact_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut names: Vec<String> = entries
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                entry.file_type().ok().filter(|kind| kind.is_file())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let logical = format!("generated-tests/{name}");
                logical.starts_with(prefix).then_some(logical)
            })
            .collect();
        names.sort();
        names
    }
}

impl GeneratedArtifactStore for FsGeneratedArtifactStore {
    fn list(&self, _plan: &str, prefix: &str) -> Vec<String> {
        self.matching_entries(prefix)
    }

    fn read(&self, _plan: &str, name: &str) -> Option<Vec<u8>> {
        let relative = name.strip_prefix("generated-tests/")?;
        if relative.contains("..") || relative.contains('/') {
            return None;
        }
        std::fs::read(self.artifact_dir().join(relative)).ok()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SharedGateEvaluator implementation (#275)
//
// Makes RunnerProductionGateAdapter consumable from the Graph side as
// `CellContext.resources.gates`. Both Runner-v2 and Graph can now use
// the exact same adapter instance to evaluate gate rungs.
// ═══════════════════════════════════════════════════════════════════════════

#[async_trait::async_trait]
impl roko_core::SharedGateEvaluator for RunnerProductionGateAdapter {
    async fn verify_rung(
        &self,
        request: &roko_core::SharedGateRequest,
    ) -> std::result::Result<roko_core::SharedGateVerdict, roko_core::SharedGateError> {
        use roko_core::{SharedGateError, SharedGateVerdict};

        // Parse the rung name to validate it.
        let rung = roko_gate::rung_selector::Rung::from_label(&request.rung).ok_or_else(|| {
            SharedGateError::UnknownRung {
                rung: request.rung.clone(),
            }
        })?;

        // Build a minimal ProductionGateRequest from the shared request, with
        // the run's `[gates]`. The pipeline runs up to the requested rung or
        // the lookahead, whichever is higher, within the run's `max_rung`.
        let cancel = tokio_util::sync::CancellationToken::new();
        let mut gates_config = self.gates_config.clone();
        let ceiling = rung.as_index().max(SHARED_GATE_LOOKAHEAD.as_index());
        let ceiling = gates_config
            .max_rung
            .map_or(ceiling, |max| ceiling.min(u32::from(max)));
        gates_config.max_rung = Some(u8::try_from(ceiling).unwrap_or(u8::MAX));

        // The request names its plan, run and attempt key in its context (the
        // Graph plan gate sends them); without them each attempt of a task
        // still gets an identity of its own.
        let context = |key: &str| {
            request
                .context
                .get(key)
                .filter(|value| !value.is_empty())
                .cloned()
        };
        let own_identity = || format!("shared:{}:{}", request.task_id, request.attempt_id);
        let production_request = roko_gate::ProductionGateRequest {
            run_id: context("run_id").unwrap_or_else(own_identity),
            plan_id: context("plan_id").unwrap_or_else(|| request.plan_dir.clone()),
            task_id: request.task_id.clone(),
            attempt: request.attempt_id,
            workspace: request.worktree_path.clone(),
            workspace_fingerprint: context("attempt_key").unwrap_or_else(own_identity),
            changed_files: request.changed_files.clone(),
            verify_steps: Vec::new(),
            gates_config,
            task_context: roko_gate::production_request::GateTaskContextSpec {
                title: request.context.get("title").cloned().unwrap_or_default(),
                description: request.context.get("description").cloned(),
                symbols: Vec::new(),
                acceptance: Vec::new(),
            },
            timeout_secs: 600,
            cancel,
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        };

        // A later request of the attempt reads the verdict of its first.
        let key = format!(
            "{}\0{}\0{}\0{}\0{}\0{}",
            production_request.run_id,
            production_request.plan_id,
            production_request.task_id,
            production_request.attempt,
            production_request.workspace_fingerprint,
            production_request.workspace.display()
        );
        let cached = {
            let mut runs = self.shared_runs.lock();
            match runs.get(&key) {
                Some(run) if run.ceiling >= rung.as_index() => {
                    let verdict = Arc::clone(&run.verdict);
                    if rung.as_index() >= run.ceiling {
                        runs.remove(&key);
                    }
                    Some(verdict)
                }
                _ => None,
            }
        };
        let verdict_v1 = match cached {
            Some(verdict) => verdict,
            None => {
                let progress = Arc::new(roko_gate::production_service::NoopProgressSink);
                let verdict = Arc::new(
                    self.service
                        .run(production_request, progress)
                        .await
                        .map_err(|err| SharedGateError::Internal {
                            reason: err.to_string(),
                        })?,
                );
                // A pipeline that ran no rung is not kept: a later request
                // tries again.
                if rung.as_index() < ceiling && !verdict.rung_verdicts.is_empty() {
                    let mut runs = self.shared_runs.lock();
                    if runs.len() >= SHARED_RUNS_MAX {
                        runs.clear();
                    }
                    runs.insert(
                        key,
                        SharedPipelineRun {
                            ceiling,
                            verdict: Arc::clone(&verdict),
                        },
                    );
                }
                verdict
            }
        };

        // Convert the production verdict into a SharedGateVerdict.
        // Find the verdict for the specific requested rung.
        let rung_verdict = verdict_v1.rung_verdicts.iter().find(|rv| rv.rung == rung);

        match rung_verdict {
            Some(rv) => {
                if rv.skipped() {
                    Ok(SharedGateVerdict::skip(&request.rung))
                } else if rv.passed() {
                    let mut v = SharedGateVerdict::pass(&request.rung);
                    if !rv.diagnostic.is_empty() {
                        v.evidence = Some(rv.diagnostic.clone());
                    }
                    Ok(v)
                } else {
                    let reasons: Vec<String> = rv
                        .failure_classification
                        .as_ref()
                        .map(|fc| vec![format!("{:?}: {}", fc.primary, rv.diagnostic)])
                        .unwrap_or_else(|| vec![rv.diagnostic.clone()]);
                    let mut v = SharedGateVerdict::fail(&request.rung, reasons);
                    if !rv.diagnostic.is_empty() {
                        v.evidence = Some(rv.diagnostic.clone());
                    }
                    Ok(v)
                }
            }
            None => {
                // No verdict for the requested rung -- may have been skipped
                // or short-circuited by an earlier rung failure.
                if verdict_v1.rung_verdicts.is_empty() {
                    // Pipeline was cancelled or timed out before any rung ran.
                    match verdict_v1.outcome {
                        roko_gate::production_verdict::PipelineOutcome::Cancelled => {
                            Err(SharedGateError::Cancelled)
                        }
                        roko_gate::production_verdict::PipelineOutcome::TimedOut => {
                            Err(SharedGateError::Timeout { timeout_secs: 600 })
                        }
                        _ => Ok(SharedGateVerdict::skip(&request.rung)),
                    }
                } else {
                    // Earlier rung failed and short-circuited before reaching
                    // the requested rung. Report as skip with context.
                    Ok(SharedGateVerdict::skip(&request.rung))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use roko_core::{SharedGateEvaluator, SharedGateRequest};
    use roko_gate::production_service::{ProductionGateRunner, ProgressSink};

    use super::*;

    /// The identity of the one production request it was asked to run.
    type Recorded = (String, String, u32, PathBuf, String);

    /// Records the identity of each request, then fails it.
    #[derive(Default)]
    struct RecordingRunner {
        requests: parking_lot::Mutex<Vec<Recorded>>,
    }

    #[async_trait::async_trait]
    impl ProductionGateRunner for RecordingRunner {
        async fn run(
            &self,
            request: roko_gate::ProductionGateRequest,
            _progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<roko_gate::ProductionGateVerdictV1> {
            self.requests.lock().push((
                request.run_id,
                request.plan_id,
                request.attempt,
                request.workspace,
                request.workspace_fingerprint,
            ));
            Err(roko_core::RokoError::Invalid("recorded".to_string()))
        }
    }

    /// Passes every rung up to the request's `max_rung`, and records the
    /// `[gates]` config of each request.
    #[derive(Default)]
    struct PassingRunner {
        configs: parking_lot::Mutex<Vec<GatesConfig>>,
    }

    #[async_trait::async_trait]
    impl ProductionGateRunner for PassingRunner {
        async fn run(
            &self,
            request: roko_gate::ProductionGateRequest,
            _progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<roko_gate::ProductionGateVerdictV1> {
            use roko_gate::production_verdict::{
                EvidenceRef, PipelineOutcome, ProductionGateRungVerdict, RungState,
                VERDICT_SCHEMA_VERSION,
            };
            use roko_gate::rung_selector::Rung;

            let max = request.gates_config.max_rung.map_or(u32::MAX, u32::from);
            self.configs.lock().push(request.gates_config.clone());
            let rung_verdicts = [Rung::Compile, Rung::Lint, Rung::Test]
                .into_iter()
                .filter(|rung| rung.as_index() <= max)
                .map(|rung| ProductionGateRungVerdict {
                    rung,
                    gate_name: rung.label().to_string(),
                    state: RungState::Passed,
                    failure_classification: None,
                    diagnostic: String::new(),
                    evidence: EvidenceRef::default(),
                    duration: std::time::Duration::ZERO,
                    test_counts: None,
                    input_fingerprint: String::new(),
                    skip_reason: None,
                })
                .collect();
            Ok(roko_gate::ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: request.workspace_fingerprint.clone(),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts,
                outcome: PipelineOutcome::Passed,
                mostly_passing: false,
                total_duration: std::time::Duration::ZERO,
                adaptive_snapshot: None,
            })
        }
    }

    fn shared_request(context: &[(&str, &str)]) -> SharedGateRequest {
        SharedGateRequest {
            task_id: "T1".to_string(),
            attempt_id: 2,
            rung: "compile".to_string(),
            plan_dir: "plans/plan-a".to_string(),
            worktree_path: PathBuf::from("/wt/attempt"),
            changed_files: Vec::new(),
            context: context
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect::<HashMap<_, _>>(),
        }
    }

    /// gap-6daad9 follow-up: the Graph plan gate's rung requests for one
    /// attempt run the gate pipeline once, and with the run's `[gates]`
    /// config, not the defaults.
    #[tokio::test]
    async fn an_attempts_rung_requests_run_the_pipeline_once_with_the_runs_gates() {
        let runner = Arc::new(PassingRunner::default());
        let mut gates = GatesConfig::default();
        gates.env_passthrough = vec!["FROM_THE_RUN".to_string()];
        let adapter =
            RunnerProductionGateAdapter::new(Arc::clone(&runner) as _).with_gates_config(gates);
        for rung in ["compile", "lint", "test"] {
            let request = SharedGateRequest {
                rung: rung.to_string(),
                ..shared_request(&[("attempt_key", "run-1:plan-a:T1:2")])
            };
            let verdict = adapter.verify_rung(&request).await.expect("verdict");
            assert!(verdict.passed && !verdict.skipped, "{rung}: {verdict:?}");
        }
        // The next attempt runs a pipeline of its own.
        let next = SharedGateRequest {
            attempt_id: 3,
            ..shared_request(&[("attempt_key", "run-1:plan-a:T1:3")])
        };
        assert!(adapter.verify_rung(&next).await.expect("verdict").passed);

        let configs = runner.configs.lock();
        assert_eq!(configs.len(), 2, "one pipeline run per attempt");
        for config in configs.iter() {
            assert_eq!(config.env_passthrough, ["FROM_THE_RUN"]);
            assert_eq!(config.max_rung, Some(2));
        }
    }

    /// bug-50caf2: a Graph plan gate request is run as its plan, its run and
    /// its exact attempt, in the attempt's checkout.
    #[tokio::test]
    async fn shared_request_keeps_the_plan_run_and_attempt_it_names() {
        let runner = Arc::new(RecordingRunner::default());
        let adapter = RunnerProductionGateAdapter::new(Arc::clone(&runner) as _);
        let named = shared_request(&[
            ("plan_id", "plan-a"),
            ("run_id", "run-7"),
            ("attempt_key", "run-7:plan-a:T1:2"),
        ]);
        assert!(adapter.verify_rung(&named).await.is_err());
        // Without context, each attempt of the task still has its own identity.
        assert!(adapter.verify_rung(&shared_request(&[])).await.is_err());

        let requests = runner.requests.lock();
        let attempt = PathBuf::from("/wt/attempt");
        assert_eq!(
            requests[0],
            (
                "run-7".to_string(),
                "plan-a".to_string(),
                2,
                attempt.clone(),
                "run-7:plan-a:T1:2".to_string()
            )
        );
        assert_eq!(
            requests[1],
            (
                "shared:T1:2".to_string(),
                "plans/plan-a".to_string(),
                2,
                attempt,
                "shared:T1:2".to_string()
            )
        );
    }
}
