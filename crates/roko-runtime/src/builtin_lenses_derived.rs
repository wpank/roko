//! Derived and collective built-in telemetry Lenses.
//!
//! Trend and anomaly Lenses consume only canonical, typed Lens output Signals.
//! Usage and collective-intelligence Lenses aggregate only values present on
//! lifecycle events. All state is bounded and updated in deterministic order.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use async_trait::async_trait;
use parking_lot::Mutex;
use roko_core::cfactor::{CohortMetrics, CohortWeights};
use roko_core::{
    AnomalyDirection, AnomalyLevel, AnomalyPayload, CFactorPayload, LensScope, ObservableEvent,
    ObservableEventKind, Result, RokoError, Signal, TelemetryObserve, TrendDirection, TrendPayload,
    UsagePayload, citation_reciprocity, delivery_rate, hdc_diversity, peer_prediction_accuracy,
    turn_taking_entropy,
};
use roko_primitives::HdcVector;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{LensPayload, LensSignalEnvelope, TelemetryProjectionError};

/// Factory block aliases for [`TrendLens`].
pub const TREND_LENS_ALIASES: &[&str] = &["roko:trend-lens", "trend-lens"];
/// Factory block aliases for [`AnomalyLens`].
pub const ANOMALY_LENS_ALIASES: &[&str] = &["roko:anomaly-lens", "anomaly-lens"];
/// Factory block aliases for [`UsageLens`].
pub const USAGE_LENS_ALIASES: &[&str] = &["roko:usage-lens", "usage-lens"];
/// Factory block aliases for [`CollectiveIntelligenceLens`].
pub const COLLECTIVE_INTELLIGENCE_LENS_ALIASES: &[&str] = &[
    "roko:collective-intelligence-lens",
    "collective-intelligence-lens",
    "roko:c-factor-lens",
    "c-factor-lens",
];

/// Signal tag containing an observed peer prediction in `[0, 1]`.
pub const PEER_PREDICTION_TAG: &str = "telemetry.peer_prediction";
/// Signal tag containing the outcome paired with [`PEER_PREDICTION_TAG`].
pub const PEER_OUTCOME_TAG: &str = "telemetry.peer_outcome";
/// Signal tag containing a non-negative count of confirmed deliveries.
pub const DELIVERY_CONFIRMED_TAG: &str = "telemetry.delivery_confirmed";
/// Signal tag containing a non-negative count of dropped deliveries.
pub const DELIVERY_DROPPED_TAG: &str = "telemetry.delivery_dropped";

const DEFAULT_INTERVAL_MS: u64 = 60_000;
const DEFAULT_MAX_POINTS: usize = 64;
const HARD_MAX_POINTS: usize = 4_096;

/// A bounded linear-regression and exponential-moving-average Lens.
pub struct TrendLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    metric: String,
    window_ms: u64,
    max_points: usize,
    min_data_points: usize,
    ema_alpha: f64,
    stable_epsilon: f64,
    state: Mutex<TrendState>,
}

#[derive(Default)]
struct TrendState {
    values: VecDeque<(i64, f64)>,
    ema: Option<f64>,
    last_timestamp_ms: Option<i64>,
}

impl TrendLens {
    /// Construct a Trend Lens from a validated registration.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: &BTreeMap<String, P>,
    ) -> Result<Self> {
        let name = validate_name(name.into())?;
        validate_chained_scope(&scope, "TrendLens")?;
        validate_observes(
            &observes,
            &[ObservableEventKind::SignalLifecycle],
            "TrendLens",
        )?;
        let params = params_object(
            params,
            &[
                "metric",
                "window",
                "window_ms",
                "max_points",
                "min_data_points",
                "ema_alpha",
                "stable_epsilon",
            ],
        )?;
        let metric = required_string(&params, "metric")?;
        let window_ms = interval_param(&params, 600_000)?;
        let max_points = usize_param(
            &params,
            "max_points",
            DEFAULT_MAX_POINTS,
            2,
            HARD_MAX_POINTS,
        )?;
        let min_data_points = usize_param(&params, "min_data_points", 3, 2, max_points)?;
        let ema_alpha = f64_param(&params, "ema_alpha", 0.3, f64::EPSILON, 1.0)?;
        let stable_epsilon = f64_param(&params, "stable_epsilon", 0.0, 0.0, f64::MAX)?;

        Ok(Self {
            name,
            scope,
            observes,
            metric,
            window_ms,
            max_points,
            min_data_points,
            ema_alpha,
            stable_epsilon,
            state: Mutex::new(TrendState::default()),
        })
    }
}

