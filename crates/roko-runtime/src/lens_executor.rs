//! Routed execution of event-oriented telemetry Lenses.
//!
//! [`LensExecutor`] binds the declarative routing table from `roko-core` to
//! named [`TelemetryObserve`] implementations. Raw-event fan-out is concurrent;
//! downstream Lens chains are completed before the event cycle returns. Lens
//! failures are reported but isolated from the observed execution path.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use roko_core::dashboard_snapshot::{DashboardEvent, DiagnosisSeverity, DiagnosisSummary};
use roko_core::lens_circuit_breaker::{LensBreakerAction, LensBreakerStage, LensCircuitBreaker};
use roko_core::{
    CostReportPayload, LensRegistration, LensRegistry, LensScope, ObservableEvent,
    ObservableEventKind, Result, RokoError, Signal, TelemetryEventSink, TelemetryObserve,
};
use tokio::sync::Notify;
use tokio::task::JoinSet;

use crate::{
    AnomalyLens, CollectiveIntelligenceLens, EfficiencyLens, LatencyLens, LensOperatorStatus,
    LensPayload, LensQueueSnapshot, LensRuntimeControl, LensRuntimeSnapshot, LensSignalEnvelope,
    QualityLens, StateHubSender, TelemetryProjectionAggregator, TelemetryProjectionError,
    TrendLens, UsageLens, create_builtin_health_lens,
};

/// Circuit-breaker policy applied independently to every registered Lens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LensBreakerConfig {
    /// Maximum Lens time as a fraction of the observed operation time.
    pub overhead_budget_pct: f64,
    /// Consecutive violations before deterministic 50% sampling begins.
    pub sample_threshold: u32,
    /// Consecutive violations before the Lens is disabled.
    pub disable_threshold: u32,
}

impl Default for LensBreakerConfig {
    fn default() -> Self {
        Self {
            overhead_budget_pct: 0.01,
            sample_threshold: 3,
            disable_threshold: 10,
        }
    }
}

impl LensBreakerConfig {
    fn validate(self) -> Result<Self> {
        if !self.overhead_budget_pct.is_finite() || self.overhead_budget_pct < 0.0 {
            return Err(RokoError::config(
                "lens overhead budget must be a finite non-negative fraction",
            ));
        }
        if self.sample_threshold == 0 || self.disable_threshold <= self.sample_threshold {
            return Err(RokoError::config(
                "lens breaker thresholds require 0 < sample < disable",
            ));
        }
        Ok(self)
    }

    fn breaker(self) -> LensCircuitBreaker {
        LensCircuitBreaker::new(self.overhead_budget_pct)
            .with_thresholds(self.sample_threshold, self.disable_threshold)
    }
}

/// Backpressure policy for the passive telemetry delivery queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LensBackpressurePolicy {
    /// Retain the newest observation and evict the oldest queued observation.
    #[default]
    DropOldest,
}

impl LensBackpressurePolicy {
    const fn label(self) -> &'static str {
        match self {
            Self::DropOldest => "drop_oldest",
        }
    }
}

/// Configuration for non-blocking Lens event delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LensQueueConfig {
    /// Maximum observations waiting behind the active dispatch.
    pub capacity: usize,
    /// Overflow behavior. Observation topics use drop-oldest by specification.
    pub backpressure: LensBackpressurePolicy,
}

impl Default for LensQueueConfig {
    fn default() -> Self {
        Self {
            capacity: 1_024,
            backpressure: LensBackpressurePolicy::DropOldest,
        }
    }
}

impl LensQueueConfig {
    fn validate(self) -> Result<Self> {
        if self.capacity == 0 {
            return Err(RokoError::config(
                "Lens delivery queue capacity must be greater than zero",
            ));
        }
        Ok(self)
    }
}

/// Immediate result of a non-blocking observation enqueue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LensEnqueueOutcome {
    /// Observation was appended without evicting another pending event.
    Accepted,
    /// Queue was full, so its oldest pending observation was evicted.
    ReplacedOldest,
}

/// Result of one routed Lens invocation (or breaker-enforced omission).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensExecutionOutcome {
    /// The Lens completed normally.
    Succeeded,
    /// The Lens returned a typed runtime error.
    Failed(String),
    /// This event was omitted by the breaker's deterministic 50% sampler.
    SampledOut,
    /// The breaker has disabled this Lens.
    Disabled,
    /// A declarative registration has no bound implementation.
    MissingImplementation,
    /// The spawned Lens task panicked or was cancelled.
    TaskFailed(String),
}

/// Observable execution and circuit-breaker transition for one Lens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensExecutionRecord {
    /// Declarative Lens instance name.
    pub lens: String,
    /// Upstream Lens name for chained delivery; `None` for raw events.
    pub source_lens: Option<String>,
    /// Invocation result.
    pub outcome: LensExecutionOutcome,
    /// Number of Signals emitted by this invocation.
    pub emitted_signals: usize,
    /// Measured wall-clock Lens time.
    pub duration_micros: u64,
    /// Circuit stage immediately before routing the event.
    pub stage_before: Option<LensBreakerStage>,
    /// Circuit stage after accounting for invocation overhead.
    pub stage_after: Option<LensBreakerStage>,
    /// Action selected by the breaker, including sampling/disable transitions.
    pub breaker_action: Option<LensBreakerAction>,
}

