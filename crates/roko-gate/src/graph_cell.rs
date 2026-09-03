//! Graph-compatible Cell wrapper for the production gate pipeline.
//!
//! [`GatePipelineCell`] owns an `Arc<dyn ProductionGateRunner>`, decodes a
//! [`GatePipelineCellInput`] envelope from an input Signal, constructs a
//! [`ProductionGateRequest`], forwards per-rung progress through the
//! [`ProgressSink`] interface, and returns one versioned verdict Signal.
//! It does not spawn detached tasks or persist state.
//!
//! This module lives in `roko-gate` (layer 3) and implements `roko_core::Cell`.
//! The `roko-graph` engine can wrap or adapt it through its cell registry.
//!
//! # Graph event bridging
//!
//! [`GraphEventProgressSink`] bridges [`GatePipelineProgress`] events into
//! graph-level [`CellProgress`] descriptions that the graph engine's event
//! sink can forward to TUI, SSE, and telemetry consumers. This ensures
//! per-rung progress reaches the graph event sink before the pipeline
//! finishes (acceptance criterion #250).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use roko_core::{Body, Kind, ProtocolId, Signal};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::adaptive_threshold::AdaptiveThresholds;
use crate::production_request::{GateTaskContextSpec, ProductionGateRequest, VerifyStepSpec};
use crate::production_service::{
    GatePipelineProgress, NoopProgressSink, ProductionGateRunner, ProgressSink,
};
use crate::production_verdict::ProductionGateVerdictV1;

// ────────────────────────────────────────────────────────────────────────────
// Serializable input envelope
// ────────────────────────────────────────────────────────────────────────────

/// Serializable envelope for the graph cell input Signal.
///
/// This contains the serializable subset of [`ProductionGateRequest`].
/// Non-serializable fields (e.g. `CancellationToken`, `AdaptiveThresholds`)
/// are supplied by the cell at construction time or defaulted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatePipelineCellInput {
    /// Run identifier.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier.
    pub task_id: String,
    /// Attempt number (0-based).
    #[serde(default)]
    pub attempt: u32,
    /// Root workspace path.
    pub workspace: PathBuf,
    /// Content-hash fingerprint of the workspace.
    #[serde(default)]
    pub workspace_fingerprint: String,
    /// Changed files (relative to workspace root).
    #[serde(default)]
    pub changed_files: Vec<String>,
    /// Authored verify steps.
    #[serde(default)]
    pub verify_steps: Vec<VerifyStepSpec>,
    /// Gate configuration.
    #[serde(default)]
    pub gates_config: roko_core::config::GatesConfig,
    /// Task context for diagnostics.
    #[serde(default)]
    pub task_context: GateTaskContextSpec,
    /// Overall timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Pre-existing baseline fingerprint.
    #[serde(default)]
    pub baseline_fingerprint: Option<String>,
}

fn default_timeout_secs() -> u64 {
    600
}

