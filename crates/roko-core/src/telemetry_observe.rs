//! Event-oriented telemetry observation contracts, typed Lens payloads, and
//! pure Lens calculations.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Result, Signal, Verdict};

/// Read-only protocol implemented by event-oriented telemetry Lenses.
///
/// This is deliberately distinct from [`crate::Observe`], the synchronous
/// environment-observation protocol that returns Engrams. Implementations
/// declare both their event-family filters and scope so a router can avoid
/// invoking them for irrelevant events.
#[async_trait]
pub trait TelemetryObserve: Send + Sync {
    /// Observe one immutable lifecycle event and emit zero or more Signals.
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;

    /// Event families accepted by this Lens.
    fn observes(&self) -> &[ObservableEventKind];

    /// Subject (or upstream Lens) observed by this Lens.
    fn scope(&self) -> LensScope;
}

/// Runtime boundary used by execution engines to deliver lifecycle events.
///
/// The engine depends only on this passive sink contract; routing, chained
/// Lens execution, projection updates, and overhead enforcement remain owned
/// by the telemetry runtime. Implementations must not mutate the observed
/// Cell or Graph.
#[async_trait]
pub trait TelemetryEventSink: Send + Sync {
    /// Deliver one event together with its runtime-resolved scope ancestry.
    async fn emit(&self, event: &ObservableEvent, ancestry: &[LensScope]) -> Result<Vec<Signal>>;
}

/// Subject scope attached to a telemetry Lens.
///
/// Identifiers remain strings so this core contract does not depend on graph,
/// agent, workspace, or extension runtime implementations.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LensScope {
    Cell(String),
    Graph(String),
    Agent(String),
    Space(String),
    Lens(String),
    Global,
}

impl LensScope {
    /// Whether this Lens scope accepts the source represented by `event`.
    ///
    /// An empty identifier is a family wildcard (for example `Cell("")`).
    /// Global Lenses accept every source. Cross-level containment (such as a
    /// Cell belonging to an Agent) must be resolved by the runtime because
    /// this portable event contract intentionally carries no topology graph.
    #[must_use]
    pub fn matches_event(&self, event: &ObservableEvent) -> bool {
        self.matches_source(&event.source_scope())
    }

    /// Whether this scope accepts a normalized event source scope.
    #[must_use]
    pub fn matches_source(&self, source: &Self) -> bool {
        match (self, source) {
            (Self::Global, _) => true,
            (Self::Cell(expected), Self::Cell(actual))
            | (Self::Graph(expected), Self::Graph(actual))
            | (Self::Agent(expected), Self::Agent(actual))
            | (Self::Space(expected), Self::Space(actual))
            | (Self::Lens(expected), Self::Lens(actual)) => {
                expected.is_empty() || expected == actual
            }
            _ => false,
        }
    }
}

/// Lifecycle family used to pre-filter telemetry delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObservableEventKind {
    SignalLifecycle,
    CellLifecycle,
    GraphLifecycle,
    AgentLifecycle,
    MemoryLifecycle,
    VerifyLifecycle,
    TriggerLifecycle,
    ExtensionLifecycle,
    All,
}

impl ObservableEventKind {
    /// Whether this filter accepts `event`.
    #[must_use]
    pub fn matches(self, event: &ObservableEvent) -> bool {
        self == Self::All || self == event.kind()
    }
}

