//! Stateful built-in Latency, Quality, and Efficiency telemetry Lenses.
//!
//! The portable event contract has no timestamps, token/cache accounting, or
//! hard-criterion marker. Consequently these reducers report `interval_ms = 0`
//! (an event-count window), and leave unavailable numeric dimensions at zero
//! instead of inventing measurements.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use parking_lot::Mutex;
use roko_core::{
    EfficiencyPayload, LatencyPayload, LensScope, ObservableEvent, ObservableEventKind,
    PassFailCounts, QualityPayload, Result, RokoError, Signal, TelemetryObserve,
};
use serde::Serialize;
use serde_json::Value;

use crate::{LensPayload, LensSignalEnvelope};

const DEFAULT_WINDOW_SIZE: usize = 1_024;
const DEFAULT_MAX_TARGETS: usize = 128;
const DEFAULT_MAX_AGENTS: usize = 256;
const MAX_WINDOW_SIZE: usize = 100_000;
const MAX_CARDINALITY: usize = 4_096;
const MAX_BUFFERED_SAMPLES: usize = 500_000;

/// Rolling duration percentiles for Cell and Graph completion events.
pub struct LatencyLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    window_size: usize,
    max_targets: usize,
    samples: Mutex<BTreeMap<String, VecDeque<u64>>>,
}

impl LatencyLens {
    /// Construct a Lens from its declarative registration fields.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: P,
    ) -> Result<Self> {
        let name = validate_registration(
            name,
            &observes,
            &[
                ObservableEventKind::CellLifecycle,
                ObservableEventKind::GraphLifecycle,
            ],
            "LatencyLens",
        )?;
        let mut params = parse_params(params, "LatencyLens")?;
        let window_size = take_usize(
            &mut params,
            "window_size",
            DEFAULT_WINDOW_SIZE,
            MAX_WINDOW_SIZE,
            "LatencyLens",
        )?;
        let max_targets = take_usize(
            &mut params,
            "max_targets",
            DEFAULT_MAX_TARGETS,
            MAX_CARDINALITY,
            "LatencyLens",
        )?;
        validate_memory_bound(window_size, max_targets, "LatencyLens")?;
        reject_unknown_params(params, "LatencyLens")?;
        Ok(Self {
            name,
            scope,
            observes,
            window_size,
            max_targets,
            samples: Mutex::new(BTreeMap::new()),
        })
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for LatencyLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        if !event.matches_any(&self.observes) {
            return Ok(Vec::new());
        }
        let (target, duration_ms) = match event {
            ObservableEvent::CellCompleted {
                block, duration_ms, ..
            } => (format!("cell:{block}"), *duration_ms),
            ObservableEvent::GraphNodeCompleted {
                graph,
                node,
                duration_ms,
                ..
            } => (format!("graph:{graph}/node:{node}"), *duration_ms),
            ObservableEvent::GraphCompleted {
                graph, duration_ms, ..
            } => (format!("graph:{graph}"), *duration_ms),
            _ => return Ok(Vec::new()),
        };

        let payload = {
            let mut states = self.samples.lock();
            if !states.contains_key(&target) && states.len() >= self.max_targets {
                return Err(RokoError::invalid(format!(
                    "LatencyLens `{}` exceeded max_targets ({})",
                    self.name, self.max_targets
                )));
            }
            let samples = states.entry(target.clone()).or_default();
            push_bounded(samples, duration_ms, self.window_size);
            latency_payload(target, samples)
        };
        encode(&self.name, LensPayload::Latency(payload), "LatencyLens")
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

#[derive(Clone)]
enum QualitySample {
    Pre {
        vetoed: bool,
    },
    Final {
        gate: String,
        passed: bool,
        reward: f64,
    },
}

/// Rolling verification outcomes, reward, and per-gate pass/fail counts.
pub struct QualityLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    window_size: usize,
    pass_rate_warn: Option<f64>,
    samples: Mutex<VecDeque<QualitySample>>,
}