#[async_trait]
impl TelemetryObserve for TrendLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let ObservableEvent::SignalCreated(signal) = event else {
            return Ok(Vec::new());
        };
        let Some(envelope) = decode_upstream(signal, upstream_name(&self.scope)?)? else {
            return Ok(Vec::new());
        };
        let Some(value) = metric_value(&envelope.payload, &self.metric) else {
            return Ok(Vec::new());
        };
        ensure_finite(value, "TrendLens observed metric")?;

        let payload = {
            let mut state = self.state.lock();
            if state
                .last_timestamp_ms
                .is_some_and(|previous| signal.created_at_ms < previous)
            {
                return Err(config(
                    "TrendLens upstream Signal timestamps must be non-decreasing",
                ));
            }
            let ema_previous = state.ema.unwrap_or(value);
            let ema = state.ema.map_or(value, |previous| {
                self.ema_alpha
                    .mul_add(value, (1.0 - self.ema_alpha) * previous)
            });
            state.ema = Some(ema);
            state.last_timestamp_ms = Some(signal.created_at_ms);
            let window_ms = i64::try_from(self.window_ms).unwrap_or(i64::MAX);
            let cutoff = signal.created_at_ms.saturating_sub(window_ms);
            while state
                .values
                .front()
                .is_some_and(|(timestamp, _)| *timestamp < cutoff)
            {
                state.values.pop_front();
            }
            state.values.push_back((signal.created_at_ms, value));
            while state.values.len() > self.max_points {
                state.values.pop_front();
            }
            if state.values.len() < self.min_data_points {
                return Ok(Vec::new());
            }
            let values = state
                .values
                .iter()
                .map(|(_, value)| *value)
                .collect::<Vec<_>>();
            let (slope, r_squared) = linear_regression(&values);
            let direction = if slope > self.stable_epsilon {
                TrendDirection::Rising
            } else if slope < -self.stable_epsilon {
                TrendDirection::Falling
            } else {
                TrendDirection::Stable
            };
            TrendPayload {
                source_lens: upstream_name(&self.scope)?.to_owned(),
                metric: self.metric.clone(),
                window_ms: self.window_ms,
                slope,
                ema,
                ema_previous,
                direction,
                r_squared,
                data_points: state.values.len(),
            }
        };
        encode(&self.name, LensPayload::Trend(payload))
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

/// A bounded rolling outlier detector with standard-deviation and IQR scoring.
pub struct AnomalyLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    metric: String,
    max_points: usize,
    min_data_points: usize,
    sigma_moderate: f64,
    sigma_severe: f64,
    sigma_critical: f64,
    state: Mutex<VecDeque<f64>>,
}

impl AnomalyLens {
    /// Construct an Anomaly Lens from a validated registration.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: &BTreeMap<String, P>,
    ) -> Result<Self> {
        let name = validate_name(name.into())?;
        validate_chained_scope(&scope, "AnomalyLens")?;
        validate_observes(
            &observes,
            &[ObservableEventKind::SignalLifecycle],
            "AnomalyLens",
        )?;
        let params = params_object(
            params,
            &[
                "metric",
                "window_points",
                "min_data_points",
                "sigma_moderate",
                "sigma_severe",
                "sigma_critical",
            ],
        )?;
        let metric = optional_string(&params, "metric")?.unwrap_or_else(|| "ema".to_owned());
        let max_points = usize_param(
            &params,
            "window_points",
            DEFAULT_MAX_POINTS,
            3,
            HARD_MAX_POINTS,
        )?;
        let min_data_points = usize_param(&params, "min_data_points", 5, 3, max_points)?;
        let sigma_moderate = f64_param(&params, "sigma_moderate", 3.0, f64::EPSILON, f64::MAX)?;
        let sigma_severe = f64_param(&params, "sigma_severe", 4.0, f64::EPSILON, f64::MAX)?;
        let sigma_critical = f64_param(&params, "sigma_critical", 5.0, f64::EPSILON, f64::MAX)?;
        if !(sigma_moderate < sigma_severe && sigma_severe < sigma_critical) {
            return Err(config(
                "AnomalyLens sigma thresholds must be strictly increasing",
            ));
        }
        Ok(Self {
            name,
            scope,
            observes,
            metric,
            max_points,
            min_data_points,
            sigma_moderate,
            sigma_severe,
            sigma_critical,
            state: Mutex::new(VecDeque::new()),
        })
    }
}

#[async_trait]
impl TelemetryObserve for AnomalyLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let ObservableEvent::SignalCreated(signal) = event else {
            return Ok(Vec::new());
        };
        let Some(envelope) = decode_upstream(signal, upstream_name(&self.scope)?)? else {
            return Ok(Vec::new());
        };
        let Some(value) = metric_value(&envelope.payload, &self.metric) else {
            return Ok(Vec::new());
        };
        ensure_finite(value, "AnomalyLens observed metric")?;

        let anomaly = {
            let mut values = self.state.lock();
            let anomaly = if values.len() >= self.min_data_points {
                classify_anomaly(
                    values.make_contiguous(),
                    value,
                    self.sigma_moderate,
                    self.sigma_severe,
                    self.sigma_critical,
                )
            } else {
                None
            };
            values.push_back(value);
            while values.len() > self.max_points {
                values.pop_front();
            }
            anomaly
        };
        let Some((expected_value, deviation, direction, severity)) = anomaly else {
            return Ok(Vec::new());
        };
        encode(
            &self.name,
            LensPayload::Anomaly(AnomalyPayload {
                source_lens: upstream_name(&self.scope)?.to_owned(),
                metric: self.metric.clone(),
                observed_value: value,
                expected_value,
                deviation,
                direction,
                severity,
            }),
        )
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

/// Cumulative usage derived from cell, graph, and trigger lifecycle evidence.
pub struct UsageLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    interval_ms: u64,
    max_runs: usize,
    state: Mutex<UsageState>,
}

#[derive(Default)]
struct UsageState {
    cell_runs: u64,
    graph_runs: u64,
    trigger_fires: u64,
    total_cost_usd: f64,
    total_duration_ms: u64,
    runs_with_cells: BTreeSet<String>,
    run_order: VecDeque<String>,
}