/// Immutable event delivered to telemetry Lenses.
///
/// The specification names 38 events, but its complete enumerated list has 39
/// (8 + 7 + 6 + 7 + 4 + 2 + 3 + 2). All named events are retained here. Ref
/// types and runtime-specific summaries use `String`; durations and costs use
/// portable scalar units suitable for serde and cross-process transport.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObservableEvent {
    // Signal lifecycle (8).
    SignalCreated(Signal),
    SignalScored(String, String),
    SignalRouted(String, String),
    SignalVerified(String, Verdict),
    SignalComposed(Vec<String>, Signal),
    SignalDemurrageApplied(String, f64),
    SignalPromoted(String, String, String),
    SignalPruned(String),

    // Cell lifecycle (7).
    CellStarted {
        block: String,
        run: String,
        input_hash: String,
    },
    CellCompleted {
        block: String,
        run: String,
        duration_ms: u64,
        cost_usd: f64,
    },
    CellFailed {
        block: String,
        run: String,
        error: String,
    },
    CellRetried {
        block: String,
        run: String,
        attempt: u32,
        reason: String,
    },
    CellCancelled {
        block: String,
        run: String,
    },
    CellPredictionPublished {
        block: String,
        prediction: String,
    },
    CellCalibrationReceived {
        block: String,
        error: f64,
    },

    // Graph lifecycle (6).
    GraphStarted {
        graph: String,
        run: String,
        input_hash: String,
    },
    GraphNodeCompleted {
        graph: String,
        run: String,
        node: String,
        duration_ms: u64,
    },
    GraphCompleted {
        graph: String,
        run: String,
        duration_ms: u64,
        cost_usd: f64,
    },
    GraphFailed {
        graph: String,
        run: String,
        error: String,
    },
    GraphPaused {
        graph: String,
        run: String,
        reason: String,
    },
    GraphResumed {
        graph: String,
        run: String,
    },

    // Agent lifecycle (7).
    AgentTick {
        agent: String,
        regime: String,
        prediction_error: f64,
        vitality: f64,
    },
    AgentRegimeChange {
        agent: String,
        old: String,
        new_regime: String,
    },
    AgentBudgetUpdate {
        agent: String,
        spent_usd: f64,
        remaining_usd: f64,
        vitality: f64,
    },
    AgentModeChange {
        agent: String,
        old: String,
        new_mode: String,
    },
    AgentPhaseChange {
        agent: String,
        old: String,
        new_phase: String,
    },
    AgentStateTransition {
        agent: String,
        old: String,
        new_state: String,
    },
    AgentSlotUpdate {
        agent: String,
        slot: String,
        state: String,
    },

    // Memory lifecycle (4).
    MemoryRetrieved {
        query: String,
        results: usize,
        duration_ms: u64,
    },
    MemoryStored {
        signal: String,
        tier: String,
    },
    MemoryConsolidated {
        promoted: usize,
        demoted: usize,
        pruned: usize,
    },
    DemurrageApplied {
        count: usize,
        total_balance_lost: f64,
    },

    // Verify lifecycle (2).
    VerifyPreResult {
        block: String,
        verdict: Verdict,
        evidence: Vec<String>,
    },
    VerifyPostResult {
        block: String,
        verdict: Verdict,
        reward: f64,
        evidence: Vec<String>,
    },

    // Trigger lifecycle (3).
    TriggerFired {
        trigger: String,
        graph: String,
    },
    TriggerArmed {
        trigger: String,
    },
    TriggerDisarmed {
        trigger: String,
    },

    // Extension lifecycle (2).
    ExtensionHookCalled {
        extension: String,
        hook: String,
        layer: u8,
        duration_ms: u64,
    },
    ExtensionHookFailed {
        extension: String,
        hook: String,
        error: String,
    },
}

impl ObservableEvent {
    /// The lifecycle family of this event.
    #[must_use]
    pub const fn kind(&self) -> ObservableEventKind {
        match self {
            Self::SignalCreated(_)
            | Self::SignalScored(_, _)
            | Self::SignalRouted(_, _)
            | Self::SignalVerified(_, _)
            | Self::SignalComposed(_, _)
            | Self::SignalDemurrageApplied(_, _)
            | Self::SignalPromoted(_, _, _)
            | Self::SignalPruned(_) => ObservableEventKind::SignalLifecycle,
            Self::CellStarted { .. }
            | Self::CellCompleted { .. }
            | Self::CellFailed { .. }
            | Self::CellRetried { .. }
            | Self::CellCancelled { .. }
            | Self::CellPredictionPublished { .. }
            | Self::CellCalibrationReceived { .. } => ObservableEventKind::CellLifecycle,
            Self::GraphStarted { .. }
            | Self::GraphNodeCompleted { .. }
            | Self::GraphCompleted { .. }
            | Self::GraphFailed { .. }
            | Self::GraphPaused { .. }
            | Self::GraphResumed { .. } => ObservableEventKind::GraphLifecycle,
            Self::AgentTick { .. }
            | Self::AgentRegimeChange { .. }
            | Self::AgentBudgetUpdate { .. }
            | Self::AgentModeChange { .. }
            | Self::AgentPhaseChange { .. }
            | Self::AgentStateTransition { .. }
            | Self::AgentSlotUpdate { .. } => ObservableEventKind::AgentLifecycle,
            Self::MemoryRetrieved { .. }
            | Self::MemoryStored { .. }
            | Self::MemoryConsolidated { .. }
            | Self::DemurrageApplied { .. } => ObservableEventKind::MemoryLifecycle,
            Self::VerifyPreResult { .. } | Self::VerifyPostResult { .. } => {
                ObservableEventKind::VerifyLifecycle
            }
            Self::TriggerFired { .. }
            | Self::TriggerArmed { .. }
            | Self::TriggerDisarmed { .. } => ObservableEventKind::TriggerLifecycle,
            Self::ExtensionHookCalled { .. } | Self::ExtensionHookFailed { .. } => {
                ObservableEventKind::ExtensionLifecycle
            }
        }
    }