impl QualityLens {
    /// Construct a Lens from its declarative registration fields.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: P,
    ) -> Result<Self> {
        let name = validate_registration(
            name,
            &observes,
            &[
                ObservableEventKind::VerifyLifecycle,
                ObservableEventKind::SignalLifecycle,
            ],
            "QualityLens",
        )?;
        let mut params = parse_params(params, "QualityLens")?;
        let window_size = take_usize(
            &mut params,
            "window_size",
            DEFAULT_WINDOW_SIZE,
            MAX_WINDOW_SIZE,
            "QualityLens",
        )?;
        let pass_rate_warn = take_optional_rate(&mut params, "pass_rate_warn", "QualityLens")?;
        reject_unknown_params(params, "QualityLens")?;
        Ok(Self {
            name,
            scope,
            observes,
            window_size,
            pass_rate_warn,
            samples: Mutex::new(VecDeque::new()),
        })
    }

    /// Configured warning threshold. Alert emission is intentionally owned by
    /// a policy/alert Lens rather than this passive measurement Lens.
    #[must_use]
    pub const fn pass_rate_warn(&self) -> Option<f64> {
        self.pass_rate_warn
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for QualityLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        if !event.matches_any(&self.observes) {
            return Ok(Vec::new());
        }
        let sample = match event {
            ObservableEvent::VerifyPreResult { verdict, .. } if !verdict.skipped => {
                QualitySample::Pre {
                    vetoed: !verdict.passed,
                }
            }
            ObservableEvent::VerifyPostResult {
                verdict, reward, ..
            } if !verdict.skipped => final_quality_sample(verdict, *reward)?,
            ObservableEvent::SignalVerified(_, verdict) if !verdict.skipped => {
                final_quality_sample(verdict, f64::from(verdict.score))?
            }
            _ => return Ok(Vec::new()),
        };

        let payload = {
            let mut samples = self.samples.lock();
            push_bounded(&mut samples, sample, self.window_size);
            quality_payload(scope_target(&self.scope), &samples)
        };
        encode(&self.name, LensPayload::Quality(payload), "QualityLens")
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

#[derive(Default)]
struct AgentEfficiencyState {
    task_costs: VecDeque<f64>,
    prediction_errors: VecDeque<f64>,
    vitality: Option<f64>,
    vitality_phase: Option<String>,
}

/// Rolling completion cost plus the Agent vitality and prediction data that is
/// present in the portable lifecycle protocol.
pub struct EfficiencyLens {
    name: String,
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    window_size: usize,
    max_agents: usize,
    agents: Mutex<BTreeMap<String, AgentEfficiencyState>>,
}

impl EfficiencyLens {
    /// Construct a Lens from its declarative registration fields.
    pub fn new<P: Serialize>(
        name: impl Into<String>,
        scope: LensScope,
        observes: Vec<ObservableEventKind>,
        params: P,
    ) -> Result<Self> {
        let name = validate_registration(
            name,
            &observes,
            &[
                ObservableEventKind::CellLifecycle,
                ObservableEventKind::AgentLifecycle,
            ],
            "EfficiencyLens",
        )?;
        let mut params = parse_params(params, "EfficiencyLens")?;
        let window_size = take_usize(
            &mut params,
            "window_size",
            DEFAULT_WINDOW_SIZE,
            MAX_WINDOW_SIZE,
            "EfficiencyLens",
        )?;
        let max_agents = take_usize(
            &mut params,
            "max_agents",
            DEFAULT_MAX_AGENTS,
            MAX_CARDINALITY,
            "EfficiencyLens",
        )?;
        validate_memory_bound(window_size, max_agents, "EfficiencyLens")?;
        reject_unknown_params(params, "EfficiencyLens")?;
        Ok(Self {
            name,
            scope,
            observes,
            window_size,
            max_agents,
            agents: Mutex::new(BTreeMap::new()),
        })
    }
}

#[async_trait::async_trait]
impl TelemetryObserve for EfficiencyLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        if !event.matches_any(&self.observes) {
            return Ok(Vec::new());
        }
        let update = efficiency_update(event, &self.scope)?;
        let Some((agent, update)) = update else {
            return Ok(Vec::new());
        };

        let payload = {
            let mut agents = self.agents.lock();
            if !agents.contains_key(&agent) && agents.len() >= self.max_agents {
                return Err(RokoError::invalid(format!(
                    "EfficiencyLens `{}` exceeded max_agents ({})",
                    self.name, self.max_agents
                )));
            }
            let state = agents.entry(agent.clone()).or_default();
            match update {
                EfficiencyUpdate::TaskCost(cost) => {
                    push_bounded(&mut state.task_costs, cost, self.window_size);
                }
                EfficiencyUpdate::Prediction { error, vitality } => {
                    push_bounded(&mut state.prediction_errors, error, self.window_size);
                    if let Some(vitality) = vitality {
                        state.vitality = Some(vitality);
                    }
                }
                EfficiencyUpdate::Vitality(vitality) => state.vitality = Some(vitality),
                EfficiencyUpdate::Phase(phase) => state.vitality_phase = Some(phase),
            }
            efficiency_payload(agent, state)
        };
        encode(
            &self.name,
            LensPayload::Efficiency(payload),
            "EfficiencyLens",
        )
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

enum EfficiencyUpdate {
    TaskCost(f64),
    Prediction { error: f64, vitality: Option<f64> },
    Vitality(f64),
    Phase(String),
}

fn efficiency_update(
    event: &ObservableEvent,
    scope: &LensScope,
) -> Result<Option<(String, EfficiencyUpdate)>> {
    match event {
        ObservableEvent::CellCompleted { cost_usd, .. } => {
            validate_non_negative(*cost_usd, "CellCompleted.cost_usd")?;
            Ok(named_agent(scope).map(|agent| (agent, EfficiencyUpdate::TaskCost(*cost_usd))))
        }
        ObservableEvent::CellCalibrationReceived { error, .. } => {
            validate_non_negative(*error, "CellCalibrationReceived.error")?;
            Ok(named_agent(scope).map(|agent| {
                (
                    agent,
                    EfficiencyUpdate::Prediction {
                        error: *error,
                        vitality: None,
                    },
                )
            }))
        }
        ObservableEvent::AgentTick {
            agent,
            prediction_error,
            vitality,
            ..
        } => {
            validate_agent(agent)?;
            validate_non_negative(*prediction_error, "AgentTick.prediction_error")?;
            validate_rate(*vitality, "AgentTick.vitality")?;
            Ok(Some((
                agent.clone(),
                EfficiencyUpdate::Prediction {
                    error: *prediction_error,
                    vitality: Some(*vitality),
                },
            )))
        }
        ObservableEvent::AgentBudgetUpdate {
            agent, vitality, ..
        } => {
            validate_agent(agent)?;
            validate_rate(*vitality, "AgentBudgetUpdate.vitality")?;
            Ok(Some((agent.clone(), EfficiencyUpdate::Vitality(*vitality))))
        }
        ObservableEvent::AgentPhaseChange {
            agent, new_phase, ..
        } => {
            validate_agent(agent)?;
            if new_phase.trim().is_empty() {
                return Err(RokoError::invalid("AgentPhaseChange.new_phase is empty"));
            }
            Ok(Some((
                agent.clone(),
                EfficiencyUpdate::Phase(new_phase.clone()),
            )))
        }
        _ => Ok(None),
    }
}

fn latency_payload(target: String, samples: &VecDeque<u64>) -> LatencyPayload {
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort_unstable();
    let sum = sorted.iter().copied().map(u128::from).sum::<u128>();
    let count = u64::try_from(sorted.len()).unwrap_or(u64::MAX);
    let mean_ms = if sorted.is_empty() {
        0
    } else {
        u64::try_from(sum / sorted.len() as u128).unwrap_or(u64::MAX)
    };
    LatencyPayload {
        target,
        interval_ms: 0,
        count,
        p50_ms: percentile(&sorted, 50),
        p95_ms: percentile(&sorted, 95),
        p99_ms: percentile(&sorted, 99),
        mean_ms,
    }
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = percentile.saturating_mul(sorted.len()).div_ceil(100);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn final_quality_sample(verdict: &roko_core::Verdict, reward: f64) -> Result<QualitySample> {
    if verdict.gate.trim().is_empty() {
        return Err(RokoError::invalid("verification verdict gate is empty"));
    }
    if !reward.is_finite() {
        return Err(RokoError::invalid("verification reward is not finite"));
    }
    Ok(QualitySample::Final {
        gate: verdict.gate.clone(),
        passed: verdict.passed,
        reward,
    })
}

fn quality_payload(target: String, samples: &VecDeque<QualitySample>) -> QualityPayload {
    let mut pre_verify_vetoes = 0_u64;
    let mut post_verify_passed = 0_u64;
    let mut post_verify_failed = 0_u64;
    let mut reward_total = 0.0;
    let mut reward_count = 0_u64;
    let mut rung_breakdown = BTreeMap::<String, PassFailCounts>::new();
    for sample in samples {
        match sample {
            QualitySample::Pre { vetoed } => pre_verify_vetoes += u64::from(*vetoed),
            QualitySample::Final {
                gate,
                passed,
                reward,
            } => {
                let counts = rung_breakdown.entry(gate.clone()).or_default();
                if *passed {
                    post_verify_passed += 1;
                    counts.passed += 1;
                } else {
                    post_verify_failed += 1;
                    counts.failed += 1;
                }
                reward_total += reward;
                reward_count += 1;
            }
        }
    }
    let post_total = post_verify_passed.saturating_add(post_verify_failed);
    QualityPayload {
        target,
        interval_ms: 0,
        // Pre and post events have no correlation ID, so the terminal count is
        // the only non-duplicating verification total available.
        total_verifications: post_total,
        pre_verify_vetoes,
        post_verify_passed,
        post_verify_failed,
        pass_rate: ratio(post_verify_passed, post_total),
        avg_reward: if reward_count == 0 {
            0.0
        } else {
            reward_total / reward_count as f64
        },
        // ObservableEvent has no hard-criterion flag; evidence strings are not
        // interpreted heuristically because doing so would fabricate semantics.
        hard_criteria_failures: 0,
        rung_breakdown,
    }
}

fn efficiency_payload(agent: String, state: &AgentEfficiencyState) -> EfficiencyPayload {
    let tasks_completed = u64::try_from(state.task_costs.len()).unwrap_or(u64::MAX);
    EfficiencyPayload {
        agent,
        interval_ms: 0,
        tasks_completed,
        // No token or cache-tier counts exist on ObservableEvent.
        tokens_per_task: 0.0,
        usd_per_task: mean(state.task_costs.iter().copied()),
        // No joined Quality payload is present on raw lifecycle events.
        quality_per_usd: 0.0,
        t0_hit_rate: 0.0,
        t1_hit_rate: 0.0,
        t2_hit_rate: 0.0,
        avg_prediction_error: mean(state.prediction_errors.iter().copied()),
        vitality: state.vitality.unwrap_or(0.0),
        vitality_phase: state
            .vitality_phase
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    }
}

fn validate_registration(
    name: impl Into<String>,
    observes: &[ObservableEventKind],
    allowed: &[ObservableEventKind],
    lens: &str,
) -> Result<String> {
    let name = name.into();
    if name.trim().is_empty() || name != name.trim() {
        return Err(RokoError::config(format!(
            "{lens} name must be non-empty without surrounding whitespace"
        )));
    }
    if observes.is_empty() {
        return Err(RokoError::config(format!(
            "{lens} must observe at least one event family"
        )));
    }
    let unique = observes.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != observes.len() {
        return Err(RokoError::config(format!(
            "{lens} event filters contain duplicates"
        )));
    }
    if observes.contains(&ObservableEventKind::All) && observes.len() != 1 {
        return Err(RokoError::config(format!(
            "{lens} `All` event filter must be declared alone"
        )));
    }
    if !observes.contains(&ObservableEventKind::All)
        && observes.iter().any(|kind| !allowed.contains(kind))
    {
        return Err(RokoError::config(format!(
            "{lens} received an unsupported event-family filter"
        )));
    }
    Ok(name)
}

fn parse_params<P: Serialize>(params: P, lens: &str) -> Result<BTreeMap<String, Value>> {
    let value = serde_json::to_value(params).map_err(|error| {
        RokoError::config(format!("{lens} params are not serializable: {error}"))
    })?;
    let Value::Object(params) = value else {
        return Err(RokoError::config(format!(
            "{lens} params must be a TOML table"
        )));
    };
    Ok(params.into_iter().collect())
}

fn take_usize(
    params: &mut BTreeMap<String, Value>,
    key: &str,
    default: usize,
    max: usize,
    lens: &str,
) -> Result<usize> {
    let Some(value) = params.remove(key) else {
        return Ok(default);
    };
    let value = value.as_u64().ok_or_else(|| {
        RokoError::config(format!("{lens} param `{key}` must be a positive integer"))
    })?;
    let value = usize::try_from(value)
        .map_err(|_| RokoError::config(format!("{lens} param `{key}` does not fit in usize")))?;
    if value == 0 || value > max {
        return Err(RokoError::config(format!(
            "{lens} param `{key}` must be in 1..={max}"
        )));
    }
    Ok(value)
}

fn take_optional_rate(
    params: &mut BTreeMap<String, Value>,
    key: &str,
    lens: &str,
) -> Result<Option<f64>> {
    let Some(value) = params.remove(key) else {
        return Ok(None);
    };
    let value = value.as_f64().ok_or_else(|| {
        RokoError::config(format!(
            "{lens} param `{key}` must be a finite number in 0..=1"
        ))
    })?;
    validate_rate(value, &format!("{lens} param `{key}`"))?;
    Ok(Some(value))
}

fn reject_unknown_params(params: BTreeMap<String, Value>, lens: &str) -> Result<()> {
    if params.is_empty() {
        Ok(())
    } else {
        Err(RokoError::config(format!(
            "{lens} has unknown param{}: {}",
            if params.len() == 1 { "" } else { "s" },
            params.keys().cloned().collect::<Vec<_>>().join(", ")
        )))
    }
}

fn validate_memory_bound(window: usize, cardinality: usize, lens: &str) -> Result<()> {
    if window.saturating_mul(cardinality) > MAX_BUFFERED_SAMPLES {
        return Err(RokoError::config(format!(
            "{lens} window_size * cardinality must not exceed {MAX_BUFFERED_SAMPLES}"
        )));
    }
    Ok(())
}

fn validate_agent(agent: &str) -> Result<()> {
    if agent.trim().is_empty() {
        Err(RokoError::invalid("agent identifier is empty"))
    } else {
        Ok(())
    }
}

fn validate_non_negative(value: f64, field: &str) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(RokoError::invalid(format!(
            "{field} must be finite and non-negative"
        )))
    }
}