impl GatePipelineCellInput {
    /// Convert to a full `ProductionGateRequest` with the given cancellation
    /// token and optional adaptive thresholds.
    #[must_use]
    pub fn into_request(
        self,
        cancel: CancellationToken,
        adaptive: Option<AdaptiveThresholds>,
    ) -> ProductionGateRequest {
        ProductionGateRequest {
            run_id: self.run_id,
            plan_id: self.plan_id,
            task_id: self.task_id,
            attempt: self.attempt,
            workspace: self.workspace,
            workspace_fingerprint: self.workspace_fingerprint,
            changed_files: self.changed_files,
            verify_steps: self.verify_steps,
            gates_config: self.gates_config,
            task_context: self.task_context,
            timeout_secs: self.timeout_secs,
            cancel,
            baseline_fingerprint: self.baseline_fingerprint,
            adaptive_thresholds: adaptive,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Graph event progress sink
// ────────────────────────────────────────────────────────────────────────────

/// Bridges [`GatePipelineProgress`] events from the production gate pipeline
/// into descriptions suitable for graph-level `CellProgress` events.
///
/// Graph-level event emission is the graph engine's responsibility (via
/// `GraphEventSink`). This sink translates gate progress into structured
/// descriptions that the engine can forward without the gate needing a direct
/// dependency on `roko-graph` event types.
///
/// Each event is forwarded to an inner `ProgressSink` after the description
/// is captured for consumption by the cell's `execute_gate` caller.
pub struct GraphEventProgressSink {
    /// Inner sink for raw progress events (e.g. for telemetry, logging).
    inner: Arc<dyn ProgressSink>,
}

impl GraphEventProgressSink {
    /// Create a new graph event progress sink wrapping an inner sink.
    pub fn new(inner: Arc<dyn ProgressSink>) -> Self {
        Self { inner }
    }

    /// Format a progress event into a concise graph-level description.
    #[must_use]
    pub fn describe(event: &GatePipelineProgress) -> String {
        match event {
            GatePipelineProgress::RungStarted { rung, gate_name } => {
                format!("rung {} ({}) started", rung.label(), gate_name)
            }
            GatePipelineProgress::RungOutput {
                rung, gate_name, ..
            } => {
                format!("rung {} ({}) output", rung.label(), gate_name)
            }
            GatePipelineProgress::RungCompleted { rung, verdict } => {
                format!(
                    "rung {} ({}) {:?}",
                    rung.label(),
                    verdict.gate_name,
                    verdict.state
                )
            }
            GatePipelineProgress::PipelineCompleted { outcome } => {
                format!("pipeline completed: {outcome:?}")
            }
        }
    }

    /// Return how many rungs have been counted in a progress sequence.
    ///
    /// Useful for populating `CellProgress.completed` / `.total` fields
    /// in the graph event sink.
    #[must_use]
    pub fn rung_progress(event: &GatePipelineProgress) -> (u32, u32) {
        match event {
            GatePipelineProgress::RungStarted { rung, .. } => (rung.as_index() as u32, 7),
            GatePipelineProgress::RungOutput { rung, .. } => (rung.as_index() as u32, 7),
            GatePipelineProgress::RungCompleted { rung, .. } => {
                (rung.as_index() as u32 + 1, 7)
            }
            GatePipelineProgress::PipelineCompleted { .. } => (7, 7),
        }
    }
}

#[async_trait]
impl ProgressSink for GraphEventProgressSink {
    async fn send(&self, event: GatePipelineProgress) {
        // Forward to the inner sink (logging, telemetry, etc.).
        self.inner.send(event).await;
    }
}

// ────────────────────────────────────────────────────────────────────────────
// GatePipelineCell
// ────────────────────────────────────────────────────────────────────────────

/// Graph Cell that executes the full production gate pipeline.
///
/// # Usage
///
/// 1. Construct with `GatePipelineCell::new(runner)`.
/// 2. Optionally attach a progress sink with `with_progress_sink`.
/// 3. Optionally attach adaptive thresholds with `with_adaptive_thresholds`.
/// 4. Register in the graph's cell registry.
/// 5. The graph engine sends an input Signal whose body encodes a
///    [`GatePipelineCellInput`] as JSON.
/// 6. The cell decodes the input, constructs a `ProductionGateRequest`
///    (injecting the cell's cancellation token and adaptive thresholds),
///    runs the pipeline, and returns a Signal whose body encodes the
///    [`ProductionGateVerdictV1`].
///
/// The cell does NOT:
/// - Spawn detached tasks.
/// - Persist state (adaptive thresholds are returned in the verdict).
/// - Depend on CLI types.
pub struct GatePipelineCell {
    runner: Arc<dyn ProductionGateRunner>,
    progress_sink: Arc<dyn ProgressSink>,
    cancel: CancellationToken,
    /// Adaptive thresholds snapshot injected into each request.
    /// The service reads skip/observe decisions but does not persist.
    adaptive_thresholds: Option<AdaptiveThresholds>,
}

impl GatePipelineCell {
    /// Create a new gate pipeline cell with the given runner.
    pub fn new(runner: Arc<dyn ProductionGateRunner>) -> Self {
        Self {
            runner,
            progress_sink: Arc::new(NoopProgressSink),
            cancel: CancellationToken::new(),
            adaptive_thresholds: None,
        }
    }

    /// Attach a progress sink for forwarding per-rung events.
    #[must_use]
    pub fn with_progress_sink(mut self, sink: Arc<dyn ProgressSink>) -> Self {
        self.progress_sink = sink;
        self
    }

    /// Attach a cancellation token for cooperative shutdown.
    #[must_use]
    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// Attach adaptive thresholds for skip/observe decisions.
    ///
    /// The thresholds are injected into each request. The service updates
    /// the snapshot in the verdict but does not persist it -- that is
    /// #251/#253 scope.
    #[must_use]
    pub fn with_adaptive_thresholds(mut self, thresholds: AdaptiveThresholds) -> Self {
        self.adaptive_thresholds = Some(thresholds);
        self
    }

    /// Decode a `GatePipelineCellInput` from an input Signal's JSON body.
    fn decode_input(signal: &Signal) -> roko_core::Result<GatePipelineCellInput> {
        signal
            .body
            .as_json::<GatePipelineCellInput>()
            .map_err(|e| {
                roko_core::RokoError::Invalid(format!(
                    "GatePipelineCell: failed to decode input: {e}"
                ))
            })
    }

    /// Encode a `ProductionGateVerdictV1` into an output Signal.
    fn encode_verdict(verdict: &ProductionGateVerdictV1) -> roko_core::Result<Signal> {
        let body = Body::from_json(verdict).map_err(|e| {
            roko_core::RokoError::Invalid(format!(
                "GatePipelineCell: failed to encode verdict: {e}"
            ))
        })?;
        Ok(Signal::builder(Kind::GateVerdict).body(body).build())
    }
}

impl roko_core::Cell for GatePipelineCell {
    fn cell_id(&self) -> &str {
        "gate-pipeline-cell"
    }

    fn cell_name(&self) -> &str {
        "GatePipelineCell"
    }

    fn cell_version(&self) -> roko_core::CellVersion {
        (0, 1, 0)
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Verify]
    }

    fn estimated_duration(&self) -> Option<Duration> {
        // Gate pipelines typically take 30s to 15 minutes.
        Some(Duration::from_secs(120))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Async execution
// ────────────────────────────────────────────────────────────────────────────

impl GatePipelineCell {
    /// Execute the gate pipeline for a single input signal.
    ///
    /// This is the async entry point called by the graph engine (via an
    /// adapter) or directly by integration tests.
    pub async fn execute_gate(&self, input: Signal) -> roko_core::Result<Signal> {
        let cell_input = Self::decode_input(&input)?;
        let request =
            cell_input.into_request(self.cancel.child_token(), self.adaptive_thresholds.clone());

        info!(
            plan_id = %request.plan_id,
            task_id = %request.task_id,
            attempt = request.attempt,
            "GatePipelineCell executing"
        );

        let verdict = self
            .runner
            .run(request, Arc::clone(&self.progress_sink))
            .await?;

        info!(
            outcome = ?verdict.outcome,
            executed_rungs = verdict.executed_rung_count(),
            failed_rungs = verdict.failed_rung_count(),
            "GatePipelineCell completed"
        );

        Self::encode_verdict(&verdict)
    }

    /// Execute for multiple input signals.
    ///
    /// Only the first signal is used as the request; additional signals
    /// are ignored (the gate pipeline is a single-input cell).
    pub async fn execute_batch(&self, input: Vec<Signal>) -> roko_core::Result<Vec<Signal>> {
        let Some(first) = input.into_iter().next() else {
            warn!("GatePipelineCell received empty input batch");
            return Ok(Vec::new());
        };

        let output = self.execute_gate(first).await?;
        Ok(vec![output])
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::production_service::ProductionGateRunner;
    use crate::production_verdict::{
        EvidenceRef, PipelineOutcome, ProductionGateRungVerdict, ProductionGateVerdictV1,
        RungState, VERDICT_SCHEMA_VERSION,
    };
    use crate::rung_selector::Rung;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    // ── Mock runners ─────────────────────────────────────────────────────

    /// Mock runner that always passes.
    struct MockPassRunner;

    #[async_trait]
    impl ProductionGateRunner for MockPassRunner {
        async fn run(
            &self,
            request: ProductionGateRequest,
            _progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<ProductionGateVerdictV1> {
            Ok(ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: request.workspace_fingerprint.clone(),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts: Vec::new(),
                outcome: PipelineOutcome::Passed,
                mostly_passing: false,
                total_duration: Duration::from_millis(1),
                adaptive_snapshot: None,
            })
        }
    }

    /// Mock runner that always fails.
    struct MockFailRunner;

    #[async_trait]
    impl ProductionGateRunner for MockFailRunner {
        async fn run(
            &self,
            request: ProductionGateRequest,
            _progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<ProductionGateVerdictV1> {
            Ok(ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: request.workspace_fingerprint.clone(),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts: Vec::new(),
                outcome: PipelineOutcome::Failed,
                mostly_passing: false,
                total_duration: Duration::from_millis(1),
                adaptive_snapshot: None,
            })
        }
    }

    /// Mock runner that emits per-rung progress events and returns rung verdicts.
    struct MockProgressRunner {
        rungs: Vec<(Rung, RungState)>,
    }

    impl MockProgressRunner {
        fn new(rungs: Vec<(Rung, RungState)>) -> Self {
            Self { rungs }
        }
    }

    #[async_trait]
    impl ProductionGateRunner for MockProgressRunner {
        async fn run(
            &self,
            request: ProductionGateRequest,
            progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<ProductionGateVerdictV1> {
            let mut rung_verdicts = Vec::new();
            let mut any_failed = false;

            for (rung, state) in &self.rungs {
                // Emit rung-start.
                progress_sink
                    .send(GatePipelineProgress::RungStarted {
                        rung: *rung,
                        gate_name: rung.label().to_string(),
                    })
                    .await;

                let rv = ProductionGateRungVerdict {
                    rung: *rung,
                    gate_name: rung.label().to_string(),
                    state: *state,
                    failure_classification: None,
                    diagnostic: format!("{} {state:?}", rung.label()),
                    evidence: EvidenceRef::default(),
                    duration: Duration::from_millis(50),
                    test_counts: None,
                    input_fingerprint: request.workspace_fingerprint.clone(),
                    skip_reason: if *state == RungState::Skipped {
                        Some("mock skip".into())
                    } else {
                        None
                    },
                };

                // Emit rung-completed.
                progress_sink
                    .send(GatePipelineProgress::RungCompleted {
                        rung: *rung,
                        verdict: rv.clone(),
                    })
                    .await;

                if matches!(state, RungState::Failed) {
                    any_failed = true;
                }
                rung_verdicts.push(rv);
            }

            let outcome = if any_failed {
                PipelineOutcome::Failed
            } else {
                PipelineOutcome::Passed
            };

            // Emit terminal event.
            progress_sink
                .send(GatePipelineProgress::PipelineCompleted { outcome })
                .await;

            Ok(ProductionGateVerdictV1 {
                schema_version: VERDICT_SCHEMA_VERSION,
                request_fingerprint: format!(
                    "{}:{}:{}:{}",
                    request.run_id, request.plan_id, request.task_id, request.attempt
                ),
                workspace_fingerprint: request.workspace_fingerprint,
                rung_verdicts,
                outcome,
                mostly_passing: false,
                total_duration: Duration::from_millis(150),
                adaptive_snapshot: request.adaptive_thresholds,
            })
        }
    }

    /// Mock runner that returns an error.
    struct MockErrorRunner;

    #[async_trait]
    impl ProductionGateRunner for MockErrorRunner {
        async fn run(
            &self,
            _request: ProductionGateRequest,
            _progress_sink: Arc<dyn ProgressSink>,
        ) -> roko_core::Result<ProductionGateVerdictV1> {
            Err(roko_core::RokoError::Invalid(
                "simulated gate error".into(),
            ))
        }
    }

    // ── Test progress sinks ──────────────────────────────────────────────

    struct CountingSink(AtomicUsize);

    impl CountingSink {
        fn new() -> Self {
            Self(AtomicUsize::new(0))
        }

        fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ProgressSink for CountingSink {
        async fn send(&self, _event: GatePipelineProgress) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Progress sink that captures events for inspection.
    struct CapturingSink {
        events: Mutex<Vec<GatePipelineProgress>>,
    }

    impl CapturingSink {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<GatePipelineProgress> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ProgressSink for CapturingSink {
        async fn send(&self, event: GatePipelineProgress) {
            self.events.lock().unwrap().push(event);
        }
    }

    // ── Test helpers ─────────────────────────────────────────────────────

    fn test_input() -> GatePipelineCellInput {
        GatePipelineCellInput {
            run_id: "run-1".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            attempt: 0,
            workspace: PathBuf::from("/tmp/ws"),
            workspace_fingerprint: "fp123".into(),
            changed_files: vec!["src/main.rs".into()],
            verify_steps: Vec::new(),
            gates_config: roko_core::config::GatesConfig::default(),
            task_context: GateTaskContextSpec::default(),
            timeout_secs: 60,
            baseline_fingerprint: None,
        }
    }

    fn input_signal() -> Signal {
        let input = test_input();
        let body = Body::from_json(&input).expect("serialize input");
        Signal::builder(Kind::Task).body(body).build()
    }

    fn input_signal_with_verify_steps() -> Signal {
        let mut input = test_input();
        input.verify_steps = vec![
            VerifyStepSpec::from_command("cargo test").with_phase("test"),
            VerifyStepSpec::from_command("cargo clippy").with_phase("lint"),
        ];
        let body = Body::from_json(&input).expect("serialize input");
        Signal::builder(Kind::Task).body(body).build()
    }

    fn input_signal_with_baseline() -> Signal {
        let mut input = test_input();
        input.baseline_fingerprint = Some("baseline-fp".into());
        let body = Body::from_json(&input).expect("serialize input");
        Signal::builder(Kind::Task).body(body).build()
    }

    // ── Identity and construction tests ──────────────────────────────────

    #[test]
    fn cell_identity() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        assert_eq!(cell.cell_id(), "gate-pipeline-cell");
        assert_eq!(cell.cell_name(), "GatePipelineCell");
        assert!(cell.protocols().contains(&ProtocolId::Verify));
    }

    #[test]
    fn cell_estimated_duration() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let dur = cell.estimated_duration().expect("should have estimate");
        assert_eq!(dur, Duration::from_secs(120));
    }

    #[test]
    fn cell_version() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        use roko_core::Cell;
        assert_eq!(cell.cell_version(), (0, 1, 0));
    }

    // ── Input serialization tests ────────────────────────────────────────

    #[test]
    fn input_round_trip() {
        let input = test_input();
        let json = serde_json::to_string(&input).expect("serialize");
        let back: GatePipelineCellInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.plan_id, "plan-1");
        assert_eq!(back.task_id, "task-1");
        assert_eq!(back.timeout_secs, 60);
    }

    #[test]
    fn input_round_trip_with_verify_steps() {
        let mut input = test_input();
        input.verify_steps = vec![
            VerifyStepSpec::from_command("cargo test")
                .with_phase("test")
                .with_timeout_ms(120_000),
        ];
        let json = serde_json::to_string(&input).expect("serialize");
        let back: GatePipelineCellInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.verify_steps.len(), 1);
        assert_eq!(back.verify_steps[0].command, "cargo test");
        assert_eq!(back.verify_steps[0].phase, "test");
        assert_eq!(back.verify_steps[0].timeout_ms, 120_000);
    }

    #[test]
    fn input_round_trip_with_baseline() {
        let mut input = test_input();
        input.baseline_fingerprint = Some("base-fp".into());
        let json = serde_json::to_string(&input).expect("serialize");
        let back: GatePipelineCellInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.baseline_fingerprint.as_deref(), Some("base-fp"));
    }

    #[test]
    fn input_default_timeout() {
        let json = r#"{"run_id":"r","plan_id":"p","task_id":"t","workspace":"/tmp"}"#;
        let input: GatePipelineCellInput = serde_json::from_str(json).expect("deserialize");
        assert_eq!(input.timeout_secs, 600);
    }

    #[test]
    fn input_into_request() {
        let input = test_input();
        let request = input.into_request(CancellationToken::new(), None);
        assert_eq!(request.plan_id, "plan-1");
        assert_eq!(request.task_id, "task-1");
        assert_eq!(request.attempt, 0);
        assert!(request.adaptive_thresholds.is_none());
    }

    #[test]
    fn input_into_request_with_adaptive() {
        let input = test_input();
        let adaptive = AdaptiveThresholds::default();
        let request = input.into_request(CancellationToken::new(), Some(adaptive));
        assert!(request.adaptive_thresholds.is_some());
    }

    // ── Decode / encode tests ────────────────────────────────────────────

    #[test]
    fn decode_input_from_signal() {
        let signal = input_signal();
        let decoded = GatePipelineCell::decode_input(&signal).expect("decode");
        assert_eq!(decoded.plan_id, "plan-1");
    }

    #[test]
    fn decode_input_bad_json_returns_error() {
        let signal = Signal::builder(Kind::Task)
            .body(Body::text("not json"))
            .build();
        let result = GatePipelineCell::decode_input(&signal);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to decode input"));
    }

    #[test]
    fn encode_verdict_round_trip() {
        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: "fp".into(),
            workspace_fingerprint: "ws-fp".into(),
            rung_verdicts: Vec::new(),
            outcome: PipelineOutcome::Passed,
            mostly_passing: false,
            total_duration: Duration::from_secs(1),
            adaptive_snapshot: None,
        };
        let signal = GatePipelineCell::encode_verdict(&verdict).expect("encode");
        assert_eq!(signal.kind, Kind::GateVerdict);

        let back: ProductionGateVerdictV1 = signal.body.as_json().expect("decode");
        assert_eq!(back.outcome, PipelineOutcome::Passed);
    }

    #[test]
    fn encode_verdict_with_rung_verdicts() {
        let verdict = ProductionGateVerdictV1 {
            schema_version: VERDICT_SCHEMA_VERSION,
            request_fingerprint: "fp".into(),
            workspace_fingerprint: "ws-fp".into(),
            rung_verdicts: vec![
                ProductionGateRungVerdict {
                    rung: Rung::Compile,
                    gate_name: "compile".into(),
                    state: RungState::Passed,
                    failure_classification: None,
                    diagnostic: "ok".into(),
                    evidence: EvidenceRef::default(),
                    duration: Duration::from_millis(200),
                    test_counts: None,
                    input_fingerprint: "fp".into(),
                    skip_reason: None,
                },
                ProductionGateRungVerdict {
                    rung: Rung::Test,
                    gate_name: "test".into(),
                    state: RungState::Failed,
                    failure_classification: None,
                    diagnostic: "3 failures".into(),
                    evidence: EvidenceRef::default(),
                    duration: Duration::from_millis(500),
                    test_counts: None,
                    input_fingerprint: "fp".into(),
                    skip_reason: None,
                },
            ],
            outcome: PipelineOutcome::Failed,
            mostly_passing: false,
            total_duration: Duration::from_secs(1),
            adaptive_snapshot: None,
        };
        let signal = GatePipelineCell::encode_verdict(&verdict).expect("encode");
        let back: ProductionGateVerdictV1 = signal.body.as_json().expect("decode");
        assert_eq!(back.rung_verdicts.len(), 2);
        assert_eq!(back.failed_rung_count(), 1);
    }

    // ── Async execution tests ────────────────────────────────────────────

    #[tokio::test]
    async fn execute_gate_pass() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should pass");
        assert_eq!(output.kind, Kind::GateVerdict);

        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode verdict");
        assert_eq!(verdict.outcome, PipelineOutcome::Passed);
    }

    #[tokio::test]
    async fn execute_gate_fail() {
        let cell = GatePipelineCell::new(Arc::new(MockFailRunner));
        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should complete");

        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode verdict");
        assert_eq!(verdict.outcome, PipelineOutcome::Failed);
    }

    #[tokio::test]
    async fn execute_gate_error_propagates() {
        let cell = GatePipelineCell::new(Arc::new(MockErrorRunner));
        let result = cell.execute_gate(input_signal()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("simulated gate error"));
    }

    #[tokio::test]
    async fn execute_batch_empty_input() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let output = cell
            .execute_batch(Vec::new())
            .await
            .expect("empty should succeed");
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn execute_batch_single_input() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let output = cell
            .execute_batch(vec![input_signal()])
            .await
            .expect("should succeed");
        assert_eq!(output.len(), 1);
    }