    /// Whether at least one declared family filter accepts this event.
    ///
    /// An empty filter list observes nothing; including [`ObservableEventKind::All`]
    /// observes every event.
    #[must_use]
    pub fn matches_any(&self, filters: &[ObservableEventKind]) -> bool {
        filters.iter().any(|filter| filter.matches(self))
    }

    /// A normalized source scope for routing without runtime-type coupling.
    #[must_use]
    pub fn source_scope(&self) -> LensScope {
        match self {
            Self::CellStarted { block, .. }
            | Self::CellCompleted { block, .. }
            | Self::CellFailed { block, .. }
            | Self::CellRetried { block, .. }
            | Self::CellCancelled { block, .. }
            | Self::CellPredictionPublished { block, .. }
            | Self::CellCalibrationReceived { block, .. }
            | Self::VerifyPreResult { block, .. }
            | Self::VerifyPostResult { block, .. } => LensScope::Cell(block.clone()),
            Self::GraphStarted { graph, .. }
            | Self::GraphNodeCompleted { graph, .. }
            | Self::GraphCompleted { graph, .. }
            | Self::GraphFailed { graph, .. }
            | Self::GraphPaused { graph, .. }
            | Self::GraphResumed { graph, .. }
            | Self::TriggerFired { graph, .. } => LensScope::Graph(graph.clone()),
            Self::AgentTick { agent, .. }
            | Self::AgentRegimeChange { agent, .. }
            | Self::AgentBudgetUpdate { agent, .. }
            | Self::AgentModeChange { agent, .. }
            | Self::AgentPhaseChange { agent, .. }
            | Self::AgentStateTransition { agent, .. }
            | Self::AgentSlotUpdate { agent, .. } => LensScope::Agent(agent.clone()),
            _ => LensScope::Global,
        }
    }

    /// Duration of the observed operation, when this event closes one.
    ///
    /// The Lens runtime uses this value as the denominator for its overhead
    /// breaker. Start, transition, and marker events intentionally return
    /// `None` because no meaningful operation duration exists yet.
    #[must_use]
    pub const fn observed_duration_ms(&self) -> Option<u64> {
        match self {
            Self::CellCompleted { duration_ms, .. }
            | Self::GraphNodeCompleted { duration_ms, .. }
            | Self::GraphCompleted { duration_ms, .. }
            | Self::MemoryRetrieved { duration_ms, .. }
            | Self::ExtensionHookCalled { duration_ms, .. } => Some(*duration_ms),
            _ => None,
        }
    }
}

/// Cost and token expenditure for one target interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostReportPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_usd: f64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cumulative_usd: f64,
    pub budget_remaining: Option<f64>,
    pub vitality: Option<f64>,
}

/// Execution-duration distribution for one target interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatencyPayload {
    pub target: String,
    pub interval_ms: u64,
    pub count: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub mean_ms: u64,
}

/// Pass and fail totals for one verification rung.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassFailCounts {
    pub passed: u64,
    pub failed: u64,
}

/// Verification quality measurements for one target interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QualityPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_verifications: u64,
    pub pre_verify_vetoes: u64,
    pub post_verify_passed: u64,
    pub post_verify_failed: u64,
    pub pass_rate: f64,
    pub avg_reward: f64,
    pub hard_criteria_failures: u64,
    pub rung_breakdown: BTreeMap<String, PassFailCounts>,
}

/// Task, cache, prediction, and vitality efficiency for one agent interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EfficiencyPayload {
    pub agent: String,
    pub interval_ms: u64,
    pub tasks_completed: u64,
    pub tokens_per_task: f64,
    pub usd_per_task: f64,
    pub quality_per_usd: f64,
    pub t0_hit_rate: f64,
    pub t1_hit_rate: f64,
    pub t2_hit_rate: f64,
    pub avg_prediction_error: f64,
    pub vitality: f64,
    pub vitality_phase: String,
}

/// Telemetry-specific error classification.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorCategory {
    Timeout,
    CapabilityDenied,
    External,
    LogicError,
    InputInvalid,
    Cancelled,
}

/// Error totals and retry outcomes for one target interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_errors: u64,
    pub by_category: BTreeMap<String, u64>,
    pub by_block: BTreeMap<String, u64>,
    pub retry_count: u64,
    pub retry_success_rate: f64,
    pub error_rate: f64,
}