/// Complete result of a raw event and all transitively chained Lens output.
#[derive(Clone, Debug, Default)]
pub struct LensDispatchReport {
    /// One record per matched Lens, ordered deterministically by routing order.
    pub records: Vec<LensExecutionRecord>,
    /// All Signals from raw and transitively chained Lens invocations.
    pub signals: Vec<Signal>,
    /// Malformed or spoofed Lens envelopes skipped by the projection bridge.
    pub projection_errors: Vec<String>,
}

impl LensDispatchReport {
    /// Whether any Lens invocation failed or lacked an implementation.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.records.iter().any(|record| {
            matches!(
                record.outcome,
                LensExecutionOutcome::Failed(_)
                    | LensExecutionOutcome::MissingImplementation
                    | LensExecutionOutcome::TaskFailed(_)
            )
        })
    }
}

/// Current cumulative status of one named Lens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensRuntimeStatus {
    /// Declarative Lens instance name.
    pub lens: String,
    /// Whether operator policy currently permits invocation.
    pub enabled: bool,
    /// Current overhead circuit stage.
    pub stage: LensBreakerStage,
    /// Lifetime count of overhead violations.
    pub total_violations: u64,
    /// Calls that reached the implementation.
    pub invocations: u64,
    /// Events omitted by the 50% sampler.
    pub sampled_out: u64,
    /// Implementation errors returned by the Lens.
    pub failures: u64,
    /// Lifetime output Signal count.
    pub emitted_signals: u64,
}

struct LensState {
    breaker: LensCircuitBreaker,
    operator_enabled: bool,
    sample_sequence: u64,
    invocations: u64,
    sampled_out: u64,
    failures: u64,
    emitted_signals: u64,
}

impl LensState {
    fn new(config: LensBreakerConfig) -> Self {
        Self {
            breaker: config.breaker(),
            operator_enabled: true,
            sample_sequence: 0,
            invocations: 0,
            sampled_out: 0,
            failures: 0,
            emitted_signals: 0,
        }
    }
}

/// Runtime executor for one validated declarative Lens registry.
pub struct LensExecutor {
    registry: LensRegistry,
    implementations: BTreeMap<String, Arc<dyn TelemetryObserve>>,
    states: Mutex<BTreeMap<String, LensState>>,
    breaker_config: LensBreakerConfig,
    projection: Option<ProjectionBridge>,
}

struct ProjectionBridge {
    aggregator: Arc<Mutex<TelemetryProjectionAggregator>>,
    sender: StateHubSender,
}

struct QueuedObservation {
    event: ObservableEvent,
    ancestry: Vec<LensScope>,
}

struct QueueState {
    pending: VecDeque<QueuedObservation>,
    in_flight: usize,
    enqueued: u64,
    processed: u64,
    dropped_oldest: u64,
    failed_dispatches: u64,
    last_error: Option<String>,
    closed: bool,
}

impl QueueState {
    fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_flight: 0,
            enqueued: 0,
            processed: 0,
            dropped_oldest: 0,
            failed_dispatches: 0,
            last_error: None,
            closed: false,
        }
    }
}

struct QueuedLensExecutorInner {
    runtime_id: String,
    executor: Arc<LensExecutor>,
    config: LensQueueConfig,
    queue: Mutex<QueueState>,
    work_ready: Arc<Notify>,
    idle: Arc<Notify>,
    handles: AtomicUsize,
}

/// Cloneable passive telemetry sink backed by a bounded drop-oldest queue.
///
/// `emit` only clones and enqueues the observation; one background worker
/// preserves event-cycle order while each cycle retains concurrent stacked
/// Lens execution and ordered chaining.
pub struct QueuedLensExecutor {
    inner: Arc<QueuedLensExecutorInner>,
}

impl Clone for QueuedLensExecutor {
    fn clone(&self) -> Self {
        self.inner.handles.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Drop for QueuedLensExecutor {
    fn drop(&mut self) {
        if self.inner.handles.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.inner.queue.lock().closed = true;
            self.inner.work_ready.notify_waiters();
            self.inner.idle.notify_waiters();
        }
    }
}

impl LensExecutor {
    /// Create an executor using the specification's 1% / 3 / 10 policy.
    pub fn new(registry: LensRegistry) -> Result<Self> {
        Self::with_breaker_config(registry, LensBreakerConfig::default())
    }

    /// Create an executor with an explicit per-Lens breaker policy.
    pub fn with_breaker_config(
        registry: LensRegistry,
        breaker_config: LensBreakerConfig,
    ) -> Result<Self> {
        registry.validate()?;
        let breaker_config = breaker_config.validate()?;
        Ok(Self {
            registry,
            implementations: BTreeMap::new(),
            states: Mutex::new(BTreeMap::new()),
            breaker_config,
            projection: None,
        })
    }

