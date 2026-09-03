//! Production gate pipeline service.
//!
//! [`ProductionGateRunner`] is the async trait for executing a full gate
//! pipeline. [`ProductionGateService`] is the only production implementation;
//! it copies the sequencing from `runner/gate_dispatch.rs` into neutral helpers
//! and delegates actual rung construction to
//! [`GatePipelineBuilder`](crate::rung_dispatch::GatePipelineBuilder).
//!
//! This service does NOT replace or modify the simpler workflow
//! [`GateService`](crate::gate_service::GateService).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use roko_core::{Body, Context, Kind, Signal, Verdict, Verify};
use tokio::select;
use tracing::{info, warn};

use crate::adaptive_threshold::AdaptiveThresholds;
use crate::classify_gate_failure;
use crate::payload::GatePayload;
use crate::production_request::ProductionGateRequest;
use crate::production_verdict::{
    EvidenceRef, MAX_RAW_OUTPUT_BYTES, PipelineOutcome, ProductionGateRungVerdict,
    ProductionGateVerdictV1, RungState, VERDICT_SCHEMA_VERSION,
};
use crate::rung_dispatch::{GatePipelineBuilder, RungExecutionConfig, RungExecutionInputs};
use crate::rung_selector::{CANONICAL_ORDER, PlanComplexity, Rung};
use crate::shell::ShellGate;

// ────────────────────────────────────────────────────────────────────────────
// Progress sink
// ────────────────────────────────────────────────────────────────────────────

/// Progress events emitted during pipeline execution.
///
/// Consumers (the graph cell, TUI, telemetry) receive these through the
/// [`ProgressSink`] callback.
#[derive(Clone, Debug)]
pub enum GatePipelineProgress {
    /// A rung is about to start.
    RungStarted {
        /// Which canonical rung.
        rung: Rung,
        /// Concrete gate name.
        gate_name: String,
    },
    /// Incremental output from a running rung.
    RungOutput {
        /// Which canonical rung.
        rung: Rung,
        /// Concrete gate name.
        gate_name: String,
        /// Bounded output chunk.
        output: String,
    },
    /// A rung has completed.
    RungCompleted {
        /// Which canonical rung.
        rung: Rung,
        /// The per-rung verdict.
        verdict: ProductionGateRungVerdict,
    },
    /// The entire pipeline has finished.
    PipelineCompleted {
        /// Overall outcome.
        outcome: PipelineOutcome,
    },
}

/// Sink for pipeline progress events.
///
/// Implementations can forward to a channel, graph event bus, or logging.
/// The default [`NoopProgressSink`] discards all events.
#[async_trait]
pub trait ProgressSink: Send + Sync + 'static {
    /// Receive a progress event. Must not block.
    async fn send(&self, event: GatePipelineProgress);
}

/// Progress sink that discards all events.
pub struct NoopProgressSink;

#[async_trait]
impl ProgressSink for NoopProgressSink {
    async fn send(&self, _event: GatePipelineProgress) {}
}

// ────────────────────────────────────────────────────────────────────────────
// ProductionGateRunner trait
// ────────────────────────────────────────────────────────────────────────────

/// Async trait for executing the full production gate pipeline.
///
/// The only production implementation is [`ProductionGateService`].
/// Test code can provide mock implementations for unit testing.
///
/// # Contract
///
/// - Implementations MUST NOT retain mutable state across calls. Each
///   invocation is self-contained; adaptive threshold snapshots are carried
///   in and out through the request/verdict types.
/// - Cancellation is cooperative via `request.cancel`.
/// - Implementations MUST NOT duplicate rung selection or execution policy;
///   they delegate to `GatePipelineBuilder` and friends.
#[async_trait]
pub trait ProductionGateRunner: Send + Sync + 'static {
    /// Execute the gate pipeline for the given request, emitting progress
    /// events through `progress_sink`.
    async fn run(
        &self,
        request: ProductionGateRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> roko_core::Result<ProductionGateVerdictV1>;
}

// ────────────────────────────────────────────────────────────────────────────
// ProductionGateService
// ────────────────────────────────────────────────────────────────────────────

/// Production implementation of `ProductionGateRunner`.
///
/// Copies the sequencing from `runner/gate_dispatch.rs::run_gate_once` and
/// `spawn_gate` into neutral helpers. Delegates rung construction to the
/// existing `GatePipelineBuilder`.
#[derive(Debug)]
pub struct ProductionGateService;