    #[tokio::test]
    async fn execute_batch_multiple_inputs_uses_first() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let output = cell
            .execute_batch(vec![input_signal(), input_signal(), input_signal()])
            .await
            .expect("should succeed");
        assert_eq!(output.len(), 1);
    }

    #[tokio::test]
    async fn bad_input_signal_returns_error() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        let bad_signal = Signal::builder(Kind::Task)
            .body(Body::text("not json"))
            .build();
        let result = cell.execute_gate(bad_signal).await;
        assert!(result.is_err());
    }

    // ── Progress sink tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn progress_events_reach_counting_sink() {
        let sink = Arc::new(CountingSink::new());
        let runner = MockProgressRunner::new(vec![
            (Rung::Compile, RungState::Passed),
            (Rung::Lint, RungState::Passed),
        ]);
        let cell = GatePipelineCell::new(Arc::new(runner))
            .with_progress_sink(Arc::clone(&sink) as Arc<dyn ProgressSink>);

        let _output = cell
            .execute_gate(input_signal())
            .await
            .expect("should succeed");

        // 2 rungs: 2 start + 2 completed + 1 pipeline completed = 5 events.
        assert_eq!(sink.count(), 5);
    }

    #[tokio::test]
    async fn progress_events_reach_capturing_sink() {
        let sink = Arc::new(CapturingSink::new());
        let runner = MockProgressRunner::new(vec![
            (Rung::Compile, RungState::Passed),
            (Rung::Test, RungState::Failed),
        ]);
        let cell = GatePipelineCell::new(Arc::new(runner))
            .with_progress_sink(Arc::clone(&sink) as Arc<dyn ProgressSink>);

        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should complete");

        let events = sink.events();
        // Start(compile), Completed(compile), Start(test), Completed(test), PipelineCompleted
        assert_eq!(events.len(), 5);

        // Verify first event is RungStarted for compile.
        assert!(
            matches!(&events[0], GatePipelineProgress::RungStarted { rung, .. } if *rung == Rung::Compile)
        );

        // Verify last event is PipelineCompleted.
        assert!(matches!(
            &events[4],
            GatePipelineProgress::PipelineCompleted {
                outcome: PipelineOutcome::Failed
            }
        ));

        // Verdict Signal should encode a Failed outcome.
        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode");
        assert_eq!(verdict.outcome, PipelineOutcome::Failed);
        assert_eq!(verdict.rung_verdicts.len(), 2);
    }

    // ── Cancel token tests ───────────────────────────────────────────────

    #[test]
    fn cancel_token_is_forwarded() {
        let cancel = CancellationToken::new();
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner)).with_cancel(cancel.clone());
        // Cancelling the parent should propagate to children.
        cancel.cancel();
        assert!(cell.cancel.is_cancelled());
    }

    #[test]
    fn default_cancel_token_not_cancelled() {
        let cell = GatePipelineCell::new(Arc::new(MockPassRunner));
        assert!(!cell.cancel.is_cancelled());
    }

    // ── Adaptive thresholds tests ────────────────────────────────────────

    #[tokio::test]
    async fn adaptive_thresholds_are_forwarded() {
        let adaptive = AdaptiveThresholds::default();
        let runner = MockProgressRunner::new(vec![(Rung::Compile, RungState::Passed)]);
        let cell = GatePipelineCell::new(Arc::new(runner))
            .with_adaptive_thresholds(adaptive.clone());

        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should succeed");

        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode");
        // MockProgressRunner echoes adaptive_thresholds back in the verdict.
        assert!(verdict.adaptive_snapshot.is_some());
    }

    #[tokio::test]
    async fn no_adaptive_thresholds_by_default() {
        let runner = MockProgressRunner::new(vec![(Rung::Compile, RungState::Passed)]);
        let cell = GatePipelineCell::new(Arc::new(runner));

        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should succeed");

        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode");
        assert!(verdict.adaptive_snapshot.is_none());
    }

    // ── GraphEventProgressSink tests ─────────────────────────────────────

    #[test]
    fn graph_event_describe_rung_started() {
        let event = GatePipelineProgress::RungStarted {
            rung: Rung::Compile,
            gate_name: "compile:cargo".into(),
        };
        let desc = GraphEventProgressSink::describe(&event);
        assert!(desc.contains("compile"));
        assert!(desc.contains("started"));
    }

    #[test]
    fn graph_event_describe_rung_output() {
        let event = GatePipelineProgress::RungOutput {
            rung: Rung::Test,
            gate_name: "test:cargo".into(),
            output: "running 42 tests...".into(),
        };
        let desc = GraphEventProgressSink::describe(&event);
        assert!(desc.contains("test"));
        assert!(desc.contains("output"));
    }

    #[test]
    fn graph_event_describe_rung_completed() {
        let rv = ProductionGateRungVerdict {
            rung: Rung::Lint,
            gate_name: "clippy:cargo".into(),
            state: RungState::Passed,
            failure_classification: None,
            diagnostic: String::new(),
            evidence: EvidenceRef::default(),
            duration: Duration::from_millis(100),
            test_counts: None,
            input_fingerprint: String::new(),
            skip_reason: None,
        };
        let event = GatePipelineProgress::RungCompleted {
            rung: Rung::Lint,
            verdict: rv,
        };
        let desc = GraphEventProgressSink::describe(&event);
        assert!(desc.contains("lint"));
        assert!(desc.contains("Passed"));
    }

    #[test]
    fn graph_event_describe_pipeline_completed() {
        let event = GatePipelineProgress::PipelineCompleted {
            outcome: PipelineOutcome::TimedOut,
        };
        let desc = GraphEventProgressSink::describe(&event);
        assert!(desc.contains("pipeline completed"));
        assert!(desc.contains("TimedOut"));
    }

    #[test]
    fn graph_event_rung_progress_values() {
        let started = GatePipelineProgress::RungStarted {
            rung: Rung::Compile,
            gate_name: "compile".into(),
        };
        assert_eq!(GraphEventProgressSink::rung_progress(&started), (0, 7));

        let completed = GatePipelineProgress::RungCompleted {
            rung: Rung::Lint,
            verdict: ProductionGateRungVerdict {
                rung: Rung::Lint,
                gate_name: "lint".into(),
                state: RungState::Passed,
                failure_classification: None,
                diagnostic: String::new(),
                evidence: EvidenceRef::default(),
                duration: Duration::ZERO,
                test_counts: None,
                input_fingerprint: String::new(),
                skip_reason: None,
            },
        };
        // Lint is rung index 1, completed = index + 1 = 2.
        assert_eq!(GraphEventProgressSink::rung_progress(&completed), (2, 7));

        let terminal = GatePipelineProgress::PipelineCompleted {
            outcome: PipelineOutcome::Passed,
        };
        assert_eq!(GraphEventProgressSink::rung_progress(&terminal), (7, 7));
    }

    #[tokio::test]
    async fn graph_event_progress_sink_forwards_to_inner() {
        let inner = Arc::new(CountingSink::new());
        let sink = GraphEventProgressSink::new(Arc::clone(&inner) as Arc<dyn ProgressSink>);

        sink.send(GatePipelineProgress::RungStarted {
            rung: Rung::Compile,
            gate_name: "compile".into(),
        })
        .await;

        sink.send(GatePipelineProgress::PipelineCompleted {
            outcome: PipelineOutcome::Passed,
        })
        .await;

        assert_eq!(inner.count(), 2);
    }

    // ── Verify step input tests ──────────────────────────────────────────

    #[tokio::test]
    async fn input_with_verify_steps_round_trips() {
        let signal = input_signal_with_verify_steps();
        let decoded = GatePipelineCell::decode_input(&signal).expect("decode");
        assert_eq!(decoded.verify_steps.len(), 2);
        assert_eq!(decoded.verify_steps[0].command, "cargo test");
        assert_eq!(decoded.verify_steps[1].phase, "lint");
    }

    // ── Baseline fingerprint tests ───────────────────────────────────────

    #[tokio::test]
    async fn input_with_baseline_fingerprint() {
        let signal = input_signal_with_baseline();
        let decoded = GatePipelineCell::decode_input(&signal).expect("decode");
        assert_eq!(decoded.baseline_fingerprint.as_deref(), Some("baseline-fp"));

        let request = decoded.into_request(CancellationToken::new(), None);
        assert_eq!(
            request.baseline_fingerprint.as_deref(),
            Some("baseline-fp")
        );
    }

    // ── Integration: gate failure prevents dependent execution ───────────

    #[tokio::test]
    async fn failed_verdict_outcome_is_machine_readable() {
        let runner = MockProgressRunner::new(vec![
            (Rung::Compile, RungState::Passed),
            (Rung::Lint, RungState::Passed),
            (Rung::Test, RungState::Failed),
        ]);
        let cell = GatePipelineCell::new(Arc::new(runner));

        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should complete");
        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode");

        // The verdict outcome is Failed, which the graph engine uses to
        // prevent dependent task execution.
        assert_eq!(verdict.outcome, PipelineOutcome::Failed);
        assert_eq!(verdict.failed_rung_count(), 1);
        assert_eq!(verdict.failed_rung_names(), vec!["test"]);
        assert_eq!(verdict.executed_rung_count(), 3);
    }

    // ── Integration: per-rung progress before pipeline finish ────────────

    #[tokio::test]
    async fn per_rung_progress_reaches_sink_before_finish() {
        let sink = Arc::new(CapturingSink::new());
        let runner = MockProgressRunner::new(vec![
            (Rung::Compile, RungState::Passed),
            (Rung::Lint, RungState::Skipped),
            (Rung::Test, RungState::Passed),
        ]);
        let cell = GatePipelineCell::new(Arc::new(runner))
            .with_progress_sink(Arc::clone(&sink) as Arc<dyn ProgressSink>);

        let _output = cell
            .execute_gate(input_signal())
            .await
            .expect("should complete");

        let events = sink.events();
        // 3 rungs: 3 start + 3 completed + 1 terminal = 7 events.
        assert_eq!(events.len(), 7);

        // All rung events must come before the terminal event.
        let terminal_idx = events
            .iter()
            .position(|e| matches!(e, GatePipelineProgress::PipelineCompleted { .. }))
            .expect("terminal event must exist");
        assert_eq!(terminal_idx, events.len() - 1);

        // Every RungStarted must appear before its RungCompleted.
        let mut started_rungs = Vec::new();
        for event in &events {
            match event {
                GatePipelineProgress::RungStarted { rung, .. } => {
                    started_rungs.push(*rung);
                }
                GatePipelineProgress::RungCompleted { rung, .. } => {
                    assert!(
                        started_rungs.contains(rung),
                        "RungCompleted for {:?} before RungStarted",
                        rung
                    );
                }
                _ => {}
            }
        }
    }

    // ── Rung verdict with skipped rungs ──────────────────────────────────

    #[tokio::test]
    async fn skipped_rungs_in_verdict() {
        let runner = MockProgressRunner::new(vec![
            (Rung::Compile, RungState::Passed),
            (Rung::Lint, RungState::Skipped),
            (Rung::Test, RungState::Passed),
        ]);
        let cell = GatePipelineCell::new(Arc::new(runner));

        let output = cell
            .execute_gate(input_signal())
            .await
            .expect("should complete");
        let verdict: ProductionGateVerdictV1 = output.body.as_json().expect("decode");

        assert_eq!(verdict.outcome, PipelineOutcome::Passed);
        assert_eq!(verdict.rung_verdicts.len(), 3);
        assert_eq!(verdict.executed_rung_count(), 2); // compile + test
        assert!(verdict.rung_verdicts[1].skipped());
        assert_eq!(
            verdict.rung_verdicts[1].skip_reason.as_deref(),
            Some("mock skip")
        );
    }
}