    /// Build and bind the built-in implementations represented by `registry`.
    ///
    /// Every specified built-in is constructed from the same validated
    /// registration. Unsupported/plugin blocks are rejected rather than
    /// turning into a silent hole in observability.
    pub fn from_registry(registry: &LensRegistry, sender: StateHubSender) -> Result<Self> {
        let mut executor = Self::new(registry.clone())?.with_projection(sender);
        for registration in registry.registrations() {
            let name = registration.config.name.clone();
            let implementation = create_builtin_lens(registration)?;
            executor.register(name, implementation)?;
        }
        executor.validate()?;
        Ok(executor)
    }

    /// Attach a fresh typed projection reducer and StateHub publisher.
    #[must_use]
    pub fn with_projection(self, sender: StateHubSender) -> Self {
        self.with_shared_projection(
            Arc::new(Mutex::new(TelemetryProjectionAggregator::new())),
            sender,
        )
    }

    /// Attach a caller-owned projection reducer shared across executors.
    #[must_use]
    pub fn with_shared_projection(
        mut self,
        aggregator: Arc<Mutex<TelemetryProjectionAggregator>>,
        sender: StateHubSender,
    ) -> Self {
        self.projection = Some(ProjectionBridge { aggregator, sender });
        self
    }

    /// Move this executor behind a bounded non-blocking observation queue.
    ///
    /// A Tokio runtime must be active so the single ordered queue worker can
    /// be spawned. When a StateHub projection bridge is attached, the queue is
    /// also weakly registered for operator inspection and control.
    pub fn into_queued(
        self,
        runtime_id: impl Into<String>,
        config: LensQueueConfig,
    ) -> Result<QueuedLensExecutor> {
        let config = config.validate()?;
        let runtime_id = runtime_id.into();
        if runtime_id.trim().is_empty() {
            return Err(RokoError::config("Lens runtime ID must not be empty"));
        }
        let runtime = tokio::runtime::Handle::try_current().map_err(|error| {
            RokoError::config(format!(
                "queued Lens execution requires an active Tokio runtime: {error}"
            ))
        })?;
        let registration_sender = self
            .projection
            .as_ref()
            .map(|projection| projection.sender.clone());
        let inner = Arc::new(QueuedLensExecutorInner {
            runtime_id,
            executor: Arc::new(self),
            config,
            queue: Mutex::new(QueueState::new()),
            work_ready: Arc::new(Notify::new()),
            idle: Arc::new(Notify::new()),
            handles: AtomicUsize::new(1),
        });
        if let Some(sender) = registration_sender {
            let control: Arc<dyn LensRuntimeControl> = inner.clone();
            sender.register_lens_runtime(&control).map_err(|error| {
                RokoError::config(format!("failed to register Lens runtime: {error}"))
            })?;
        }
        let worker = Arc::downgrade(&inner);
        runtime.spawn(run_lens_queue(worker));
        Ok(QueuedLensExecutor { inner })
    }

    /// Bind a named implementation to its declarative registration.
    ///
    /// Scope and event filters are checked eagerly so runtime routing cannot
    /// disagree with the implementation's declared contract.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        lens: Arc<dyn TelemetryObserve>,
    ) -> Result<()> {
        let name = name.into();
        let Some(registration) = self.registry.get(&name) else {
            return Err(RokoError::config(format!(
                "cannot bind unregistered lens `{name}`"
            )));
        };
        if self.implementations.contains_key(&name) {
            return Err(RokoError::config(format!(
                "lens implementation `{name}` is already bound"
            )));
        }
        if lens.scope() != registration.scope {
            return Err(RokoError::config(format!(
                "lens `{name}` scope mismatch: registry {:?}, implementation {:?}",
                registration.scope,
                lens.scope()
            )));
        }
        if normalized_filters(lens.observes()) != normalized_filters(&registration.observes) {
            return Err(RokoError::config(format!(
                "lens `{name}` event filters disagree with its registration"
            )));
        }

        self.implementations.insert(name.clone(), lens);
        self.states
            .get_mut()
            .insert(name, LensState::new(self.breaker_config));
        Ok(())
    }