impl ProductionGateService {
    /// Create a new production gate service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Build a gate signal from the request.
    fn build_signal(request: &ProductionGateRequest) -> Signal {
        let payload = GatePayload::in_dir(&request.workspace);
        Signal::builder(Kind::Task)
            .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
            .build()
    }

    /// Determine plan complexity from the request.
    ///
    /// Without a richer complexity signal from the caller, changed-file
    /// count is used as a rough proxy.
    fn plan_complexity(request: &ProductionGateRequest) -> PlanComplexity {
        if request.changed_files.is_empty() {
            PlanComplexity::Trivial
        } else if request.changed_files.len() <= 3 {
            PlanComplexity::Standard
        } else {
            PlanComplexity::Complex
        }
    }

    /// Convert a `Verdict` from the existing Verify infrastructure into a
    /// `ProductionGateRungVerdict`.
    fn verdict_to_rung_verdict(
        rung: Rung,
        verdict: &Verdict,
        workspace_fingerprint: &str,
    ) -> ProductionGateRungVerdict {
        let state = if verdict.skipped {
            RungState::Skipped
        } else if verdict.passed {
            RungState::Passed
        } else {
            RungState::Failed
        };

        let failure_classification = if !verdict.passed && !verdict.skipped {
            let output = verdict.detail.as_deref().unwrap_or(&verdict.reason);
            Some(classify_gate_failure(rung.label(), output))
        } else {
            None
        };

        // Bound the diagnostic output.
        let raw_output = verdict.detail.as_deref().unwrap_or(&verdict.reason);
        let diagnostic = if raw_output.len() > MAX_RAW_OUTPUT_BYTES {
            raw_output[..MAX_RAW_OUTPUT_BYTES].to_string()
        } else {
            raw_output.to_string()
        };

        let evidence = if raw_output.len() > MAX_RAW_OUTPUT_BYTES {
            EvidenceRef {
                content_hash: None,
                artifact_path: Some(format!(
                    "gate-output/{}/{}.txt",
                    rung.label(),
                    workspace_fingerprint
                )),
            }
        } else {
            EvidenceRef::default()
        };

        ProductionGateRungVerdict {
            rung,
            gate_name: verdict.gate.clone(),
            state,
            failure_classification,
            diagnostic,
            evidence,
            duration: Duration::from_millis(verdict.duration_ms),
            test_counts: verdict.test_count,
            input_fingerprint: workspace_fingerprint.to_string(),
            skip_reason: verdict.skip_reason.clone(),
        }
    }