fn validate_rate(value: f64, field: &str) -> Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(RokoError::invalid(format!(
            "{field} must be finite and in 0..=1"
        )))
    }
}

fn named_agent(scope: &LensScope) -> Option<String> {
    match scope {
        LensScope::Agent(agent) if !agent.is_empty() => Some(agent.clone()),
        _ => None,
    }
}

fn scope_target(scope: &LensScope) -> String {
    match scope {
        LensScope::Cell(name) => format!("cell:{}", wildcard(name)),
        LensScope::Graph(name) => format!("graph:{}", wildcard(name)),
        LensScope::Agent(name) => format!("agent:{}", wildcard(name)),
        LensScope::Space(name) => format!("space:{}", wildcard(name)),
        LensScope::Lens(name) => format!("lens:{}", wildcard(name)),
        LensScope::Global => "global".to_string(),
    }
}

fn wildcard(name: &str) -> &str {
    if name.is_empty() { "*" } else { name }
}

fn push_bounded<T>(values: &mut VecDeque<T>, value: T, limit: usize) {
    if values.len() == limit {
        values.pop_front();
    }
    values.push_back(value);
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0_u64), |(sum, count), value| (sum + value, count + 1));
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn encode(name: &str, payload: LensPayload, lens: &str) -> Result<Vec<Signal>> {
    LensSignalEnvelope::new(name, payload)
        .to_signal()
        .map(|signal| vec![signal])
        .map_err(|error| RokoError::config(format!("{lens} envelope encoding failed: {error}")))
}