    /// Verify that every declarative Lens has one executable implementation.
    pub fn validate(&self) -> Result<()> {
        let missing = self
            .registry
            .registrations()
            .iter()
            .filter(|registration| !self.implementations.contains_key(&registration.config.name))
            .map(|registration| registration.config.name.clone())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(RokoError::config(format!(
                "missing lens implementation{}: {}",
                if missing.len() == 1 { "" } else { "s" },
                missing.join(", ")
            )))
        }
    }

    /// Execute matching raw Lenses and every downstream chain before return.
    ///
    /// Stacked raw Lenses run concurrently. Failures are isolated and exposed
    /// in the report; successful outputs continue into their own chains.
    pub async fn dispatch(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> LensDispatchReport {
        let routed = self
            .registry
            .route_with_ancestry(event, ancestry)
            .into_iter()
            .map(|registration| registration.config.name.clone())
            .collect::<Vec<_>>();
        let mut report = LensDispatchReport::default();
        let observed_duration_ms = event.observed_duration_ms();
        let raw = self
            .invoke_stacked(routed, event, None, observed_duration_ms)
            .await;
        let mut chain_queue = VecDeque::new();
        for completed in raw {
            self.project_signals(
                &completed.record.lens,
                &completed.signals,
                &mut report.projection_errors,
            );
            for signal in &completed.signals {
                chain_queue.push_back((completed.record.lens.clone(), signal.clone()));
            }
            report.signals.extend(completed.signals);
            report.records.push(completed.record);
        }

        while let Some((upstream, signal)) = chain_queue.pop_front() {
            let output_event = ObservableEvent::SignalCreated(signal);
            let downstream = self
                .registry
                .route_lens_output(&upstream, &output_event)
                .into_iter()
                .map(|registration| registration.config.name.clone())
                .collect::<Vec<_>>();
            for completed in self
                .invoke_stacked(
                    downstream,
                    &output_event,
                    Some(&upstream),
                    observed_duration_ms,
                )
                .await
            {
                self.project_signals(
                    &completed.record.lens,
                    &completed.signals,
                    &mut report.projection_errors,
                );
                for signal in &completed.signals {
                    chain_queue.push_back((completed.record.lens.clone(), signal.clone()));
                }
                report.signals.extend(completed.signals);
                report.records.push(completed.record);
            }
        }
        report
    }

    fn project_signals(&self, emitter: &str, signals: &[Signal], errors: &mut Vec<String>) {
        let Some(bridge) = &self.projection else {
            return;
        };
        let mut aggregator = bridge.aggregator.lock();
        for signal in signals {
            let envelope = match LensSignalEnvelope::from_signal(signal) {
                Ok(envelope) => envelope,
                Err(TelemetryProjectionError::UnexpectedSignalKind(_)) => continue,
                Err(error) => {
                    errors.push(format!(
                        "Lens `{emitter}` emitted an invalid envelope: {error}"
                    ));
                    continue;
                }
            };
            if envelope.source_lens != emitter {
                errors.push(format!(
                    "Lens `{emitter}` emitted an envelope attributed to `{}`",
                    envelope.source_lens
                ));
                continue;
            }
            match aggregator.apply_envelope(envelope) {
                Ok(updates) => {
                    for update in updates {
                        update.apply_to(&bridge.sender);
                    }
                }
                Err(error) => errors.push(format!(
                    "Lens `{emitter}` projection update was rejected: {error}"
                )),
            }
        }
    }

    /// Inspect current breaker and accounting state for every bound Lens.
    #[must_use]
    pub fn statuses(&self) -> Vec<LensRuntimeStatus> {
        self.states
            .lock()
            .iter()
            .map(|(name, state)| LensRuntimeStatus {
                lens: name.clone(),
                enabled: state.operator_enabled,
                stage: state.breaker.stage(),
                total_violations: state.breaker.total_violations(),
                invocations: state.invocations,
                sampled_out: state.sampled_out,
                failures: state.failures,
                emitted_signals: state.emitted_signals,
            })
            .collect()
    }

    /// Operator recovery: re-enable a disabled Lens in sampled mode.
    pub fn reset_lens(&self, name: &str) -> Result<()> {
        {
            let mut states = self.states.lock();
            let Some(state) = states.get_mut(name) else {
                return Err(RokoError::config(format!("unknown bound lens `{name}`")));
            };
            state.breaker.reset();
            state.operator_enabled = true;
            state.sample_sequence = 0;
        }
        self.publish_operator_alert(name, true, "breaker reset; sampled recovery enabled");
        Ok(())
    }

    /// Explicitly enable or disable one bound Lens.
    pub fn set_lens_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        {
            let mut states = self.states.lock();
            let Some(state) = states.get_mut(name) else {
                return Err(RokoError::config(format!("unknown bound lens `{name}`")));
            };
            state.operator_enabled = enabled;
            state.sample_sequence = 0;
            if enabled {
                state.breaker.reset();
            }
        }
        self.publish_operator_alert(
            name,
            enabled,
            if enabled {
                "operator enabled Lens in sampled recovery mode"
            } else {
                "operator disabled Lens"
            },
        );
        Ok(())
    }

    fn publish_operator_alert(&self, lens: &str, enabled: bool, detail: &str) {
        let Some(projection) = &self.projection else {
            return;
        };
        projection.sender.publish(DashboardEvent::Diagnosis {
            summary: DiagnosisSummary {
                id: format!("lens-operator:{lens}"),
                severity: if enabled {
                    DiagnosisSeverity::Info
                } else {
                    DiagnosisSeverity::Warn
                },
                subject: format!("Lens operator control: {lens}"),
                detail: detail.to_string(),
                suggested_action: enabled.then(|| "Monitor Lens overhead".to_string()),
                intervention_taken: Some(detail.to_string()),
                ..DiagnosisSummary::default()
            },
        });
    }

    fn publish_breaker_transition(
        &self,
        lens: &str,
        before: LensBreakerStage,
        after: LensBreakerStage,
    ) {
        if before == after {
            return;
        }
        let Some(projection) = &self.projection else {
            return;
        };
        let (severity, detail, action) = match after {
            LensBreakerStage::Sampled => (
                DiagnosisSeverity::Warn,
                "Lens exceeded its overhead budget repeatedly and is now sampled at 50%",
                "Inspect Lens runtime status or reset after remediation",
            ),
            LensBreakerStage::Disabled => (
                DiagnosisSeverity::Alert,
                "Lens exceeded its overhead budget repeatedly and was disabled",
                "Inspect the Lens and use the reset or enable operator control",
            ),
            LensBreakerStage::Active => return,
        };
        projection.sender.publish(DashboardEvent::Diagnosis {
            summary: DiagnosisSummary {
                id: format!("lens-breaker:{lens}:{after:?}"),
                severity,
                subject: format!("Lens circuit breaker: {lens}"),
                detail: detail.to_string(),
                suggested_action: Some(action.to_string()),
                intervention_taken: (after == LensBreakerStage::Disabled)
                    .then(|| "Lens disabled automatically".to_string()),
                ..DiagnosisSummary::default()
            },
        });
    }

    async fn invoke_stacked(
        &self,
        names: Vec<String>,
        event: &ObservableEvent,
        source_lens: Option<&str>,
        observed_duration_ms: Option<u64>,
    ) -> Vec<CompletedInvocation> {
        let mut completed = BTreeMap::new();
        let mut tasks = JoinSet::new();
        let mut task_metadata = HashMap::new();
        for (index, name) in names.into_iter().enumerate() {
            let source_lens = source_lens.map(str::to_owned);
            let Some(lens) = self.implementations.get(&name).cloned() else {
                completed.insert(index, CompletedInvocation::missing(name, source_lens));
                continue;
            };
            match self.invocation_gate(&name, source_lens.clone()) {
                InvocationGate::Skip(record) => {
                    completed.insert(index, CompletedInvocation::without_signals(record));
                }
                InvocationGate::Invoke(stage_before) => {
                    let event = event.clone();
                    let task_name = name.clone();
                    let task_source = source_lens.clone();
                    let handle = tasks.spawn(async move {
                        let started = Instant::now();
                        let result = lens.observe(&event).await;
                        (
                            index,
                            name,
                            source_lens,
                            stage_before,
                            started.elapsed(),
                            result,
                        )
                    });
                    task_metadata
                        .insert(handle.id(), (index, task_name, task_source, stage_before));
                }
            }
        }

        while let Some(joined) = tasks.join_next_with_id().await {
            match joined {
                Ok((task_id, (index, name, source, before, elapsed, result))) => {
                    task_metadata.remove(&task_id);
                    let invocation = self.finish_invocation(
                        name,
                        source,
                        before,
                        elapsed,
                        observed_duration_ms,
                        result,
                    );
                    completed.insert(index, invocation);
                }
                Err(error) => {
                    if let Some((index, name, source, stage)) = task_metadata.remove(&error.id()) {
                        completed.insert(
                            index,
                            self.task_failed_invocation(name, source, stage, error.to_string()),
                        );
                    }
                }
            }
        }
        completed.into_values().collect()
    }

    fn invocation_gate(&self, name: &str, source_lens: Option<String>) -> InvocationGate {
        let mut states = self.states.lock();
        let Some(state) = states.get_mut(name) else {
            return InvocationGate::Skip(
                CompletedInvocation::missing(name.to_string(), source_lens).record,
            );
        };
        let stage = state.breaker.stage();
        if !state.operator_enabled {
            return InvocationGate::Skip(LensExecutionRecord {
                lens: name.to_string(),
                source_lens,
                outcome: LensExecutionOutcome::Disabled,
                emitted_signals: 0,
                duration_micros: 0,
                stage_before: Some(stage),
                stage_after: Some(stage),
                breaker_action: Some(LensBreakerAction::Disable),
            });
        }
        if stage == LensBreakerStage::Disabled {
            return InvocationGate::Skip(LensExecutionRecord {
                lens: name.to_string(),
                source_lens,
                outcome: LensExecutionOutcome::Disabled,
                emitted_signals: 0,
                duration_micros: 0,
                stage_before: Some(stage),
                stage_after: Some(stage),
                breaker_action: Some(LensBreakerAction::Disable),
            });
        }
        if stage == LensBreakerStage::Sampled {
            let invoke = state.sample_sequence & 1 == 0;
            state.sample_sequence = state.sample_sequence.saturating_add(1);
            if !invoke {
                state.sampled_out = state.sampled_out.saturating_add(1);
                return InvocationGate::Skip(LensExecutionRecord {
                    lens: name.to_string(),
                    source_lens,
                    outcome: LensExecutionOutcome::SampledOut,
                    emitted_signals: 0,
                    duration_micros: 0,
                    stage_before: Some(stage),
                    stage_after: Some(stage),
                    breaker_action: Some(LensBreakerAction::Skip),
                });
            }
        }
        InvocationGate::Invoke(stage)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_invocation(
        &self,
        name: String,
        source_lens: Option<String>,
        stage_before: LensBreakerStage,
        elapsed: std::time::Duration,
        observed_duration_ms: Option<u64>,
        result: Result<Vec<Signal>>,
    ) -> CompletedInvocation {
        let duration_micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        // The core breaker accepts integer milliseconds. Round non-zero
        // sub-millisecond work up so fast observed Cells cannot hide Lens cost
        // behind truncation to zero.
        let duration_ms = duration_micros.saturating_add(999) / 1_000;
        let (completed, stage_after) = {
            let mut states = self.states.lock();
            let Some(state) = states.get_mut(&name) else {
                return CompletedInvocation::missing(name, source_lens);
            };
            state.invocations = state.invocations.saturating_add(1);
            let action =
                observed_duration_ms.map(|observed| state.breaker.check(duration_ms, observed));
            let stage_after = state.breaker.stage();

            let completed = match result {
                Ok(signals) => {
                    state.emitted_signals =
                        state.emitted_signals.saturating_add(signals.len() as u64);
                    CompletedInvocation {
                        record: LensExecutionRecord {
                            lens: name.clone(),
                            source_lens,
                            outcome: LensExecutionOutcome::Succeeded,
                            emitted_signals: signals.len(),
                            duration_micros,
                            stage_before: Some(stage_before),
                            stage_after: Some(stage_after),
                            breaker_action: action,
                        },
                        signals,
                    }
                }
                Err(error) => {
                    state.failures = state.failures.saturating_add(1);
                    CompletedInvocation {
                        record: LensExecutionRecord {
                            lens: name.clone(),
                            source_lens,
                            outcome: LensExecutionOutcome::Failed(error.to_string()),
                            emitted_signals: 0,
                            duration_micros,
                            stage_before: Some(stage_before),
                            stage_after: Some(stage_after),
                            breaker_action: action,
                        },
                        signals: Vec::new(),
                    }
                }
            };
            (completed, stage_after)
        };
        self.publish_breaker_transition(&name, stage_before, stage_after);
        completed
    }

    fn task_failed_invocation(
        &self,
        name: String,
        source_lens: Option<String>,
        stage: LensBreakerStage,
        error: String,
    ) -> CompletedInvocation {
        let mut states = self.states.lock();
        let Some(state) = states.get_mut(&name) else {
            return CompletedInvocation::missing(name, source_lens);
        };
        state.invocations = state.invocations.saturating_add(1);
        state.failures = state.failures.saturating_add(1);
        CompletedInvocation::without_signals(LensExecutionRecord {
            lens: name,
            source_lens,
            outcome: LensExecutionOutcome::TaskFailed(error),
            emitted_signals: 0,
            duration_micros: 0,
            stage_before: Some(stage),
            stage_after: Some(state.breaker.stage()),
            breaker_action: None,
        })
    }
}

