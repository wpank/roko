//! Deterministic materialization of typed Lens output into StateHub projections.
//!
//! [`Signal`] has a typed body and custom kind, but no topic field. This module's
//! adapter contract therefore carries the canonical topic in both the JSON
//! envelope and a routing tag. Consumers validate that the two agree before
//! applying any payload.

use std::collections::{BTreeMap, BTreeSet};

use roko_core::telemetry_observe::{
    CFactorPayload, CostReportPayload, EfficiencyPayload, LatencyPayload, QualityPayload,
};
use roko_core::telemetry_projections::{
    ActiveTasksProjection, AgentVitalityProjection, AgentVitalitySnapshot, CFactorProjection,
    CohortHealthProjection, CostMeterProjection, GatePipelineProjection, KnowledgeHealthProjection,
    RungSnapshot,
};
use roko_core::{
    AnomalyPayload, Body, BudgetAlertPayload, DriftPayload, ErrorPayload, Kind, Signal,
    TrendDirection, TrendPayload, UsagePayload,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::StateHubSender;

/// Current wire version of [`LensSignalEnvelope`].
pub const LENS_SIGNAL_SCHEMA_VERSION: u16 = 1;
/// Custom [`Kind`] assigned to every version-one Lens output Signal.
pub const LENS_SIGNAL_KIND: &str = "roko.telemetry.lens.output.v1";
/// Signal tag containing the canonical Lens payload topic.
pub const LENS_SIGNAL_TOPIC_TAG: &str = "telemetry.topic";
/// Signal tag containing the emitting Lens instance name.
pub const LENS_SIGNAL_SOURCE_TAG: &str = "telemetry.source_lens";

/// StateHub projection ID for cohort health.
pub const COHORT_HEALTH: &str = "cohort_health";
/// StateHub projection ID for active tasks.
pub const ACTIVE_TASKS: &str = "active_tasks";
/// StateHub projection ID for the gate pipeline.
pub const GATE_PIPELINE: &str = "gate_pipeline";
/// StateHub projection ID for cost and budget state.
pub const COST_METER: &str = "cost_meter";
/// StateHub projection ID for knowledge health.
pub const KNOWLEDGE_HEALTH: &str = "knowledge_health";
/// StateHub projection ID for collective intelligence.
pub const C_FACTOR: &str = "c_factor";
/// StateHub projection ID for per-agent vitality.
pub const AGENT_VITALITY: &str = "agent_vitality";

/// All typed payloads currently emitted by built-in Lenses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "payload_type", content = "payload", rename_all = "snake_case")]
pub enum LensPayload {
    /// CostLens output.
    CostReport(CostReportPayload),
    /// LatencyLens output.
    Latency(LatencyPayload),
    /// QualityLens output.
    Quality(QualityPayload),
    /// EfficiencyLens output.
    Efficiency(EfficiencyPayload),
    /// ErrorLens output.
    Error(ErrorPayload),
    /// DriftLens output.
    Drift(DriftPayload),
    /// BudgetLens output.
    BudgetAlert(BudgetAlertPayload),
    /// TrendLens output.
    Trend(TrendPayload),
    /// AnomalyLens output.
    Anomaly(AnomalyPayload),
    /// UsageLens output.
    Usage(UsagePayload),
    /// CollectiveIntelligenceLens output.
    CFactor(CFactorPayload),
}

impl LensPayload {
    #[must_use]
    /// Stable snake-case payload discriminator used on the wire.
    pub const fn payload_type(&self) -> &'static str {
        match self {
            Self::CostReport(_) => "cost_report",
            Self::Latency(_) => "latency",
            Self::Quality(_) => "quality",
            Self::Efficiency(_) => "efficiency",
            Self::Error(_) => "error",
            Self::Drift(_) => "drift",
            Self::BudgetAlert(_) => "budget_alert",
            Self::Trend(_) => "trend",
            Self::Anomaly(_) => "anomaly",
            Self::Usage(_) => "usage",
            Self::CFactor(_) => "c_factor",
        }
    }

    #[must_use]
    /// Canonical versioned topic for this payload type.
    pub const fn topic(&self) -> &'static str {
        match self {
            Self::CostReport(_) => "telemetry.lens.cost_report.v1",
            Self::Latency(_) => "telemetry.lens.latency.v1",
            Self::Quality(_) => "telemetry.lens.quality.v1",
            Self::Efficiency(_) => "telemetry.lens.efficiency.v1",
            Self::Error(_) => "telemetry.lens.error.v1",
            Self::Drift(_) => "telemetry.lens.drift.v1",
            Self::BudgetAlert(_) => "telemetry.lens.budget_alert.v1",
            Self::Trend(_) => "telemetry.lens.trend.v1",
            Self::Anomaly(_) => "telemetry.lens.anomaly.v1",
            Self::Usage(_) => "telemetry.lens.usage.v1",
            Self::CFactor(_) => "telemetry.lens.c_factor.v1",
        }
    }
}