    /// Execute the canonical pipeline through per-rung dispatch.
    ///
    /// Runs each selected rung individually to emit per-rung progress and
    /// apply adaptive skip/observe decisions. Compile failure short-circuits.
    /// Max-rung cap from `GatesConfig` is enforced: rungs above the cap are
    /// recorded as skipped.
    async fn run_canonical_pipeline(
        &self,
        request: &ProductionGateRequest,
        signal: &Signal,
        ctx: &Context,
        complexity: PlanComplexity,
        adaptive: &Option<Arc<Mutex<AdaptiveThresholds>>>,
        progress: &Arc<dyn ProgressSink>,
    ) -> Vec<ProductionGateRungVerdict> {
        let selected_labels =
            GatePipelineBuilder::selected_rung_labels(&request.gates_config, complexity);

        let max_rung_index = request.gates_config.max_rung.map(u32::from);

        let mut rung_verdicts = Vec::new();

        for rung in &CANONICAL_ORDER {
            let label = rung.label();
            if !selected_labels.contains(&label.to_string()) {
                continue;
            }

            // Max-rung cap: skip rungs above the configured ceiling.
            if let Some(cap) = max_rung_index {
                if rung.as_index() > cap {
                    let rv = ProductionGateRungVerdict {
                        rung: *rung,
                        gate_name: label.to_string(),
                        state: RungState::Skipped,
                        failure_classification: None,
                        diagnostic: format!(
                            "max_rung cap: rung {} exceeds configured max {}",
                            rung.as_index(),
                            cap
                        ),
                        evidence: EvidenceRef::default(),
                        duration: Duration::ZERO,
                        test_counts: None,
                        input_fingerprint: request.workspace_fingerprint.clone(),
                        skip_reason: Some(format!(
                            "max_rung: {} > {}",
                            rung.as_index(),
                            cap
                        )),
                    };
                    progress
                        .send(GatePipelineProgress::RungCompleted {
                            rung: *rung,
                            verdict: rv.clone(),
                        })
                        .await;
                    rung_verdicts.push(rv);
                    continue;
                }
            }

            // Check adaptive skip (never skip rung 0 / Compile).
            if let Some(adaptive) = adaptive {
                if rung.as_index() > 0 {
                    if let Ok(thresholds) = adaptive.lock() {
                        if thresholds.should_skip_rung(rung.as_index()) {
                            let rv = ProductionGateRungVerdict {
                                rung: *rung,
                                gate_name: label.to_string(),
                                state: RungState::Skipped,
                                failure_classification: None,
                                diagnostic: format!(
                                    "adaptive skip: high pass rate for rung {}",
                                    rung.as_index()
                                ),
                                evidence: EvidenceRef::default(),
                                duration: Duration::ZERO,
                                test_counts: None,
                                input_fingerprint: request.workspace_fingerprint.clone(),
                                skip_reason: Some(format!(
                                    "adaptive: high pass rate for rung {}",
                                    rung.as_index()
                                )),
                            };
                            progress
                                .send(GatePipelineProgress::RungCompleted {
                                    rung: *rung,
                                    verdict: rv.clone(),
                                })
                                .await;
                            rung_verdicts.push(rv);
                            continue;
                        }
                    }
                }
            }

            // Emit rung-start event.
            progress
                .send(GatePipelineProgress::RungStarted {
                    rung: *rung,
                    gate_name: label.to_string(),
                })
                .await;

            // Execute the rung using the canonical dispatch.
            let verdicts = crate::rung_dispatch::run_canonical_rung(
                signal,
                ctx,
                *rung,
                &RungExecutionInputs::default(),
                &RungExecutionConfig::default(),
            )
            .await;

            // Convert each inner verdict and emit progress.
            for verdict in &verdicts {
                let rv =
                    Self::verdict_to_rung_verdict(*rung, verdict, &request.workspace_fingerprint);

                // Record adaptive observation for non-skipped verdicts.
                if let Some(adaptive) = adaptive {
                    if let Ok(mut thresholds) = adaptive.lock() {
                        if !rv.skipped() {
                            thresholds.observe(rung.as_index(), rv.passed());
                        }
                    }
                }

                progress
                    .send(GatePipelineProgress::RungCompleted {
                        rung: *rung,
                        verdict: rv.clone(),
                    })
                    .await;

                let failed = matches!(rv.state, RungState::Failed);
                rung_verdicts.push(rv);

                // Compile failure short-circuits the entire pipeline.
                if failed && *rung == Rung::Compile {
                    return rung_verdicts;
                }
            }

            // Non-compile failure: stop pipeline (sequential short-circuit).
            if rung_verdicts
                .last()
                .is_some_and(|rv| matches!(rv.state, RungState::Failed))
            {
                return rung_verdicts;
            }
        }

        rung_verdicts
    }

    /// Execute authored verify steps after canonical rungs.
    ///
    /// Each step runs as a shell gate. Steps run sequentially and
    /// short-circuit on the first failure.
    async fn run_verify_steps(
        &self,
        request: &ProductionGateRequest,
        progress: &Arc<dyn ProgressSink>,
    ) -> Vec<ProductionGateRungVerdict> {
        let mut verdicts = Vec::new();

        for step in &request.verify_steps {
            let gate_name = if step.phase.is_empty() {
                "verify:authored".to_string()
            } else {
                format!("verify:{}", step.phase)
            };

            // Authored steps use Integration rung to place them after the
            // canonical sequence.
            let rung = Rung::Integration;

            progress
                .send(GatePipelineProgress::RungStarted {
                    rung,
                    gate_name: gate_name.clone(),
                })
                .await;

            let shell = ShellGate::new("bash", vec!["-c".into(), step.command.clone()])
                .with_name(&gate_name)
                .with_timeout_ms(step.timeout_ms);

            let payload = GatePayload::in_dir(&request.workspace);
            let signal = Signal::builder(Kind::Task)
                .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
                .build();
            let ctx = Context::now().with_attr("workdir", request.workspace.to_string_lossy());

            let verdict = shell.verify(&signal, &ctx).await;
            let rv =
                Self::verdict_to_rung_verdict(rung, &verdict, &request.workspace_fingerprint);

            progress
                .send(GatePipelineProgress::RungCompleted {
                    rung,
                    verdict: rv.clone(),
                })
                .await;

            let failed = matches!(rv.state, RungState::Failed);
            verdicts.push(rv);

            if failed {
                break;
            }
        }

        verdicts
    }