#[async_trait::async_trait]
impl TelemetryEventSink for LensExecutor {
    async fn emit(&self, event: &ObservableEvent, ancestry: &[LensScope]) -> Result<Vec<Signal>> {
        // Telemetry failures are isolated by design; callers can use
        // `dispatch` when they need the complete per-Lens report.
        Ok(self.dispatch(event, ancestry).await.signals)
    }
}

impl QueuedLensExecutor {
    /// Enqueue one observation without awaiting Lens execution.
    pub fn enqueue(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> Result<LensEnqueueOutcome> {
        self.inner.enqueue(event, ancestry)
    }

    /// Snapshot queue and Lens breaker/accounting state.
    #[must_use]
    pub fn snapshot(&self) -> LensRuntimeSnapshot {
        self.inner.snapshot()
    }

    /// Wait until all observations accepted before the call have drained.
    ///
    /// Returns `false` when `timeout` elapses first.
    pub async fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.inner.idle.notified();
            {
                let queue = self.inner.queue.lock();
                if queue.pending.is_empty() && queue.in_flight == 0 {
                    return true;
                }
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return false;
            }
        }
    }

    /// Reset one Lens breaker and re-enable sampled recovery.
    pub fn reset_lens(&self, name: &str) -> Result<()> {
        self.inner.executor.reset_lens(name)
    }

    /// Explicitly enable or disable one Lens.
    pub fn set_lens_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        self.inner.executor.set_lens_enabled(name, enabled)
    }
}