/// Stable JSON envelope carried by a Lens output [`Signal`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LensSignalEnvelope {
    /// Envelope schema version.
    pub schema_version: u16,
    /// Canonical topic derived from the payload variant.
    pub topic: String,
    /// Name of the Lens instance that emitted the payload.
    pub source_lens: String,
    /// Typed Lens payload and its flattened discriminator.
    #[serde(flatten)]
    pub payload: LensPayload,
}

impl LensSignalEnvelope {
    /// Construct an envelope with the canonical topic for `payload`.
    #[must_use]
    pub fn new(source_lens: impl Into<String>, payload: LensPayload) -> Self {
        Self {
            schema_version: LENS_SIGNAL_SCHEMA_VERSION,
            topic: payload.topic().to_owned(),
            source_lens: source_lens.into(),
            payload,
        }
    }

    /// Encode the envelope as a routable Signal.
    pub fn to_signal(&self) -> Result<Signal, TelemetryProjectionError> {
        self.validate()?;
        let body = Body::from_json(self)
            .map_err(|error| TelemetryProjectionError::MalformedEnvelope(error.to_string()))?;
        Ok(Signal::builder(Kind::Custom(LENS_SIGNAL_KIND.to_owned()))
            .body(body)
            .tag(LENS_SIGNAL_TOPIC_TAG, self.topic.clone())
            .tag(LENS_SIGNAL_SOURCE_TAG, self.source_lens.clone())
            .build())
    }

    /// Decode and validate a Signal at the Lens/StateHub boundary.
    pub fn from_signal(signal: &Signal) -> Result<Self, TelemetryProjectionError> {
        if signal.kind.as_str() != LENS_SIGNAL_KIND {
            return Err(TelemetryProjectionError::UnexpectedSignalKind(
                signal.kind.as_str().to_owned(),
            ));
        }
        let envelope: Self = signal
            .body
            .as_json()
            .map_err(|error| TelemetryProjectionError::MalformedEnvelope(error.to_string()))?;
        envelope.validate()?;
        validate_tag(signal, LENS_SIGNAL_TOPIC_TAG, &envelope.topic)?;
        validate_tag(signal, LENS_SIGNAL_SOURCE_TAG, &envelope.source_lens)?;
        Ok(envelope)
    }