impl UsageLens {
    /// Construct a Usage Lens from a validated registration.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: &BTreeMap<String, P>,
    ) -> Result<Self> {
        let name = validate_name(name.into())?;
        match &scope {
            LensScope::Space(space) if !space.trim().is_empty() => {}
            LensScope::Global => {}
            _ => return Err(config("UsageLens scope must be a named Space or Global")),
        }
        validate_observes(
            &observes,
            &[
                ObservableEventKind::CellLifecycle,
                ObservableEventKind::GraphLifecycle,
                ObservableEventKind::TriggerLifecycle,
            ],
            "UsageLens",
        )?;
        let params = params_object(params, &["interval", "interval_ms", "max_runs"])?;
        Ok(Self {
            name,
            scope,
            observes,
            interval_ms: interval_param(&params, DEFAULT_INTERVAL_MS)?,
            max_runs: usize_param(&params, "max_runs", 4_096, 1, 65_536)?,
            state: Mutex::new(UsageState::default()),
        })
    }

    fn target(&self) -> String {
        match &self.scope {
            LensScope::Space(space) => format!("space:{space}"),
            LensScope::Global => "global".to_owned(),
            _ => unreachable!("scope validated by constructor"),
        }
    }

    fn payload(&self, state: &UsageState) -> UsagePayload {
        UsagePayload {
            target: self.target(),
            interval_ms: self.interval_ms,
            cell_runs: state.cell_runs,
            graph_runs: state.graph_runs,
            trigger_fires: state.trigger_fires,
            total_cost_usd: state.total_cost_usd,
            total_duration_ms: state.total_duration_ms,
        }
    }
}

#[async_trait]
impl TelemetryObserve for UsageLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let payload = {
            let mut state = self.state.lock();
            match event {
                ObservableEvent::CellCompleted {
                    run,
                    duration_ms,
                    cost_usd,
                    ..
                } => {
                    ensure_non_negative_finite(*cost_usd, "CellCompleted cost_usd")?;
                    let cell_runs = checked_add(state.cell_runs, 1, "UsageLens cell_runs")?;
                    let total_duration_ms = checked_add(
                        state.total_duration_ms,
                        *duration_ms,
                        "UsageLens total_duration_ms",
                    )?;
                    let total_cost_usd = state.total_cost_usd + cost_usd;
                    ensure_finite(total_cost_usd, "UsageLens total_cost_usd")?;
                    state.cell_runs = cell_runs;
                    state.total_duration_ms = total_duration_ms;
                    state.total_cost_usd = total_cost_usd;
                    if state.runs_with_cells.insert(run.clone()) {
                        state.run_order.push_back(run.clone());
                        while state.run_order.len() > self.max_runs {
                            if let Some(expired) = state.run_order.pop_front() {
                                state.runs_with_cells.remove(&expired);
                            }
                        }
                    }
                }
                ObservableEvent::GraphCompleted {
                    run,
                    duration_ms,
                    cost_usd,
                    ..
                } => {
                    ensure_non_negative_finite(*cost_usd, "GraphCompleted cost_usd")?;
                    let graph_runs = checked_add(state.graph_runs, 1, "UsageLens graph_runs")?;
                    if !state.runs_with_cells.contains(run) {
                        let total_duration_ms = checked_add(
                            state.total_duration_ms,
                            *duration_ms,
                            "UsageLens total_duration_ms",
                        )?;
                        let total_cost_usd = state.total_cost_usd + cost_usd;
                        ensure_finite(total_cost_usd, "UsageLens total_cost_usd")?;
                        state.total_duration_ms = total_duration_ms;
                        state.total_cost_usd = total_cost_usd;
                    }
                    state.graph_runs = graph_runs;
                }
                ObservableEvent::TriggerFired { .. } => {
                    state.trigger_fires =
                        checked_add(state.trigger_fires, 1, "UsageLens trigger_fires")?;
                }
                _ => return Ok(Vec::new()),
            }
            self.payload(&state)
        };
        encode(&self.name, LensPayload::Usage(payload))
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

/// Space-level implementation of the five-component collective-intelligence Lens.
pub struct CollectiveIntelligenceLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    interval_ms: u64,
    min_agents: usize,
    max_agents: usize,
    max_signals_per_agent: usize,
    max_pending_citations: usize,
    max_evidence_points: usize,
    emit_every: u64,
    weights: CohortWeights,
    state: Mutex<CollectiveState>,
}

/// Concise alias for [`CollectiveIntelligenceLens`].
pub type CFactorLens = CollectiveIntelligenceLens;

#[derive(Default)]
struct CollectiveState {
    agents: BTreeSet<String>,
    turns: BTreeMap<String, u64>,
    vitality: BTreeMap<String, f64>,
    vectors: BTreeMap<String, VecDeque<HdcVector>>,
    peer_predictions: VecDeque<f64>,
    peer_outcomes: VecDeque<f64>,
    delivery_confirmed: u64,
    delivery_dropped: u64,
    pending_citations: BTreeMap<String, usize>,
    pending_order: VecDeque<String>,
    verified_citations: u64,
    survived_citations: u64,
    knowledge_flow_edges: usize,
    seen_signals: BTreeSet<String>,
    seen_order: VecDeque<String>,
    eligible_events: u64,
}

struct CollectiveSignalPlan {
    signal_id: String,
    author: Option<(String, u64, Option<HdcVector>)>,
    peer_evidence: Option<(f64, f64)>,
    delivery_confirmed: u64,
    delivery_dropped: u64,
    knowledge_flow_edges: usize,
    citations: usize,
}