impl QueuedLensExecutorInner {
    fn enqueue(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> Result<LensEnqueueOutcome> {
        let outcome = {
            let mut queue = self.queue.lock();
            if queue.closed {
                return Err(RokoError::config(format!(
                    "Lens runtime `{}` delivery queue is closed",
                    self.runtime_id
                )));
            }
            let outcome = if queue.pending.len() >= self.config.capacity {
                match self.config.backpressure {
                    LensBackpressurePolicy::DropOldest => {
                        queue.pending.pop_front();
                        queue.dropped_oldest = queue.dropped_oldest.saturating_add(1);
                        LensEnqueueOutcome::ReplacedOldest
                    }
                }
            } else {
                LensEnqueueOutcome::Accepted
            };
            queue.pending.push_back(QueuedObservation {
                event: event.clone(),
                ancestry: ancestry.to_vec(),
            });
            queue.enqueued = queue.enqueued.saturating_add(1);
            outcome
        };
        self.work_ready.notify_one();
        Ok(outcome)
    }
}

#[async_trait::async_trait]
impl TelemetryEventSink for QueuedLensExecutor {
    async fn emit(&self, event: &ObservableEvent, ancestry: &[LensScope]) -> Result<Vec<Signal>> {
        if self.enqueue(event, ancestry)? == LensEnqueueOutcome::ReplacedOldest {
            tracing::warn!(
                runtime = %self.inner.runtime_id,
                capacity = self.inner.config.capacity,
                "Lens observation queue full; dropped oldest pending event"
            );
        }
        Ok(Vec::new())
    }
}

impl LensRuntimeControl for QueuedLensExecutorInner {
    fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    fn snapshot(&self) -> LensRuntimeSnapshot {
        let queue = self.queue.lock();
        LensRuntimeSnapshot {
            runtime_id: self.runtime_id.clone(),
            queue: LensQueueSnapshot {
                capacity: self.config.capacity,
                depth: queue.pending.len(),
                in_flight: queue.in_flight,
                enqueued: queue.enqueued,
                processed: queue.processed,
                dropped_oldest: queue.dropped_oldest,
                failed_dispatches: queue.failed_dispatches,
                backpressure: self.config.backpressure.label().to_string(),
                last_error: queue.last_error.clone(),
            },
            lenses: self
                .executor
                .statuses()
                .into_iter()
                .map(|status| LensOperatorStatus {
                    lens: status.lens,
                    enabled: status.enabled,
                    breaker_stage: breaker_stage_label(status.stage).to_string(),
                    total_violations: status.total_violations,
                    invocations: status.invocations,
                    sampled_out: status.sampled_out,
                    failures: status.failures,
                    emitted_signals: status.emitted_signals,
                })
                .collect(),
        }
    }