    fn validate(&self) -> Result<(), TelemetryProjectionError> {
        if self.schema_version != LENS_SIGNAL_SCHEMA_VERSION {
            return Err(TelemetryProjectionError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.source_lens.trim().is_empty() {
            return Err(TelemetryProjectionError::EmptySourceLens);
        }
        let expected = self.payload.topic();
        if self.topic != expected {
            return Err(TelemetryProjectionError::TopicMismatch {
                expected: expected.to_owned(),
                actual: self.topic.clone(),
            });
        }
        Ok(())
    }
}

fn validate_tag(
    signal: &Signal,
    key: &'static str,
    expected: &str,
) -> Result<(), TelemetryProjectionError> {
    let actual = signal
        .tag(key)
        .ok_or(TelemetryProjectionError::MissingTag(key))?;
    if actual != expected {
        return Err(TelemetryProjectionError::TagMismatch {
            tag: key,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

/// Validation or materialization failure at the Lens/StateHub boundary.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TelemetryProjectionError {
    /// The Signal does not use the Lens output custom kind.
    #[error("unexpected Signal kind `{0}`")]
    UnexpectedSignalKind(String),
    /// The body is not JSON matching [`LensSignalEnvelope`].
    #[error("malformed Lens envelope: {0}")]
    MalformedEnvelope(String),
    /// The envelope uses an unsupported version.
    #[error("unsupported Lens envelope schema version {0}")]
    UnsupportedSchemaVersion(u16),
    /// The emitter identity is blank.
    #[error("Lens envelope source_lens must not be empty")]
    EmptySourceLens,
    /// The envelope topic does not match its typed payload.
    #[error("Lens topic mismatch: expected `{expected}`, got `{actual}`")]
    TopicMismatch {
        /// Topic derived from the payload variant.
        expected: String,
        /// Topic provided by the envelope.
        actual: String,
    },
    /// A mandatory routing tag is absent.
    #[error("Lens Signal is missing required routing tag `{0}`")]
    MissingTag(&'static str),
    /// A routing tag disagrees with the validated envelope.
    #[error("Lens Signal tag `{tag}` mismatch: expected `{expected}`, got `{actual}`")]
    TagMismatch {
        /// Routing tag name.
        tag: &'static str,
        /// Value copied from the envelope.
        expected: String,
        /// Value present on the Signal.
        actual: String,
    },
    /// A typed projection could not be converted to JSON.
    #[error("failed to serialize projection `{projection_id}`: {message}")]
    ProjectionSerialization {
        /// Stable projection identifier.
        projection_id: &'static str,
        /// Underlying serde failure.
        message: String,
    },
}

/// One ready-to-publish replacement value for `StateHubSender::update_projection`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionUpdate {
    /// Stable StateHub projection ID.
    pub projection_id: &'static str,
    /// Full typed projection serialized for StateHub's value store.
    pub data: Value,
    /// Lens to attribute as a contributor.
    pub source_lens: String,
}

impl ProjectionUpdate {
    /// Publish this update through the existing StateHub API.
    pub fn apply_to(&self, sender: &StateHubSender) {
        sender.update_projection(self.projection_id, self.data.clone(), &self.source_lens);
    }
}

/// Current typed values for all seven core StateHub projections.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TelemetryProjectionState {
    /// Fleet-level health.
    pub cohort_health: CohortHealthProjection,
    /// Active and recently completed tasks.
    pub active_tasks: ActiveTasksProjection,
    /// Verification rung health.
    pub gate_pipeline: GatePipelineProjection,
    /// Aggregate cost and budget state.
    pub cost_meter: CostMeterProjection,
    /// Knowledge-store health.
    pub knowledge_health: KnowledgeHealthProjection,
    /// Collective-intelligence state.
    pub c_factor: CFactorProjection,
    /// Per-agent vitality state.
    pub agent_vitality: AgentVitalityProjection,
}

/// Stateful, deterministic projection reducer for Lens output Signals.
#[derive(Clone, Debug, Default)]
pub struct TelemetryProjectionAggregator {
    state: TelemetryProjectionState,
    agent_vitality: BTreeMap<String, f64>,
    target_pass_rate: BTreeMap<String, f64>,
    target_error_rate: BTreeMap<String, f64>,
    agent_t0_hit_rate: BTreeMap<String, f64>,
    target_spend: BTreeMap<String, f64>,
    target_remaining: BTreeMap<String, f64>,
    model_breakdown: BTreeMap<String, BTreeMap<String, f64>>,
    target_latency: BTreeMap<String, (u64, u64)>,
}

impl TelemetryProjectionAggregator {
    #[must_use]
    /// Create an empty aggregator whose projections all equal their defaults.
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    /// Borrow the current fully typed projection set.
    pub const fn state(&self) -> &TelemetryProjectionState {
        &self.state
    }

    /// Decode one Lens Signal and return every changed StateHub projection.
    pub fn consume(
        &mut self,
        signal: &Signal,
    ) -> Result<Vec<ProjectionUpdate>, TelemetryProjectionError> {
        self.apply_envelope(LensSignalEnvelope::from_signal(signal)?)
    }

    /// Apply a previously decoded envelope.
    pub fn apply_envelope(
        &mut self,
        envelope: LensSignalEnvelope,
    ) -> Result<Vec<ProjectionUpdate>, TelemetryProjectionError> {
        envelope.validate()?;
        let source_lens = envelope.source_lens;
        let mut changed = BTreeSet::new();

        match envelope.payload {
            LensPayload::CostReport(payload) => {
                self.apply_cost(payload);
                changed.extend([COHORT_HEALTH, COST_METER]);
            }
            LensPayload::Latency(payload) => {
                self.apply_latency(payload);
                changed.insert(ACTIVE_TASKS);
            }
            LensPayload::Quality(payload) => {
                self.apply_quality(payload);
                changed.extend([COHORT_HEALTH, GATE_PIPELINE]);
            }
            LensPayload::Efficiency(payload) => {
                self.apply_efficiency(payload);
                changed.extend([AGENT_VITALITY, COHORT_HEALTH]);
            }
            LensPayload::Error(payload) => {
                self.target_error_rate
                    .insert(payload.target, payload.error_rate);
                self.recompute_cohort();
                changed.insert(COHORT_HEALTH);
            }
            LensPayload::Drift(payload) => {
                self.apply_drift(payload);
                changed.insert(KNOWLEDGE_HEALTH);
            }
            LensPayload::BudgetAlert(payload) => {
                let agent_changed = self.apply_budget(payload);
                changed.extend([COHORT_HEALTH, COST_METER]);
                if agent_changed {
                    changed.insert(AGENT_VITALITY);
                }
            }
            LensPayload::Trend(payload) => {
                if let Some(projection_id) = self.apply_trend(payload) {
                    changed.insert(projection_id);
                }
            }
            LensPayload::CFactor(payload) => {
                self.apply_c_factor(payload);
                changed.insert(C_FACTOR);
            }
            LensPayload::Anomaly(_) | LensPayload::Usage(_) => {}
        }

        changed
            .into_iter()
            .map(|projection_id| self.update(projection_id, &source_lens))
            .collect()
    }

    fn apply_cost(&mut self, payload: CostReportPayload) {
        self.target_spend
            .insert(payload.target.clone(), payload.cumulative_usd);
        if let Some(remaining) = payload.budget_remaining {
            self.target_remaining
                .insert(payload.target.clone(), remaining);
        }
        self.model_breakdown
            .insert(payload.target, payload.model_breakdown);
        self.recompute_cost();
        self.recompute_cohort();
    }

    fn apply_latency(&mut self, payload: LatencyPayload) {
        self.target_latency
            .insert(payload.target, (payload.count, payload.mean_ms));
        let (weighted_sum, count) = self.target_latency.values().fold(
            (0_u128, 0_u128),
            |(weighted_sum, count), (sample_count, mean)| {
                (
                    weighted_sum + u128::from(*sample_count) * u128::from(*mean),
                    count + u128::from(*sample_count),
                )
            },
        );
        self.state.active_tasks.avg_task_duration_ms = weighted_sum
            .checked_div(count)
            .map_or(0, |mean| u64::try_from(mean).unwrap_or(u64::MAX));
    }

    fn apply_quality(&mut self, payload: QualityPayload) {
        self.target_pass_rate
            .insert(payload.target, payload.pass_rate);
        self.state.gate_pipeline.rungs = payload
            .rung_breakdown
            .into_iter()
            .map(|(name, counts)| {
                let total = counts.passed.saturating_add(counts.failed);
                RungSnapshot {
                    name,
                    pass_count: counts.passed,
                    fail_count: counts.failed,
                    pass_rate: ratio(counts.passed, total),
                }
            })
            .collect();
        self.state.gate_pipeline.overall_pass_rate = payload.pass_rate;
        self.state.gate_pipeline.avg_reward = payload.avg_reward;
        self.state.gate_pipeline.hard_criteria_fail_rate =
            ratio(payload.hard_criteria_failures, payload.total_verifications);
        self.recompute_cohort();
    }

    fn apply_efficiency(&mut self, payload: EfficiencyPayload) {
        self.agent_t0_hit_rate
            .insert(payload.agent.clone(), payload.t0_hit_rate);
        self.agent_vitality
            .insert(payload.agent.clone(), payload.vitality);
        let agent = self.upsert_agent(&payload.agent);
        agent.vitality = payload.vitality;
        agent.phase = payload.vitality_phase;
        agent.tasks_completed = payload.tasks_completed;
        self.sort_agents();
        self.recompute_cohort();
    }

    fn apply_drift(&mut self, payload: DriftPayload) {
        let heuristic_count = payload
            .tier_distribution
            .get("heuristic")
            .copied()
            .unwrap_or(0);
        self.state.knowledge_health = KnowledgeHealthProjection {
            total_entries: payload.total_entries,
            tier_distribution: payload.tier_distribution,
            avg_balance: payload.avg_balance,
            cold_entries: payload.cold_entries,
            heuristic_count,
            heuristic_avg_calibration: payload.heuristic_calibration_avg,
            anti_knowledge_count: payload.anti_knowledge_count,
        };
    }

    fn apply_budget(&mut self, payload: BudgetAlertPayload) -> bool {
        self.target_spend
            .insert(payload.target.clone(), payload.budget_spent);
        self.target_remaining
            .insert(payload.target.clone(), payload.budget_remaining);
        self.state.cost_meter.burn_rate_usd_per_hour = payload.burn_rate;

        let agent_name = budget_agent_name(&payload.target);
        if let Some(agent_name) = &agent_name {
            self.agent_vitality
                .insert(agent_name.clone(), payload.vitality);
            let agent = self.upsert_agent(agent_name);
            agent.vitality = payload.vitality;
            agent.phase = payload.vitality_phase;
            self.sort_agents();
        }
        self.recompute_cost();
        self.recompute_cohort();
        agent_name.is_some()
    }

    fn apply_trend(&mut self, payload: TrendPayload) -> Option<&'static str> {
        let target = chained_trend_target(&payload);
        let direction = trend_direction(&payload.direction).to_owned();
        match target {
            Some(COST_METER) => {
                self.state.cost_meter.cost_trend = direction;
                Some(COST_METER)
            }
            Some(C_FACTOR) => {
                self.state.c_factor.trend = direction;
                Some(C_FACTOR)
            }
            _ => None,
        }
    }

    fn apply_c_factor(&mut self, payload: CFactorPayload) {
        self.state.c_factor = CFactorProjection {
            c_factor: payload.c_factor,
            components: BTreeMap::from([
                ("citation_reciprocity".into(), payload.citation_reciprocity),
                ("hdc_diversity".into(), payload.hdc_diversity),
                (
                    "peer_prediction_accuracy".into(),
                    payload.peer_prediction_accuracy,
                ),
                ("turn_taking_entropy".into(), payload.turn_taking_entropy),
            ]),
            trend: self.state.c_factor.trend.clone(),
            agent_diversity: payload.hdc_diversity,
        };
    }

    fn recompute_cost(&mut self) {
        self.state.cost_meter.total_usd = self.target_spend.values().sum();
        self.state.cost_meter.budget_remaining = self.target_remaining.values().sum();
        self.state.cost_meter.model_breakdown.clear();
        for breakdown in self.model_breakdown.values() {
            for (model, cost) in breakdown {
                *self
                    .state
                    .cost_meter
                    .model_breakdown
                    .entry(model.clone())
                    .or_default() += cost;
            }
        }
    }

    fn recompute_cohort(&mut self) {
        self.state.cohort_health.agent_count = self.agent_vitality.len();
        self.state.cohort_health.active_count = self.agent_vitality.len();
        self.state.cohort_health.avg_vitality = mean(self.agent_vitality.values().copied());
        self.state.cohort_health.avg_pass_rate = mean(self.target_pass_rate.values().copied());
        self.state.cohort_health.total_spend_usd = self.state.cost_meter.total_usd;
        self.state.cohort_health.error_rate = mean(self.target_error_rate.values().copied());
        self.state.cohort_health.t0_hit_rate = mean(self.agent_t0_hit_rate.values().copied());
    }

    fn upsert_agent(&mut self, name: &str) -> &mut AgentVitalitySnapshot {
        if let Some(index) = self
            .state
            .agent_vitality
            .agents
            .iter()
            .position(|agent| agent.name == name)
        {
            return &mut self.state.agent_vitality.agents[index];
        }
        self.state
            .agent_vitality
            .agents
            .push(AgentVitalitySnapshot {
                name: name.to_owned(),
                ..AgentVitalitySnapshot::default()
            });
        self.state
            .agent_vitality
            .agents
            .last_mut()
            .expect("agent was just inserted")
    }

    fn sort_agents(&mut self) {
        self.state
            .agent_vitality
            .agents
            .sort_by(|left, right| left.name.cmp(&right.name));
    }

    fn update(
        &self,
        projection_id: &'static str,
        source_lens: &str,
    ) -> Result<ProjectionUpdate, TelemetryProjectionError> {
        let data = match projection_id {
            COHORT_HEALTH => serialize(projection_id, &self.state.cohort_health),
            ACTIVE_TASKS => serialize(projection_id, &self.state.active_tasks),
            GATE_PIPELINE => serialize(projection_id, &self.state.gate_pipeline),
            COST_METER => serialize(projection_id, &self.state.cost_meter),
            KNOWLEDGE_HEALTH => serialize(projection_id, &self.state.knowledge_health),
            C_FACTOR => serialize(projection_id, &self.state.c_factor),
            AGENT_VITALITY => serialize(projection_id, &self.state.agent_vitality),
            _ => unreachable!("projection IDs are selected internally"),
        }?;
        Ok(ProjectionUpdate {
            projection_id,
            data,
            source_lens: source_lens.to_owned(),
        })
    }
}

fn serialize<T: Serialize>(
    projection_id: &'static str,
    value: &T,
) -> Result<Value, TelemetryProjectionError> {
    serde_json::to_value(value).map_err(|error| TelemetryProjectionError::ProjectionSerialization {
        projection_id,
        message: error.to_string(),
    })
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0_u64), |(sum, count), value| {
        (sum + value, count.saturating_add(1))
    });
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn budget_agent_name(target: &str) -> Option<String> {
    if target.starts_with("space:") {
        None
    } else {
        Some(target.strip_prefix("agent:").unwrap_or(target).to_owned())
    }
}

fn chained_trend_target(payload: &TrendPayload) -> Option<&'static str> {
    let source = payload
        .source_lens
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if source == "cost" || source == "costlens" {
        return Some(COST_METER);
    }
    if matches!(
        source.as_str(),
        "cfactor" | "cfactorlens" | "collectiveintelligencelens"
    ) {
        return Some(C_FACTOR);
    }
    match payload.metric.as_str() {
        "total_usd" | "budget_remaining" | "burn_rate_usd_per_hour" => Some(COST_METER),
        "c_factor" | "agent_diversity" => Some(C_FACTOR),
        _ => None,
    }
}

const fn trend_direction(direction: &TrendDirection) -> &'static str {
    match direction {
        TrendDirection::Rising => "rising",
        TrendDirection::Falling => "falling",
        TrendDirection::Stable => "stable",
    }
}