impl CollectiveIntelligenceLens {
    /// Construct a collective-intelligence Lens from a validated registration.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: &BTreeMap<String, P>,
    ) -> Result<Self> {
        let name = validate_name(name.into())?;
        if !matches!(&scope, LensScope::Space(space) if !space.trim().is_empty()) {
            return Err(config(
                "CollectiveIntelligenceLens scope must be a named Space",
            ));
        }
        validate_observes(
            &observes,
            &[
                ObservableEventKind::AgentLifecycle,
                ObservableEventKind::SignalLifecycle,
                ObservableEventKind::MemoryLifecycle,
            ],
            "CollectiveIntelligenceLens",
        )?;
        let params = params_object(
            params,
            &[
                "interval",
                "interval_ms",
                "min_agents",
                "max_agents",
                "max_signals_per_agent",
                "max_pending_citations",
                "max_evidence_points",
                "emit_every",
                "weight_turn_taking",
                "weight_peer_prediction",
                "weight_citation",
                "weight_delivery",
                "weight_hdc",
                "bias",
            ],
        )?;
        let max_agents = usize_param(&params, "max_agents", 128, 2, 1_024)?;
        let min_agents = usize_param(&params, "min_agents", 2, 2, max_agents)?;
        let weights = CohortWeights {
            turn_taking: f64_param(&params, "weight_turn_taking", 0.2, 0.0, 1.0)?,
            social_perceptiveness: f64_param(&params, "weight_peer_prediction", 0.2, 0.0, 1.0)?,
            trust_calibration: f64_param(&params, "weight_citation", 0.2, 0.0, 1.0)?,
            channel_openness: f64_param(&params, "weight_delivery", 0.2, 0.0, 1.0)?,
            cognitive_diversity: f64_param(&params, "weight_hdc", 0.2, 0.0, 1.0)?,
            bias: f64_param(&params, "bias", 0.0, -1.0, 1.0)?,
        };
        let weight_sum = weights.turn_taking
            + weights.social_perceptiveness
            + weights.trust_calibration
            + weights.channel_openness
            + weights.cognitive_diversity;
        if weight_sum <= f64::EPSILON {
            return Err(config(
                "CollectiveIntelligenceLens requires a positive component weight",
            ));
        }
        Ok(Self {
            name,
            scope,
            observes,
            interval_ms: interval_param(&params, DEFAULT_INTERVAL_MS)?,
            min_agents,
            max_agents,
            max_signals_per_agent: usize_param(&params, "max_signals_per_agent", 64, 1, 1_024)?,
            max_pending_citations: usize_param(&params, "max_pending_citations", 4_096, 1, 65_536)?,
            max_evidence_points: usize_param(&params, "max_evidence_points", 4_096, 1, 65_536)?,
            emit_every: u64_param(&params, "emit_every", 1, 1, 10_000)?,
            weights,
            state: Mutex::new(CollectiveState::default()),
        })
    }

    fn plan_signal(
        &self,
        state: &CollectiveState,
        signal: &Signal,
    ) -> Result<Option<CollectiveSignalPlan>> {
        let signal_id = signal.id.to_hex();
        if state.seen_signals.contains(&signal_id) {
            return Ok(None);
        }
        let peer_evidence = match (
            signal.tag(PEER_PREDICTION_TAG),
            signal.tag(PEER_OUTCOME_TAG),
        ) {
            (Some(prediction), Some(outcome)) => Some((
                parse_evidence_unit(prediction, PEER_PREDICTION_TAG)?,
                parse_evidence_unit(outcome, PEER_OUTCOME_TAG)?,
            )),
            (None, None) => None,
            _ => {
                return Err(config(format!(
                    "collective evidence requires both `{PEER_PREDICTION_TAG}` and `{PEER_OUTCOME_TAG}`"
                )));
            }
        };
        let confirmed =
            parse_optional_count(signal.tag(DELIVERY_CONFIRMED_TAG), DELIVERY_CONFIRMED_TAG)?;
        let dropped = parse_optional_count(signal.tag(DELIVERY_DROPPED_TAG), DELIVERY_DROPPED_TAG)?;
        let delivery_confirmed = checked_add(
            state.delivery_confirmed,
            confirmed,
            "CollectiveIntelligenceLens confirmed deliveries",
        )?;
        let delivery_dropped = checked_add(
            state.delivery_dropped,
            dropped,
            "CollectiveIntelligenceLens dropped deliveries",
        )?;
        let knowledge_flow_edges = state
            .knowledge_flow_edges
            .checked_add(signal.lineage.len())
            .ok_or_else(|| config("CollectiveIntelligenceLens knowledge-flow overflow"))?;
        let author = signal.provenance.author.trim();
        let tracks_author = !author.is_empty()
            && (state.agents.contains(author) || state.agents.len() < self.max_agents);
        let author = if tracks_author {
            let turns = checked_add(
                state.turns.get(author).copied().unwrap_or(0),
                1,
                "CollectiveIntelligenceLens author turns",
            )?;
            Some((
                author.to_owned(),
                turns,
                signal.fingerprint.map(|fingerprint| fingerprint.vector),
            ))
        } else {
            None
        };
        Ok(Some(CollectiveSignalPlan {
            signal_id,
            author,
            peer_evidence,
            delivery_confirmed,
            delivery_dropped,
            knowledge_flow_edges,
            citations: signal.lineage.len(),
        }))
    }

    fn apply_signal_plan(&self, state: &mut CollectiveState, plan: CollectiveSignalPlan) {
        state.seen_signals.insert(plan.signal_id.clone());
        state.seen_order.push_back(plan.signal_id.clone());
        let max_seen = self.max_agents.saturating_mul(self.max_signals_per_agent);
        while state.seen_order.len() > max_seen {
            if let Some(expired) = state.seen_order.pop_front() {
                state.seen_signals.remove(&expired);
            }
        }

        if let Some((author, turns, fingerprint)) = plan.author {
            state.agents.insert(author.clone());
            state.turns.insert(author.clone(), turns);
            if let Some(fingerprint) = fingerprint {
                let vectors = state.vectors.entry(author).or_default();
                vectors.push_back(fingerprint);
                while vectors.len() > self.max_signals_per_agent {
                    vectors.pop_front();
                }
            }
        }

        if let Some((prediction, outcome)) = plan.peer_evidence {
            state.peer_predictions.push_back(prediction);
            state.peer_outcomes.push_back(outcome);
            while state.peer_predictions.len() > self.max_evidence_points {
                state.peer_predictions.pop_front();
                state.peer_outcomes.pop_front();
            }
        }

        state.delivery_confirmed = plan.delivery_confirmed;
        state.delivery_dropped = plan.delivery_dropped;

        if plan.citations > 0 {
            state.knowledge_flow_edges = plan.knowledge_flow_edges;
            state
                .pending_citations
                .insert(plan.signal_id.clone(), plan.citations);
            state.pending_order.push_back(plan.signal_id);
            while state.pending_order.len() > self.max_pending_citations {
                if let Some(expired) = state.pending_order.pop_front() {
                    state.pending_citations.remove(&expired);
                }
            }
        }
    }

    fn process_signal(&self, state: &mut CollectiveState, signal: &Signal) -> Result<()> {
        // Build a complete, range-checked plan before mutating state, so a
        // malformed tagged Signal cannot leave behind a partial observation.
        if let Some(plan) = self.plan_signal(state, signal)? {
            self.apply_signal_plan(state, plan);
        }
        Ok(())
    }

    fn process_event(&self, state: &mut CollectiveState, event: &ObservableEvent) -> Result<()> {
        match event {
            ObservableEvent::SignalCreated(signal) | ObservableEvent::SignalComposed(_, signal) => {
                self.process_signal(state, signal)
            }
            ObservableEvent::SignalVerified(signal, verdict) => {
                if let Some(citations) = state.pending_citations.get(signal).copied() {
                    let citations = u64::try_from(citations).map_err(|_| {
                        config("CollectiveIntelligenceLens citation count overflow")
                    })?;
                    let verified_citations = checked_add(
                        state.verified_citations,
                        citations,
                        "CollectiveIntelligenceLens verified citations",
                    )?;
                    let survived_citations = if verdict.passed {
                        checked_add(
                            state.survived_citations,
                            citations,
                            "CollectiveIntelligenceLens survived citations",
                        )?
                    } else {
                        state.survived_citations
                    };
                    state.pending_citations.remove(signal);
                    state.pending_order.retain(|pending| pending != signal);
                    state.verified_citations = verified_citations;
                    state.survived_citations = survived_citations;
                }
                Ok(())
            }
            ObservableEvent::AgentTick {
                agent, vitality, ..
            } => {
                ensure_finite(*vitality, "AgentTick vitality")?;
                if !(0.0..=1.0).contains(vitality) {
                    return Err(config("AgentTick vitality must be in [0, 1]"));
                }
                let agent = agent.trim();
                if agent.is_empty() {
                    return Err(config("AgentTick agent cannot be empty"));
                }
                if state.agents.contains(agent) || state.agents.len() < self.max_agents {
                    state.agents.insert(agent.to_owned());
                    state.vitality.insert(agent.to_owned(), *vitality);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn snapshot(&self, state: &CollectiveState) -> Option<CFactorPayload> {
        if state.agents.len() < self.min_agents
            || state.peer_predictions.is_empty()
            || state.verified_citations == 0
            || state
                .delivery_confirmed
                .saturating_add(state.delivery_dropped)
                == 0
            || state.vitality.is_empty()
        {
            return None;
        }
        let centroids = state
            .agents
            .iter()
            .filter_map(|agent| {
                let vectors = state.vectors.get(agent)?;
                let refs = vectors.iter().collect::<Vec<_>>();
                (!refs.is_empty()).then(|| HdcVector::bundle(&refs))
            })
            .collect::<Vec<_>>();
        if centroids.len() < self.min_agents {
            return None;
        }
        let mut similarities = Vec::new();
        for left in 0..centroids.len() {
            for right in left + 1..centroids.len() {
                similarities.push(f64::from(centroids[left].similarity(&centroids[right])));
            }
        }
        if similarities.is_empty() {
            return None;
        }

        let turns = state
            .agents
            .iter()
            .map(|agent| state.turns.get(agent).copied().unwrap_or(0))
            .collect::<Vec<_>>();
        let total_turns = turns.iter().copied().map(u128::from).sum::<u128>();
        if total_turns == 0 {
            return None;
        }
        let turn_taking_entropy = turn_taking_entropy(&turns);
        let predictions = state.peer_predictions.iter().copied().collect::<Vec<_>>();
        let outcomes = state.peer_outcomes.iter().copied().collect::<Vec<_>>();
        let peer_prediction_accuracy = peer_prediction_accuracy(&predictions, &outcomes);
        let citation_reciprocity = citation_reciprocity(
            &[state.survived_citations as f64],
            &[state.verified_citations as f64],
        );
        let hdc_diversity = hdc_diversity(&similarities);
        let metrics = CohortMetrics {
            turn_taking_entropy,
            peer_prediction_accuracy,
            citation_reciprocity,
            delivery_rate: delivery_rate(state.delivery_confirmed, state.delivery_dropped),
            hdc_diversity,
        };
        let dominant_agent_share = turns
            .iter()
            .copied()
            .max()
            .map_or(0.0, |dominant| dominant as f64 / total_turns as f64);
        let avg_agent_vitality = state.vitality.values().sum::<f64>() / state.vitality.len() as f64;
        Some(CFactorPayload {
            space: match &self.scope {
                LensScope::Space(space) => space.clone(),
                _ => unreachable!("scope validated by constructor"),
            },
            interval_ms: self.interval_ms,
            c_factor: metrics.composite(&self.weights),
            turn_taking_entropy,
            peer_prediction_accuracy,
            citation_reciprocity,
            hdc_diversity,
            agent_count: state.agents.len(),
            active_agents: state.vitality.len(),
            dominant_agent_share,
            knowledge_flow_edges: state.knowledge_flow_edges,
            avg_agent_vitality,
        })
    }
}

#[async_trait]
impl TelemetryObserve for CollectiveIntelligenceLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let payload = {
            let mut state = self.state.lock();
            self.process_event(&mut state, event)?;
            let Some(payload) = self.snapshot(&state) else {
                return Ok(Vec::new());
            };
            state.eligible_events = checked_add(
                state.eligible_events,
                1,
                "CollectiveIntelligenceLens eligible events",
            )?;
            if !state.eligible_events.is_multiple_of(self.emit_every) {
                return Ok(Vec::new());
            }
            payload
        };
        encode(&self.name, LensPayload::CFactor(payload))
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

fn validate_name(name: String) -> Result<String> {
    if name.trim().is_empty() {
        return Err(config("lens name cannot be empty"));
    }
    if name != name.trim() {
        return Err(config("lens name cannot have surrounding whitespace"));
    }
    Ok(name)
}

fn validate_chained_scope(scope: &LensScope, lens: &str) -> Result<()> {
    match scope {
        LensScope::Lens(upstream) if !upstream.trim().is_empty() => Ok(()),
        _ => Err(config(format!(
            "{lens} scope must be a named upstream Lens"
        ))),
    }
}

fn upstream_name(scope: &LensScope) -> Result<&str> {
    match scope {
        LensScope::Lens(upstream) if !upstream.trim().is_empty() => Ok(upstream),
        _ => Err(config("derived Lens has no valid upstream Lens scope")),
    }
}

fn validate_observes(
    observes: &[ObservableEventKind],
    required: &[ObservableEventKind],
    lens: &str,
) -> Result<()> {
    if observes.contains(&ObservableEventKind::All)
        || required.iter().all(|kind| observes.contains(kind))
    {
        Ok(())
    } else {
        Err(config(format!(
            "{lens} observes must include every required lifecycle family"
        )))
    }
}

fn params_object<P: Serialize>(
    params: &BTreeMap<String, P>,
    allowed: &[&str],
) -> Result<Map<String, Value>> {
    let value = serde_json::to_value(params)
        .map_err(|error| config(format!("failed to serialize Lens params: {error}")))?;
    let object = value
        .as_object()
        .cloned()
        .ok_or_else(|| config("Lens params must serialize as an object"))?;
    if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(config(format!("unknown Lens parameter `{unknown}`")));
    }
    Ok(object)
}

fn required_string(params: &Map<String, Value>, key: &str) -> Result<String> {
    optional_string(params, key)?
        .ok_or_else(|| config(format!("missing required parameter `{key}`")))
}

fn optional_string(params: &Map<String, Value>, key: &str) -> Result<Option<String>> {
    let Some(value) = params.get(key) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| config(format!("parameter `{key}` must be a string")))?;
    if value.trim().is_empty() || value != value.trim() {
        return Err(config(format!(
            "parameter `{key}` must be non-empty without surrounding whitespace"
        )));
    }
    Ok(Some(value.to_owned()))
}

fn u64_param(
    params: &Map<String, Value>,
    key: &str,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64> {
    let value = match params.get(key) {
        Some(value) => value
            .as_u64()
            .ok_or_else(|| config(format!("parameter `{key}` must be an unsigned integer")))?,
        None => default,
    };
    if !(min..=max).contains(&value) {
        return Err(config(format!(
            "parameter `{key}` must be in {min}..={max}"
        )));
    }
    Ok(value)
}

fn usize_param(
    params: &Map<String, Value>,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize> {
    let value = u64_param(params, key, default as u64, min as u64, max as u64)?;
    usize::try_from(value).map_err(|_| config(format!("parameter `{key}` is too large")))
}

fn f64_param(
    params: &Map<String, Value>,
    key: &str,
    default: f64,
    min: f64,
    max: f64,
) -> Result<f64> {
    let value = match params.get(key) {
        Some(value) => value
            .as_f64()
            .ok_or_else(|| config(format!("parameter `{key}` must be numeric")))?,
        None => default,
    };
    ensure_finite(value, key)?;
    if !(min..=max).contains(&value) {
        return Err(config(format!(
            "parameter `{key}` is outside its allowed range"
        )));
    }
    Ok(value)
}

fn interval_param(params: &Map<String, Value>, default: u64) -> Result<u64> {
    if params.contains_key("interval") && params.contains_key("interval_ms")
        || params.contains_key("window") && params.contains_key("window_ms")
    {
        return Err(config(
            "duration must use only one of the string or millisecond parameters",
        ));
    }
    if let Some(value) = params
        .get("interval_ms")
        .or_else(|| params.get("window_ms"))
    {
        let milliseconds = value
            .as_u64()
            .ok_or_else(|| config("duration millisecond parameter must be an unsigned integer"))?;
        return (milliseconds > 0)
            .then_some(milliseconds)
            .ok_or_else(|| config("duration must be greater than zero"));
    }
    let Some(value) = params.get("interval").or_else(|| params.get("window")) else {
        return Ok(default);
    };
    let value = value
        .as_str()
        .ok_or_else(|| config("duration parameter must be a string"))?;
    parse_duration_ms(value)
}

fn parse_duration_ms(value: &str) -> Result<u64> {
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .ok_or_else(|| config("duration requires a unit: ms, s, m, h, or d"))?;
    let (amount, unit) = value.split_at(split);
    if amount.is_empty() || unit.is_empty() || value != value.trim() {
        return Err(config(
            "duration must be a positive integer followed by ms, s, m, h, or d",
        ));
    }
    let amount = amount
        .parse::<u64>()
        .map_err(|_| config("duration amount is not an unsigned integer"))?;
    let factor = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err(config("duration unit must be ms, s, m, h, or d")),
    };
    if amount == 0 {
        return Err(config("duration must be greater than zero"));
    }
    amount
        .checked_mul(factor)
        .ok_or_else(|| config("duration exceeds u64 milliseconds"))
}

fn decode_upstream(signal: &Signal, upstream: &str) -> Result<Option<LensSignalEnvelope>> {
    match LensSignalEnvelope::from_signal(signal) {
        Ok(envelope) if envelope.source_lens == upstream => Ok(Some(envelope)),
        Ok(_) => Ok(None),
        Err(TelemetryProjectionError::UnexpectedSignalKind(_)) => Ok(None),
        Err(error) => Err(config(format!(
            "invalid canonical upstream Lens output: {error}"
        ))),
    }
}

fn encode(name: &str, payload: LensPayload) -> Result<Vec<Signal>> {
    LensSignalEnvelope::new(name, payload)
        .to_signal()
        .map(|signal| vec![signal])
        .map_err(|error| config(format!("failed to encode Lens output: {error}")))
}

fn metric_value(payload: &LensPayload, metric: &str) -> Option<f64> {
    match payload {
        LensPayload::CostReport(value) => match metric {
            "total_usd" => Some(value.total_usd),
            "total_tokens" => Some(value.total_tokens as f64),
            "input_tokens" => Some(value.input_tokens as f64),
            "output_tokens" => Some(value.output_tokens as f64),
            "cumulative_usd" => Some(value.cumulative_usd),
            "budget_remaining" => value.budget_remaining,
            "vitality" => value.vitality,
            _ => None,
        },
        LensPayload::Latency(value) => match metric {
            "count" => Some(value.count as f64),
            "p50_ms" => Some(value.p50_ms as f64),
            "p95_ms" => Some(value.p95_ms as f64),
            "p99_ms" => Some(value.p99_ms as f64),
            "mean_ms" => Some(value.mean_ms as f64),
            _ => None,
        },
        LensPayload::Quality(value) => match metric {
            "total_verifications" => Some(value.total_verifications as f64),
            "pre_verify_vetoes" => Some(value.pre_verify_vetoes as f64),
            "post_verify_passed" => Some(value.post_verify_passed as f64),
            "post_verify_failed" => Some(value.post_verify_failed as f64),
            "pass_rate" => Some(value.pass_rate),
            "avg_reward" => Some(value.avg_reward),
            "hard_criteria_failures" => Some(value.hard_criteria_failures as f64),
            _ => None,
        },
        LensPayload::Efficiency(value) => match metric {
            "tasks_completed" => Some(value.tasks_completed as f64),
            "tokens_per_task" => Some(value.tokens_per_task),
            "usd_per_task" => Some(value.usd_per_task),
            "quality_per_usd" => Some(value.quality_per_usd),
            "t0_hit_rate" => Some(value.t0_hit_rate),
            "t1_hit_rate" => Some(value.t1_hit_rate),
            "t2_hit_rate" => Some(value.t2_hit_rate),
            "avg_prediction_error" => Some(value.avg_prediction_error),
            "vitality" => Some(value.vitality),
            _ => None,
        },
        LensPayload::Error(value) => match metric {
            "total_errors" => Some(value.total_errors as f64),
            "retry_count" => Some(value.retry_count as f64),
            "retry_success_rate" => Some(value.retry_success_rate),
            "error_rate" => Some(value.error_rate),
            _ => None,
        },
        LensPayload::Drift(value) => match metric {
            "total_entries" => Some(value.total_entries as f64),
            "avg_balance" => Some(value.avg_balance),
            "balance_delta" => Some(value.balance_delta),
            "promotion_rate" => Some(value.promotion_rate),
            "demotion_rate" => Some(value.demotion_rate),
            "cold_entries" => Some(value.cold_entries as f64),
            "anti_knowledge_count" => Some(value.anti_knowledge_count as f64),
            "heuristic_calibration_avg" => Some(value.heuristic_calibration_avg),
            _ => None,
        },
        LensPayload::BudgetAlert(value) => match metric {
            "budget_total" => Some(value.budget_total),
            "budget_spent" => Some(value.budget_spent),
            "budget_remaining" => Some(value.budget_remaining),
            "vitality" => Some(value.vitality),
            "projected_exhaustion_ms" => value.projected_exhaustion_ms.map(|v| v as f64),
            "burn_rate" => Some(value.burn_rate),
            _ => None,
        },
        LensPayload::Trend(value) => match metric {
            "slope" => Some(value.slope),
            "ema" => Some(value.ema),
            "ema_previous" => Some(value.ema_previous),
            "r_squared" => Some(value.r_squared),
            "data_points" => Some(value.data_points as f64),
            _ => None,
        },
        LensPayload::Anomaly(value) => match metric {
            "observed_value" => Some(value.observed_value),
            "expected_value" => Some(value.expected_value),
            "deviation" => Some(value.deviation),
            _ => None,
        },
        LensPayload::Usage(value) => usage_metric_value(value, metric),
        LensPayload::CFactor(value) => c_factor_metric_value(value, metric),
    }
}

fn usage_metric_value(value: &UsagePayload, metric: &str) -> Option<f64> {
    match metric {
        "cell_runs" => Some(value.cell_runs as f64),
        "graph_runs" => Some(value.graph_runs as f64),
        "trigger_fires" => Some(value.trigger_fires as f64),
        "total_cost_usd" => Some(value.total_cost_usd),
        "total_duration_ms" => Some(value.total_duration_ms as f64),
        _ => None,
    }
}

fn c_factor_metric_value(value: &CFactorPayload, metric: &str) -> Option<f64> {
    match metric {
        "c_factor" => Some(value.c_factor),
        "turn_taking_entropy" => Some(value.turn_taking_entropy),
        "peer_prediction_accuracy" => Some(value.peer_prediction_accuracy),
        "citation_reciprocity" => Some(value.citation_reciprocity),
        "hdc_diversity" => Some(value.hdc_diversity),
        "agent_count" => Some(value.agent_count as f64),
        "active_agents" => Some(value.active_agents as f64),
        "dominant_agent_share" => Some(value.dominant_agent_share),
        "knowledge_flow_edges" => Some(value.knowledge_flow_edges as f64),
        "avg_agent_vitality" => Some(value.avg_agent_vitality),
        _ => None,
    }
}

fn linear_regression(values: &[f64]) -> (f64, f64) {
    let count = values.len() as f64;
    let mean_x = (count - 1.0) / 2.0;
    let mean_y = values.iter().sum::<f64>() / count;
    let mut covariance = 0.0;
    let mut variance_x = 0.0;
    let mut variance_y = 0.0;
    for (index, value) in values.iter().enumerate() {
        let x_delta = index as f64 - mean_x;
        let y_delta = *value - mean_y;
        covariance = x_delta.mul_add(y_delta, covariance);
        variance_x = x_delta.mul_add(x_delta, variance_x);
        variance_y = y_delta.mul_add(y_delta, variance_y);
    }
    let slope = if variance_x <= f64::EPSILON {
        0.0
    } else {
        covariance / variance_x
    };
    let r_squared = if variance_y <= f64::EPSILON {
        0.0
    } else {
        (covariance.powi(2) / (variance_x * variance_y)).clamp(0.0, 1.0)
    };
    (slope, r_squared)
}

fn classify_anomaly(
    baseline: &[f64],
    observed: f64,
    moderate: f64,
    severe: f64,
    critical: f64,
) -> Option<(f64, f64, AnomalyDirection, AnomalyLevel)> {
    let expected = baseline.iter().sum::<f64>() / baseline.len() as f64;
    let deviation = observed - expected;
    let variance = baseline
        .iter()
        .map(|value| (*value - expected).powi(2))
        .sum::<f64>()
        / baseline.len() as f64;
    let standard_deviation = variance.sqrt();
    let z_score = if standard_deviation <= f64::EPSILON {
        if deviation.abs() <= f64::EPSILON {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        deviation.abs() / standard_deviation
    };

    let mut sorted = baseline.to_vec();
    sorted.sort_by(f64::total_cmp);
    let q1 = quantile(&sorted, 0.25);
    let q3 = quantile(&sorted, 0.75);
    let iqr = q3 - q1;
    let iqr_score = if iqr <= f64::EPSILON {
        if observed < q1 || observed > q3 {
            f64::INFINITY
        } else {
            0.0
        }
    } else if observed > q3 {
        (observed - q3) / iqr
    } else if observed < q1 {
        (q1 - observed) / iqr
    } else {
        0.0
    };
    let score = z_score.max(iqr_score);
    if score < moderate {
        return None;
    }
    let severity = if score >= critical {
        AnomalyLevel::Critical
    } else if score >= severe {
        AnomalyLevel::Severe
    } else {
        AnomalyLevel::Moderate
    };
    let direction = if deviation >= 0.0 {
        AnomalyDirection::Above
    } else {
        AnomalyDirection::Below
    };
    Some((expected, deviation, direction, severity))
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        (position - lower as f64).mul_add(sorted[upper] - sorted[lower], sorted[lower])
    }
}

fn parse_evidence_unit(value: &str, tag: &str) -> Result<f64> {
    let value = value
        .parse::<f64>()
        .map_err(|_| config(format!("tag `{tag}` must contain a number in [0, 1]")))?;
    ensure_finite(value, tag)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(config(format!(
            "tag `{tag}` must contain a number in [0, 1]"
        )));
    }
    Ok(value)
}

fn parse_optional_count(value: Option<&str>, tag: &str) -> Result<u64> {
    value.map_or(Ok(0), |value| {
        value
            .parse::<u64>()
            .map_err(|_| config(format!("tag `{tag}` must contain an unsigned integer")))
    })
}

fn ensure_finite(value: f64, field: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(config(format!("{field} must be finite")))
    }
}

fn ensure_non_negative_finite(value: f64, field: &str) -> Result<()> {
    ensure_finite(value, field)?;
    if value < 0.0 {
        Err(config(format!("{field} must be non-negative")))
    } else {
        Ok(())
    }
}

fn checked_add(left: u64, right: u64, field: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| config(format!("{field} overflow")))
}

fn config(message: impl std::fmt::Display) -> RokoError {
    RokoError::config(message)
}