    fn reset_lens(&self, lens: &str) -> std::result::Result<(), String> {
        self.executor
            .reset_lens(lens)
            .map_err(|error| error.to_string())
    }

    fn set_lens_enabled(&self, lens: &str, enabled: bool) -> std::result::Result<(), String> {
        self.executor
            .set_lens_enabled(lens, enabled)
            .map_err(|error| error.to_string())
    }

    fn enqueue_observable(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> std::result::Result<bool, String> {
        self.enqueue(event, ancestry)
            .map(|outcome| outcome == LensEnqueueOutcome::ReplacedOldest)
            .map_err(|error| error.to_string())
    }
}

enum QueueWork {
    Dispatch(Box<QueuedObservation>),
    Wait,
    Closed,
}

async fn run_lens_queue(worker: std::sync::Weak<QueuedLensExecutorInner>) {
    loop {
        let Some(inner) = worker.upgrade() else {
            return;
        };
        let work_ready = Arc::clone(&inner.work_ready);
        let notified = work_ready.notified();
        let work = {
            let mut queue = inner.queue.lock();
            if let Some(observation) = queue.pending.pop_front() {
                queue.in_flight = queue.in_flight.saturating_add(1);
                QueueWork::Dispatch(Box::new(observation))
            } else if queue.closed {
                QueueWork::Closed
            } else {
                QueueWork::Wait
            }
        };

        match work {
            QueueWork::Dispatch(observation) => {
                let report = inner
                    .executor
                    .dispatch(&observation.event, &observation.ancestry)
                    .await;
                let mut queue = inner.queue.lock();
                queue.in_flight = queue.in_flight.saturating_sub(1);
                queue.processed = queue.processed.saturating_add(1);
                if report.has_failures() || !report.projection_errors.is_empty() {
                    queue.failed_dispatches = queue.failed_dispatches.saturating_add(1);
                    queue.last_error = report.projection_errors.first().cloned().or_else(|| {
                        report
                            .records
                            .iter()
                            .find(|record| {
                                matches!(
                                    record.outcome,
                                    LensExecutionOutcome::Failed(_)
                                        | LensExecutionOutcome::MissingImplementation
                                        | LensExecutionOutcome::TaskFailed(_)
                                )
                            })
                            .map(|record| {
                                format!(
                                    "Lens `{}` dispatch failed: {:?}",
                                    record.lens, record.outcome
                                )
                            })
                    });
                }
                let idle = queue.pending.is_empty() && queue.in_flight == 0;
                drop(queue);
                if idle {
                    inner.idle.notify_waiters();
                }
            }
            QueueWork::Wait => {
                drop(inner);
                notified.await;
            }
            QueueWork::Closed => return,
        }
    }
}

const fn breaker_stage_label(stage: LensBreakerStage) -> &'static str {
    match stage {
        LensBreakerStage::Active => "active",
        LensBreakerStage::Sampled => "sampled",
        LensBreakerStage::Disabled => "disabled",
    }
}

enum InvocationGate {
    Invoke(LensBreakerStage),
    Skip(LensExecutionRecord),
}

struct CompletedInvocation {
    record: LensExecutionRecord,
    signals: Vec<Signal>,
}

impl CompletedInvocation {
    fn without_signals(record: LensExecutionRecord) -> Self {
        Self {
            record,
            signals: Vec::new(),
        }
    }