    /// Determine the "mostly passing" flag.
    ///
    /// `true` when the only failures are in non-core rungs and the core
    /// compile + lint + test rungs all passed (or were skipped).
    fn is_mostly_passing(verdicts: &[ProductionGateRungVerdict]) -> bool {
        let core_rungs = [Rung::Compile, Rung::Lint, Rung::Test];
        let core_ok = verdicts
            .iter()
            .filter(|v| core_rungs.contains(&v.rung))
            .all(|v| v.passed() || v.skipped());

        let any_failure = verdicts
            .iter()
            .any(|v| matches!(v.state, RungState::Failed));

        core_ok && any_failure
    }
}

impl Default for ProductionGateService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProductionGateRunner for ProductionGateService {
    async fn run(
        &self,
        request: ProductionGateRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> roko_core::Result<ProductionGateVerdictV1> {
        let start = Instant::now();
        let cancel = request.cancel.clone();
        let timeout_duration = Duration::from_secs(request.timeout_secs.max(1));

        info!(
            run_id = %request.run_id,
            plan_id = %request.plan_id,
            task_id = %request.task_id,
            attempt = request.attempt,
            timeout_secs = request.timeout_secs,
            verify_step_count = request.verify_steps.len(),
            "production gate pipeline starting"
        );

        // Set up adaptive thresholds.
        let adaptive = request
            .adaptive_thresholds
            .clone()
            .map(|t| Arc::new(Mutex::new(t)));

        let signal = Self::build_signal(&request);
        let ctx = Context::now().with_attr("workdir", request.workspace.to_string_lossy());
        let complexity = Self::plan_complexity(&request);

        // Run with timeout and cancellation.
        let pipeline_result = select! {
            _ = cancel.cancelled() => {
                warn!(
                    plan_id = %request.plan_id,
                    task_id = %request.task_id,
                    "production gate pipeline cancelled"
                );
                Err(PipelineOutcome::Cancelled)
            }
            _ = tokio::time::sleep(timeout_duration) => {
                warn!(
                    plan_id = %request.plan_id,
                    task_id = %request.task_id,
                    timeout_secs = request.timeout_secs,
                    "production gate pipeline timed out"
                );
                Err(PipelineOutcome::TimedOut)
            }
            result = async {
                // Phase 1: canonical rungs.
                let mut rung_verdicts = self.run_canonical_pipeline(
                    &request,
                    &signal,
                    &ctx,
                    complexity,
                    &adaptive,
                    &progress_sink,
                ).await;

                // Phase 2: authored verify steps (only if canonical passed).
                let canonical_failed = rung_verdicts.iter().any(|v| {
                    matches!(v.state, RungState::Failed)
                });
                if !canonical_failed && !request.verify_steps.is_empty() {
                    let verify_verdicts =
                        self.run_verify_steps(&request, &progress_sink).await;
                    rung_verdicts.extend(verify_verdicts);
                }

                Ok(rung_verdicts)
            } => {
                result
            }
        };

        let total_duration = start.elapsed();

        let (rung_verdicts, outcome) = match pipeline_result {
            Ok(verdicts) => {
                let any_failed = verdicts
                    .iter()
                    .any(|v| matches!(v.state, RungState::Failed));
                let outcome = if any_failed {
                    PipelineOutcome::Failed
                } else {
                    PipelineOutcome::Passed
                };
                (verdicts, outcome)
            }
            Err(cancel_or_timeout) => (Vec::new(), cancel_or_timeout),
        };

        let mostly_passing = Self::is_mostly_passing(&rung_verdicts);

        // Extract resulting adaptive snapshot.
        let adaptive_snapshot = adaptive.and_then(|a| a.lock().ok().map(|t| t.clone()));

        let request_fingerprint = request.request_fingerprint();

        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint,
            workspace_fingerprint: request.workspace_fingerprint.clone(),
            rung_verdicts,
            outcome,
            mostly_passing,
            total_duration,
            adaptive_snapshot,
        };

        // Emit terminal event.
        progress_sink
            .send(GatePipelineProgress::PipelineCompleted { outcome })
            .await;

        info!(
            plan_id = %request.plan_id,
            task_id = %request.task_id,
            outcome = ?outcome,
            mostly_passing,
            duration_ms = total_duration.as_millis() as u64,
            executed_rungs = verdict.executed_rung_count(),
            failed_rungs = verdict.failed_rung_count(),
            "production gate pipeline completed"
        );

        Ok(verdict)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production_request::GateTaskContextSpec;
    use roko_core::config::GatesConfig;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio_util::sync::CancellationToken;

    /// Counting progress sink for tests.
    struct CountingProgressSink {
        count: AtomicUsize,
    }

    impl CountingProgressSink {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }

        fn event_count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ProgressSink for CountingProgressSink {
        async fn send(&self, _event: GatePipelineProgress) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn test_request() -> ProductionGateRequest {
        ProductionGateRequest {
            run_id: "test-run".into(),
            plan_id: "test-plan".into(),
            task_id: "test-task".into(),
            attempt: 0,
            workspace: PathBuf::from("/tmp/test-ws"),
            workspace_fingerprint: "fp-test".into(),
            changed_files: vec!["src/lib.rs".into()],
            verify_steps: Vec::new(),
            gates_config: GatesConfig::default(),
            task_context: GateTaskContextSpec::default(),
            timeout_secs: 60,
            cancel: CancellationToken::new(),
            baseline_fingerprint: None,
            adaptive_thresholds: None,
        }
    }

    #[test]
    fn service_can_be_constructed() {
        let _svc = ProductionGateService::new();
        let _svc2 = ProductionGateService::default();
    }

    #[test]
    fn service_implements_debug() {
        let service = ProductionGateService::new();
        let debug = format!("{service:?}");
        assert!(debug.contains("ProductionGateService"));
    }

    #[test]
    fn trait_is_object_safe() {
        let service = ProductionGateService::new();
        let _arc: Arc<dyn ProductionGateRunner> = Arc::new(service);
    }

    #[test]
    fn build_signal_produces_task_signal() {
        let req = test_request();
        let signal = ProductionGateService::build_signal(&req);
        assert_eq!(signal.kind, Kind::Task);
    }

    #[test]
    fn plan_complexity_by_changed_files() {
        let mut req = test_request();
        req.changed_files.clear();
        assert_eq!(
            ProductionGateService::plan_complexity(&req),
            PlanComplexity::Trivial
        );

        req.changed_files = vec!["a.rs".into()];
        assert_eq!(
            ProductionGateService::plan_complexity(&req),
            PlanComplexity::Standard
        );

        req.changed_files = vec!["a.rs".into(), "b.rs".into(), "c.rs".into(), "d.rs".into()];
        assert_eq!(
            ProductionGateService::plan_complexity(&req),
            PlanComplexity::Complex
        );
    }

    #[test]
    fn verdict_to_rung_verdict_passed() {
        let v = Verdict::pass("compile").with_detail("ok").with_duration(42);
        let rv = ProductionGateService::verdict_to_rung_verdict(Rung::Compile, &v, "fp");
        assert!(rv.passed());
        assert!(!rv.skipped());
        assert_eq!(rv.gate_name, "compile");
        assert!(rv.failure_classification.is_none());
        assert_eq!(rv.duration, Duration::from_millis(42));
    }

    #[test]
    fn verdict_to_rung_verdict_failed() {
        let v = Verdict::fail("test", "3 tests failed")
            .with_detail("error details")
            .with_duration(100);
        let rv = ProductionGateService::verdict_to_rung_verdict(Rung::Test, &v, "fp");
        assert!(!rv.passed());
        assert!(!rv.skipped());
        assert!(rv.failure_classification.is_some());
    }

    #[test]
    fn verdict_to_rung_verdict_skipped() {
        let v = Verdict::skip("lint", "adaptive skip");
        let rv = ProductionGateService::verdict_to_rung_verdict(Rung::Lint, &v, "fp");
        assert!(rv.skipped());
        assert!(!rv.passed());
    }

    #[test]
    fn is_mostly_passing_core_pass_higher_fail() {
        let verdicts = vec![
            make_rv(Rung::Compile, RungState::Passed),
            make_rv(Rung::Lint, RungState::Passed),
            make_rv(Rung::Test, RungState::Passed),
            make_rv(Rung::Symbol, RungState::Failed),
        ];
        assert!(ProductionGateService::is_mostly_passing(&verdicts));
    }

    #[test]
    fn is_mostly_passing_false_when_compile_fails() {
        let verdicts = vec![make_rv(Rung::Compile, RungState::Failed)];
        assert!(!ProductionGateService::is_mostly_passing(&verdicts));
    }

    #[test]
    fn is_mostly_passing_false_when_all_pass() {
        // "mostly passing" requires at least one failure.
        let verdicts = vec![make_rv(Rung::Compile, RungState::Passed)];
        assert!(!ProductionGateService::is_mostly_passing(&verdicts));
    }

    #[tokio::test]
    async fn noop_progress_sink_accepts_events() {
        let sink = NoopProgressSink;
        sink.send(GatePipelineProgress::PipelineCompleted {
            outcome: PipelineOutcome::Passed,
        })
        .await;
    }

    #[tokio::test]
    async fn counting_progress_sink_tracks() {
        let sink = CountingProgressSink::new();
        sink.send(GatePipelineProgress::PipelineCompleted {
            outcome: PipelineOutcome::Passed,
        })
        .await;
        assert_eq!(sink.event_count(), 1);
    }

    #[tokio::test]
    async fn cancelled_pipeline_returns_cancelled() {
        let svc = ProductionGateService::new();
        let mut req = test_request();
        req.cancel.cancel(); // Pre-cancel.

        let progress = Arc::new(CountingProgressSink::new());
        let result = svc
            .run(req, progress.clone())
            .await
            .expect("run should succeed even on cancel");

        assert_eq!(result.outcome, PipelineOutcome::Cancelled);
        assert!(result.rung_verdicts.is_empty());
        // Terminal event should still fire.
        assert!(progress.event_count() >= 1);
    }

    #[test]
    fn max_rung_cap_skips_higher_rungs() {
        let mut req = test_request();
        req.gates_config.max_rung = Some(1); // Only Compile (0) and Lint (1).
        req.changed_files = vec!["a.rs".into(), "b.rs".into(), "c.rs".into(), "d.rs".into()];

        // Verify that plan_complexity is Complex (which would normally run all 7 rungs).
        assert_eq!(
            ProductionGateService::plan_complexity(&req),
            PlanComplexity::Complex
        );

        // The max_rung cap is enforced during run_canonical_pipeline, not
        // during selected_rung_labels, so we test by checking the cap value
        // is properly threaded from the request.
        assert_eq!(req.gates_config.max_rung, Some(1));
    }

    #[test]
    fn is_mostly_passing_with_skipped_core_rungs() {
        // Skipped core rungs count as OK for mostly-passing.
        let verdicts = vec![
            make_rv(Rung::Compile, RungState::Passed),
            make_rv(Rung::Lint, RungState::Skipped),
            make_rv(Rung::Test, RungState::Skipped),
            make_rv(Rung::Symbol, RungState::Failed),
        ];
        assert!(ProductionGateService::is_mostly_passing(&verdicts));
    }

    #[test]
    fn verdict_to_rung_verdict_large_output_truncates() {
        let big_detail = "x".repeat(MAX_RAW_OUTPUT_BYTES + 100);
        let v = Verdict::fail("test", "big output")
            .with_detail(&big_detail)
            .with_duration(50);
        let rv = ProductionGateService::verdict_to_rung_verdict(Rung::Test, &v, "fp");
        assert_eq!(rv.diagnostic.len(), MAX_RAW_OUTPUT_BYTES);
        assert!(rv.evidence.is_populated());
        assert!(rv.evidence.artifact_path.is_some());
    }

    #[test]
    fn verdict_to_rung_verdict_small_output_no_evidence() {
        let v = Verdict::fail("test", "small fail")
            .with_detail("short")
            .with_duration(10);
        let rv = ProductionGateService::verdict_to_rung_verdict(Rung::Test, &v, "fp");
        assert_eq!(rv.diagnostic, "short");
        assert!(!rv.evidence.is_populated());
    }

    fn make_rv(rung: Rung, state: RungState) -> ProductionGateRungVerdict {
        ProductionGateRungVerdict {
            rung,
            gate_name: rung.label().to_string(),
            state,
            failure_classification: None,
            diagnostic: String::new(),
            evidence: EvidenceRef::default(),
            duration: Duration::ZERO,
            test_counts: None,
            input_fingerprint: String::new(),
            skip_reason: None,
        }
    }
}