/// Knowledge-quality changes for one Memory interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriftPayload {
    pub memory: String,
    pub interval_ms: u64,
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<String, u64>,
    pub avg_balance: f64,
    pub balance_delta: f64,
    pub promotion_rate: f64,
    pub demotion_rate: f64,
    pub cold_entries: u64,
    pub anti_knowledge_count: u64,
    pub heuristic_calibration_avg: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Budget consumption and projected exhaustion for an Agent or Space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetAlertPayload {
    pub target: String,
    pub budget_total: f64,
    pub budget_spent: f64,
    pub budget_remaining: f64,
    pub vitality: f64,
    pub vitality_phase: String,
    pub projected_exhaustion_ms: Option<i64>,
    pub burn_rate: f64,
    pub level: AlertLevel,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
}

/// Statistical trend computed by a Lens observing another Lens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrendPayload {
    pub source_lens: String,
    pub metric: String,
    pub window_ms: u64,
    pub slope: f64,
    pub ema: f64,
    pub ema_previous: f64,
    pub direction: TrendDirection,
    pub r_squared: f64,
    pub data_points: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyDirection {
    Above,
    Below,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyLevel {
    Moderate,
    Severe,
    Critical,
}

/// Outlier detected by a Lens observing another Lens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnomalyPayload {
    pub source_lens: String,
    pub metric: String,
    pub observed_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub direction: AnomalyDirection,
    pub severity: AnomalyLevel,
}

/// Aggregate marketplace/developer usage for one target interval.
///
/// The fields are limited to values carried by the lifecycle protocol so an
/// unknown provider token count is never collapsed into a reported zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsagePayload {
    pub target: String,
    pub interval_ms: u64,
    pub cell_runs: u64,
    pub graph_runs: u64,
    pub trigger_fires: u64,
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
}

/// Collective-intelligence Lens output for one Space interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CFactorPayload {
    pub space: String,
    pub interval_ms: u64,
    pub c_factor: f64,
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub hdc_diversity: f64,
    pub agent_count: usize,
    pub active_agents: usize,
    pub dominant_agent_share: f64,
    pub knowledge_flow_edges: usize,
    pub avg_agent_vitality: f64,
}

/// Normalized Shannon entropy of the cohort's turn distribution.
#[must_use]
pub fn turn_taking_entropy(turns_per_agent: &[u64]) -> f64 {
    let total = turns_per_agent
        .iter()
        .copied()
        .map(u128::from)
        .sum::<u128>();
    if total == 0 {
        return 0.0;
    }
    if turns_per_agent.len() <= 1 {
        return 1.0;
    }

    let total = total as f64;
    let entropy = turns_per_agent
        .iter()
        .copied()
        .filter(|turns| *turns > 0)
        .map(|turns| {
            let share = turns as f64 / total;
            -share * share.ln()
        })
        .sum::<f64>();
    entropy / (turns_per_agent.len() as f64).ln()
}

/// One minus mean squared prediction error over matched samples.
#[must_use]
pub fn peer_prediction_accuracy(predictions: &[f64], outcomes: &[f64]) -> f64 {
    let sample_count = predictions.len().min(outcomes.len());
    if sample_count == 0 {
        return 0.5;
    }
    let mse = predictions
        .iter()
        .zip(outcomes)
        .map(|(prediction, outcome)| (prediction - outcome).powi(2))
        .sum::<f64>()
        / sample_count as f64;
    (1.0 - mse).clamp(0.0, 1.0)
}

/// Recency-weighted fraction of cited Signals that survived verification.
#[must_use]
pub fn citation_reciprocity(survived_weights: &[f64], total_weights: &[f64]) -> f64 {
    let total = total_weights.iter().sum::<f64>();
    if total.abs() < f64::EPSILON {
        return 0.5;
    }
    survived_weights.iter().sum::<f64>() / total
}

/// Fraction of Bus deliveries confirmed during an interval.
#[must_use]
pub fn delivery_rate(confirmed: u64, dropped: u64) -> f64 {
    let total = u128::from(confirmed) + u128::from(dropped);
    if total == 0 {
        return 1.0;
    }
    confirmed as f64 / total as f64
}

/// One minus mean pairwise HDC similarity.
#[must_use]
pub fn hdc_diversity(pairwise_similarities: &[f64]) -> f64 {
    if pairwise_similarities.is_empty() {
        return 0.0;
    }
    1.0 - pairwise_similarities.iter().sum::<f64>() / pairwise_similarities.len() as f64
}