    fn missing(name: String, source_lens: Option<String>) -> Self {
        Self::without_signals(LensExecutionRecord {
            lens: name,
            source_lens,
            outcome: LensExecutionOutcome::MissingImplementation,
            emitted_signals: 0,
            duration_micros: 0,
            stage_before: None,
            stage_after: None,
            breaker_action: None,
        })
    }
}

fn normalized_filters(filters: &[ObservableEventKind]) -> BTreeSet<ObservableEventKind> {
    if filters.is_empty() || filters.contains(&ObservableEventKind::All) {
        BTreeSet::from([ObservableEventKind::All])
    } else {
        filters.iter().copied().collect()
    }
}

fn create_builtin_lens(registration: &LensRegistration) -> Result<Arc<dyn TelemetryObserve>> {
    if let Some(lens) = create_builtin_health_lens(&registration.config)? {
        return Ok(lens);
    }

    let name = registration.config.name.clone();
    let scope = registration.scope.clone();
    let observes = registration.observes.clone();
    let params = &registration.config.params;
    let block = normalized_builtin_block(&registration.config.block);
    let lens: Arc<dyn TelemetryObserve> = match block.as_str() {
        "roko:cost-lens" | "cost-lens" => Arc::new(EventCostLens::new(name, scope, observes)),
        "roko:latency-lens" | "latency-lens" => {
            Arc::new(LatencyLens::new(name, scope, observes, params)?)
        }
        "roko:quality-lens" | "quality-lens" => {
            Arc::new(QualityLens::new(name, scope, observes, params)?)
        }
        "roko:efficiency-lens" | "efficiency-lens" => {
            Arc::new(EfficiencyLens::new(name, scope, observes, params)?)
        }
        "roko:trend-lens" | "trend-lens" => {
            Arc::new(TrendLens::new(name, scope, observes, params)?)
        }
        "roko:anomaly-lens" | "anomaly-lens" => {
            Arc::new(AnomalyLens::new(name, scope, observes, params)?)
        }
        "roko:usage-lens" | "usage-lens" => {
            Arc::new(UsageLens::new(name, scope, observes, params)?)
        }
        "roko:collective-intelligence-lens"
        | "collective-intelligence-lens"
        | "roko:c-factor-lens"
        | "c-factor-lens" => Arc::new(CollectiveIntelligenceLens::new(
            name, scope, observes, params,
        )?),
        _ => {
            return Err(RokoError::config(format!(
                "lens `{name}` uses unsupported runtime block `{}`",
                registration.config.block
            )));
        }
    };
    Ok(lens)
}

fn normalized_builtin_block(block: &str) -> String {
    block
        .trim()
        .split('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Stateful built-in CostLens driven by leaf Cell completion events.
///
/// Graph completion costs are intentionally not emitted because they normally
/// aggregate those same Cell costs. Materializing both would double-count in
/// StateHub's target-summing cost projection.
pub struct EventCostLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    cumulative_usd: Mutex<BTreeMap<String, f64>>,
}

impl EventCostLens {
    /// Construct a named instance from its validated registration.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
    ) -> Self {
        Self {
            name: name.into(),
            scope,
            observes,
            cumulative_usd: Mutex::new(BTreeMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for EventCostLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let (target, interval_ms, total_usd) = match event {
            ObservableEvent::CellCompleted {
                block,
                duration_ms,
                cost_usd,
                ..
            } => (format!("cell:{block}"), *duration_ms, *cost_usd),
            _ => return Ok(Vec::new()),
        };
        let cumulative_usd = {
            let mut totals = self.cumulative_usd.lock();
            let cumulative = totals.entry(target.clone()).or_default();
            *cumulative += total_usd;
            *cumulative
        };
        let payload = CostReportPayload {
            target,
            interval_ms,
            total_usd,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            model_breakdown: BTreeMap::new(),
            cumulative_usd,
            budget_remaining: None,
            vitality: None,
        };
        let envelope = LensSignalEnvelope::new(&self.name, LensPayload::CostReport(payload));
        envelope
            .to_signal()
            .map(|signal| vec![signal])
            .map_err(|error| {
                RokoError::config(format!("CostLens envelope encoding failed: {error}"))
            })
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}
