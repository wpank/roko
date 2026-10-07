//! Agent efficiency events and role cost profiles.
//!
//! This module implements the efficiency monitoring pipeline described in
//! `tmp/mori-agents/22-efficiency-monitoring.md`. It bridges per-agent-turn
//! execution data with system-level optimization by providing:
//!
//! - [`AgentEfficiencyEvent`] — rich per-turn cost and quality snapshot
//! - [`PromptSectionMeta`] — per-section token attribution
//! - [`RoleCostProfile`] — aggregate cost profile per agent role
//!
//! # Design
//!
//! Efficiency events are computed *after* each agent turn and gate
//! evaluation. They are never mutated — each turn produces one immutable
//! event. Downstream consumers (bandits, dashboards, regression detector)
//! read from the accumulated event stream.

use std::collections::{HashMap, HashSet};

use roko_core::OperatingFrequency;
use serde::{Deserialize, Serialize};

const BASELINE_PLAN_COUNT: usize = 10;

/// Schema discriminator written as the `"schema"` field on every new
/// [`AgentEfficiencyEvent`] row in `.roko/learn/efficiency.jsonl`.
///
/// The file is shared with the `FeedbackService`, which writes `kind`-tagged
/// feedback rows ([`FEEDBACK_EVENT_SCHEMA`]). Rows written before this field
/// existed are classified by shape instead — see [`classify_efficiency_row`].
pub const AGENT_EFFICIENCY_EVENT_SCHEMA: &str = "agent_efficiency_event/v1";

/// Schema discriminator for the `kind`-tagged rows the `FeedbackService`
/// writes into the same `.roko/learn/efficiency.jsonl` file.
pub const FEEDBACK_EVENT_SCHEMA: &str = "feedback_event/v1";

// ─── PromptSectionMeta ──────────────────────────────────────────────────────

/// Metadata for one section in a composed prompt.
///
/// Used to attribute token budget consumption to individual prompt sections
/// so the section bandit and efficiency scorer can reason about which
/// sections pull their weight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSectionMeta {
    /// Section name (e.g. `"plan_brief"`, `"workspace_map"`, `"playbook_hits"`).
    pub name: String,
    /// Number of tokens this section consumed in the final prompt.
    pub tokens: u64,
    /// Compose-assigned priority (0 = highest, 255 = lowest).
    pub priority: u8,
    /// Whether this section was truncated due to budget pressure.
    pub was_truncated: bool,
    /// Whether this section was dropped entirely due to budget pressure.
    pub was_dropped: bool,
}

// ─── ToolCallMeta ───────────────────────────────────────────────────────────

/// Metadata for one tool call made during an agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallMeta {
    /// Tool name (e.g. `"Read"`, `"Write"`, `"Bash"`).
    pub tool_name: String,
    /// Wall-clock duration of the tool call in milliseconds.
    pub duration_ms: u64,
    /// Number of tokens in the tool result.
    pub result_tokens: u64,
    /// Whether the tool call succeeded; `None` when nothing observed its
    /// outcome, as on the Graph path, where the provider runs the tools
    /// (bug-f9ae3e).
    #[serde(default)]
    pub succeeded: Option<bool>,
    /// Whether this call contributed useful progress toward the final solution.
    #[serde(default)]
    pub advanced_task: bool,
    /// Whether this call was later determined to be unnecessary.
    #[serde(default)]
    pub was_redundant: bool,
    /// Failure category for the call, when one was identified.
    #[serde(default)]
    pub error_category: Option<String>,
}

// ─── AgentEfficiencyEvent ───────────────────────────────────────────────────

/// Emitted once per agent turn completion, summarizing cost and efficiency.
///
/// This is the bridge between agent-level execution and system-level
/// optimization. Contains 20+ fields covering identity, token accounting,
/// cost accounting, prompt composition, tool utilization, and timing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentEfficiencyEvent {
    // ── Identity ────────────────────────────────────────────────────
    /// Agent identifier.
    pub agent_id: String,
    /// Agent role (e.g. `"Implementer"`, `"Reviewer"`).
    pub role: String,
    /// Backend that ran this turn.
    pub backend: String,
    /// Exact model slug.
    pub model: String,
    /// Plan this turn belongs to.
    pub plan_id: String,
    /// Task within the plan.
    pub task_id: String,
    /// Unique identifier for this dispatch attempt.
    ///
    /// On the Graph path it is the attempt's durable key,
    /// `run:plan:task:attempt`, so it never repeats across runs: a resumed run
    /// continues the run's attempt ordinals. The attempt's gate rows extend
    /// the key (`<key>/gate-pass`, `<key>/gate-fail`), so rows of one attempt
    /// join on the key as a prefix.
    #[serde(default)]
    pub attempt_id: String,

    // ── Token accounting ────────────────────────────────────────────
    /// Input tokens from provider response.
    pub input_tokens: u64,
    /// Output tokens from provider response.
    pub output_tokens: u64,
    /// Tokens used for reasoning/thinking.
    #[serde(default)]
    pub reasoning_tokens: u64,
    /// Tokens served from cache (subset of input).
    pub cache_read_tokens: u64,
    /// Tokens written to cache.
    pub cache_write_tokens: u64,

    // ── Cost accounting ─────────────────────────────────────────────
    /// Actual cost after cache discount.
    pub cost_usd: f64,
    /// What it would have cost without caching.
    pub cost_usd_without_cache: f64,
    /// The turn's tokens priced at the API rates of `price_snapshot_id` (S01
    /// §4.4), which a subscription-billed turn's `cost_usd` (about $0) is
    /// not; the homeostasis cost fold reads it first (gap-e73a26). `None`
    /// when nothing priced the turn, and on a row written before this field
    /// (gap-546e8a).
    #[serde(default)]
    pub api_equiv_usd: Option<f64>,
    /// The price snapshot behind `api_equiv_usd`.
    #[serde(default)]
    pub price_snapshot_id: Option<String>,

    // ── Prompt composition ──────────────────────────────────────────
    /// Per-section metadata.
    pub prompt_sections: Vec<PromptSectionMeta>,
    /// Total tokens in the assembled prompt.
    pub total_prompt_tokens: u64,
    /// Tokens in the system prompt (subset of total).
    pub system_prompt_tokens: u64,

    // ── Tool utilization ────────────────────────────────────────────
    /// Number of tools available to the agent.
    pub tools_available: u32,
    /// Number of distinct tools the agent actually used.
    pub tools_used: u32,
    /// Per-tool-call metadata.
    pub tool_calls: Vec<ToolCallMeta>,

    // ── Timing ──────────────────────────────────────────────────────
    /// Wall-clock milliseconds for the entire turn.
    pub wall_time_ms: u64,
    /// Alias for wall-clock task duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Time to first token in milliseconds: from the start of the provider
    /// call to its first streamed model output. 0 when unknown.
    pub time_to_first_token_ms: u64,
    /// Whether this agent was a warm-pool reuse or cold start.
    pub was_warm_start: bool,

    // ── Outcome ─────────────────────────────────────────────────────
    /// Iteration number.
    pub iteration: u32,
    /// Which turn within this task attempt (0 = first turn).
    #[serde(default)]
    pub turn_number: u32,
    /// Whether this is the final turn for the task (i.e., the summary event).
    /// Earlier turns have `is_final_turn = false` and are lightweight cost records.
    #[serde(default = "default_true")]
    pub is_final_turn: bool,
    /// Whether the gate passed after this turn.
    /// `None` while a buffered efficiency event is awaiting its `GateResult`
    /// (runner-v2 async path). Filled in by the event subscriber when the
    /// `GateResult` arrives. `Some(true)` = passed, `Some(false)` = failed.
    #[serde(default)]
    pub gate_passed: Option<bool>,
    /// Outcome label for the observation.
    #[serde(default)]
    pub outcome: String,
    /// Verify error summaries recorded for failed tasks.
    #[serde(default)]
    pub gate_errors: Vec<String>,
    /// Resolved model used for the task attempt.
    #[serde(rename = "resolved_model", alias = "model_used", default)]
    pub model_used: String,
    /// Operating frequency for the turn.
    #[serde(default = "default_operating_frequency")]
    pub frequency: OperatingFrequency,
    /// Replanning or retry strategy attempted after failure.
    #[serde(default)]
    pub strategy_attempted: String,
    /// ISO-8601 UTC timestamp.
    pub timestamp: String,
}

// Manual `Serialize` impl: stamps every serialized row with the explicit
// `"schema"` discriminator (audit #23) without adding a field to the struct,
// which would break the many struct-literal construction sites across the
// workspace. Serde ignores unknown fields on deserialize, so the extra key
// is backward compatible for both old readers (new rows) and old rows (no
// field). Keep this field list in sync with the struct —
// `efficiency_event_serialization_roundtrip` fails if one side drifts.
impl Serialize for AgentEfficiencyEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        // The schema discriminator and 34 struct fields, then the two priced
        // figures when known, as `skip_serializing_if` would leave them out.
        let priced = usize::from(self.api_equiv_usd.is_some())
            + usize::from(self.price_snapshot_id.is_some());
        let mut state = serializer.serialize_struct("AgentEfficiencyEvent", 35 + priced)?;
        state.serialize_field("schema", AGENT_EFFICIENCY_EVENT_SCHEMA)?;
        state.serialize_field("agent_id", &self.agent_id)?;
        state.serialize_field("role", &self.role)?;
        state.serialize_field("backend", &self.backend)?;
        state.serialize_field("model", &self.model)?;
        state.serialize_field("plan_id", &self.plan_id)?;
        state.serialize_field("task_id", &self.task_id)?;
        state.serialize_field("attempt_id", &self.attempt_id)?;
        state.serialize_field("input_tokens", &self.input_tokens)?;
        state.serialize_field("output_tokens", &self.output_tokens)?;
        state.serialize_field("reasoning_tokens", &self.reasoning_tokens)?;
        state.serialize_field("cache_read_tokens", &self.cache_read_tokens)?;
        state.serialize_field("cache_write_tokens", &self.cache_write_tokens)?;
        state.serialize_field("cost_usd", &self.cost_usd)?;
        state.serialize_field("cost_usd_without_cache", &self.cost_usd_without_cache)?;
        if let Some(api_equiv_usd) = &self.api_equiv_usd {
            state.serialize_field("api_equiv_usd", api_equiv_usd)?;
        }
        if let Some(price_snapshot_id) = &self.price_snapshot_id {
            state.serialize_field("price_snapshot_id", price_snapshot_id)?;
        }
        state.serialize_field("prompt_sections", &self.prompt_sections)?;
        state.serialize_field("total_prompt_tokens", &self.total_prompt_tokens)?;
        state.serialize_field("system_prompt_tokens", &self.system_prompt_tokens)?;
        state.serialize_field("tools_available", &self.tools_available)?;
        state.serialize_field("tools_used", &self.tools_used)?;
        state.serialize_field("tool_calls", &self.tool_calls)?;
        state.serialize_field("wall_time_ms", &self.wall_time_ms)?;
        state.serialize_field("duration_ms", &self.duration_ms)?;
        state.serialize_field("time_to_first_token_ms", &self.time_to_first_token_ms)?;
        state.serialize_field("was_warm_start", &self.was_warm_start)?;
        state.serialize_field("iteration", &self.iteration)?;
        state.serialize_field("turn_number", &self.turn_number)?;
        state.serialize_field("is_final_turn", &self.is_final_turn)?;
        state.serialize_field("gate_passed", &self.gate_passed)?;
        state.serialize_field("outcome", &self.outcome)?;
        state.serialize_field("gate_errors", &self.gate_errors)?;
        // Serialized as `resolved_model`, matching the serde rename.
        state.serialize_field("resolved_model", &self.model_used)?;
        state.serialize_field("frequency", &self.frequency)?;
        state.serialize_field("strategy_attempted", &self.strategy_attempted)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.end()
    }
}

impl AgentEfficiencyEvent {
    /// Build a default empty event payload.
    #[must_use]
    pub fn default_event() -> Self {
        Self::default()
    }

    /// Compute the cache hit rate for this event.
    #[allow(clippy::cast_precision_loss)]
    pub fn cache_hit_rate(&self) -> f64 {
        if self.input_tokens == 0 {
            return 0.0;
        }
        self.cache_read_tokens as f64 / self.input_tokens as f64
    }

    /// Compute tool utilization rate (tools used / tools available).
    pub fn tool_utilization(&self) -> f64 {
        if self.tools_available == 0 {
            return 0.0;
        }
        f64::from(self.tools_used) / f64::from(self.tools_available)
    }

    /// Whether `cost_usd` was measured, with the semantics of
    /// `roko_core::Usage::has_known_cost`: it is known when non-zero, or when
    /// no tokens were consumed (a confirmed free turn). A token-consuming
    /// event recorded at `0.0` (a bench run of an external agent, say) has
    /// an unknown cost that must never be averaged in as $0.
    #[must_use]
    pub fn has_known_cost(&self) -> bool {
        self.cost_usd.abs() > f64::EPSILON
            || (self.input_tokens + self.output_tokens + self.cache_write_tokens == 0
                && self.cache_read_tokens == 0)
    }

    /// Compute cost savings from caching.
    pub fn cache_savings_usd(&self) -> f64 {
        self.cost_usd_without_cache - self.cost_usd
    }

    /// Total tokens consumed (input + output).
    pub const fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

fn default_operating_frequency() -> OperatingFrequency {
    OperatingFrequency::Theta
}

fn default_true() -> bool {
    true
}

// ─── efficiency.jsonl schema discrimination ─────────────────────────────────

/// Which of the schemas sharing `.roko/learn/efficiency.jsonl` a row uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfficiencyRowSchema {
    /// [`AgentEfficiencyEvent`] rows (per-turn cost/efficiency records).
    AgentEfficiencyEvent,
    /// `FeedbackService` `kind`-tagged rows (`model_call`, `gate_result`, ...).
    FeedbackEvent,
    /// Matches neither schema (corrupt row or unrecognized future schema).
    Unknown,
}

/// Classify one parsed efficiency.jsonl row.
///
/// Rows written after the discriminator was introduced carry an explicit
/// `"schema"` field. Legacy rows predate it and are classified by shape:
/// `FeedbackService` rows always have a `"kind"` tag, [`AgentEfficiencyEvent`]
/// rows never do and always carry `"agent_id"`.
#[must_use]
pub fn classify_efficiency_row(value: &serde_json::Value) -> EfficiencyRowSchema {
    if let Some(schema) = value.get("schema").and_then(serde_json::Value::as_str) {
        return match schema {
            AGENT_EFFICIENCY_EVENT_SCHEMA => EfficiencyRowSchema::AgentEfficiencyEvent,
            FEEDBACK_EVENT_SCHEMA => EfficiencyRowSchema::FeedbackEvent,
            _ => EfficiencyRowSchema::Unknown,
        };
    }
    if value.get("kind").is_some() {
        EfficiencyRowSchema::FeedbackEvent
    } else if value.get("agent_id").is_some() {
        EfficiencyRowSchema::AgentEfficiencyEvent
    } else {
        EfficiencyRowSchema::Unknown
    }
}

/// Outcome of tolerantly parsing efficiency.jsonl content.
#[derive(Debug, Default)]
pub struct EfficiencyEventsParse {
    /// Rows that parsed as [`AgentEfficiencyEvent`].
    pub events: Vec<AgentEfficiencyEvent>,
    /// Rows matching the foreign `FeedbackService` schema (skipped, counted).
    pub skipped_foreign_rows: usize,
    /// Rows matching neither schema or failing to parse (skipped, counted).
    pub skipped_unknown_rows: usize,
}

/// Parse efficiency.jsonl content, tolerating the `FeedbackService` rows that
/// share the file. Foreign and unparseable rows are skipped explicitly and
/// reported at debug level instead of vanishing silently (audit #23).
#[must_use]
pub fn parse_efficiency_events_jsonl(contents: &str) -> EfficiencyEventsParse {
    let mut parsed = EfficiencyEventsParse::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            parsed.skipped_unknown_rows += 1;
            continue;
        };
        match classify_efficiency_row(&value) {
            EfficiencyRowSchema::AgentEfficiencyEvent => {
                match serde_json::from_value::<AgentEfficiencyEvent>(value) {
                    Ok(event) => parsed.events.push(event),
                    Err(_) => parsed.skipped_unknown_rows += 1,
                }
            }
            EfficiencyRowSchema::FeedbackEvent => parsed.skipped_foreign_rows += 1,
            EfficiencyRowSchema::Unknown => parsed.skipped_unknown_rows += 1,
        }
    }
    if parsed.skipped_foreign_rows > 0 || parsed.skipped_unknown_rows > 0 {
        tracing::debug!(
            skipped_foreign_rows = parsed.skipped_foreign_rows,
            skipped_unknown_rows = parsed.skipped_unknown_rows,
            parsed_events = parsed.events.len(),
            "efficiency.jsonl: skipped rows outside the AgentEfficiencyEvent schema"
        );
    }
    parsed
}

impl Default for AgentEfficiencyEvent {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            role: String::new(),
            backend: String::new(),
            model: String::new(),
            plan_id: String::new(),
            task_id: String::new(),
            attempt_id: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            cost_usd_without_cache: 0.0,
            api_equiv_usd: None,
            price_snapshot_id: None,
            prompt_sections: Vec::new(),
            total_prompt_tokens: 0,
            system_prompt_tokens: 0,
            tools_available: 0,
            tools_used: 0,
            tool_calls: Vec::new(),
            wall_time_ms: 0,
            duration_ms: 0,
            time_to_first_token_ms: 0,
            was_warm_start: false,
            iteration: 0,
            turn_number: 0,
            is_final_turn: true,
            gate_passed: None,
            outcome: String::new(),
            gate_errors: Vec::new(),
            model_used: String::new(),
            frequency: default_operating_frequency(),
            strategy_attempted: String::new(),
            timestamp: String::new(),
        }
    }
}

// ─── ExecutedRow ────────────────────────────────────────────────────────────

/// A Graph attempt's efficiency or cost row, with the columns of its
/// `roko.verdict/1` `executed` block that the row type lacks, beside the
/// row's own fields. The row's `model` names the model the provider bridge
/// launched; these name the model the provider reported serving
/// (bug-31438d) and the planned model a failover replaced (bug-35379d), and
/// mark a turn count the agent never reported (bug-55fd84). Like
/// [`crate::telemetry::AttemptKeyed`], a reader that parses the row type
/// alone still reads the line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutedRow<T> {
    /// The row.
    #[serde(flatten)]
    pub row: T,
    /// The model the provider reported serving; `None` when it named none.
    #[serde(default)]
    pub model_reported: Option<String>,
    /// The provider reported serving another model than the one launched.
    #[serde(default)]
    pub model_mismatch: bool,
    /// Every model the provider named, when its responses disagreed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models_reported: Vec<String>,
    /// The planned model a failover replaced; absent when the planned model
    /// ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub substituted_from: Option<String>,
    /// Why the planned model did not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub substitution_reason: Option<String>,
    /// The agent reported no turn count, so the row's turn fields are 0 for
    /// unknown.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turns_unknown: bool,
}

impl<T> ExecutedRow<T> {
    /// `row` with the columns of `executed`, its attempt's verdict block.
    #[must_use]
    pub fn new(row: T, executed: &crate::telemetry::ExecutedModel) -> Self {
        Self {
            row,
            model_reported: executed.model_reported.clone(),
            model_mismatch: executed.model_mismatch,
            models_reported: executed.models_reported.clone(),
            substituted_from: executed.failover_chain.first().cloned(),
            substitution_reason: executed.failover_reason.clone(),
            turns_unknown: executed.turns.is_none(),
        }
    }
}

/// An efficiency row whose turn fields may be unknown. `turns_unknown`
/// marks a row whose `iteration` and `turn_number` are 0 because the agent
/// reported no count (bug-ad5487); [`ExecutedRow`] carries the same marker
/// on an attempt's dispatch row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnsRow<T> {
    /// The row.
    #[serde(flatten)]
    pub row: T,
    /// The agent reported no turn count.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turns_unknown: bool,
}

/// An efficiency row whose time to first token may be unknown.
/// `ttft_unknown` marks a row whose `time_to_first_token_ms` is 0 because no
/// stream showed model output, not because the first token came at once;
/// [`TurnsRow`] marks unknown turns the same way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtftRow<T> {
    /// The row.
    #[serde(flatten)]
    pub row: T,
    /// No stream showed model output, so the time to first token is unknown.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ttft_unknown: bool,
}

// ─── RoleCostProfile ────────────────────────────────────────────────────────

/// Aggregate cost profile for a single agent role, computed from accumulated
/// efficiency events.
///
/// Answers questions like "What does the average Implementer turn cost?" and
/// "What is the cost per gate pass?"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleCostProfile {
    /// Agent role this profile covers.
    pub role: String,
    /// Number of efficiency events contributing.
    pub observations: u64,

    // ── Token averages ──────────────────────────────────────────────
    /// Average input tokens per turn.
    pub avg_input_tokens: f64,
    /// Average output tokens per turn.
    pub avg_output_tokens: f64,
    /// Average cache hit rate (`cache_read` / `input`).
    pub avg_cache_hit_rate: f64,

    // ── Cost averages ───────────────────────────────────────────────
    //
    // The cost figures cover only turns whose cost was measured
    // (`AgentEfficiencyEvent::has_known_cost`). A token-consuming turn
    // recorded at $0 has an unknown cost, and averaging it in as free would
    // understate what the role costs.
    /// Average cost in USD per measured turn; `None` when no turn's cost was
    /// measured.
    pub avg_cost_usd: Option<f64>,
    /// 95th percentile cost in USD over the measured turns; `None` when no
    /// turn's cost was measured.
    pub p95_cost_usd: Option<f64>,
    /// Measured cost / gate passes among the measured turns — the true cost
    /// of one success; `None` when no measured turn passed.
    pub cost_per_pass: Option<f64>,
    /// Turns whose cost nobody measured, left out of the figures above.
    #[serde(default)]
    pub cost_unknown: u64,

    // ── Efficiency ──────────────────────────────────────────────────
    /// Average tool utilization (`tools_used` / `tools_available`).
    pub avg_tool_utilization: f64,
    /// Average wall time in milliseconds.
    pub avg_wall_time_ms: f64,
    /// Fraction of turns that were warm starts.
    pub warm_start_pct: f64,
    /// Overall gate pass rate for this role.
    pub pass_rate: f64,
}

impl RoleCostProfile {
    /// Cost of one successful task for this role: the average measured cost
    /// over the pass rate. `None` when no turn's cost was measured; infinite
    /// when the role never passed.
    #[must_use]
    pub fn cost_per_successful_task(&self) -> Option<f64> {
        let avg_cost = self.avg_cost_usd?;
        if self.pass_rate <= 0.0 {
            return Some(f64::INFINITY);
        }
        Some(avg_cost / self.pass_rate)
    }
}

/// Aggregate cost profile for a single operating frequency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrequencyCostProfile {
    /// Operating frequency this profile covers.
    pub frequency: OperatingFrequency,
    /// Number of efficiency events contributing.
    pub observations: u64,
    /// Average cost in USD per measured turn; `None` when no turn's cost was
    /// measured. Cost figures leave out turns with an unknown cost, as in
    /// [`RoleCostProfile`].
    pub avg_cost_usd: Option<f64>,
    /// Total measured cost in USD; `None` when no turn's cost was measured.
    pub total_cost_usd: Option<f64>,
    /// Overall gate pass rate for this frequency.
    pub pass_rate: f64,
    /// Measured cost / gate passes among the measured turns — the true cost
    /// of one success; `None` when no measured turn passed.
    pub cost_per_pass: Option<f64>,
    /// Turns whose cost nobody measured, left out of the cost figures.
    #[serde(default)]
    pub cost_unknown: u64,
}

/// Composite C-Factor snapshot for a single `roko plan run` session.
///
/// This aggregates efficiency telemetry across all agents participating in the
/// run, grouped by plan. It is the fleet-level counterpart to the episode-based
/// C-Factor in [`crate::cfactor`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCFactor {
    /// 0.0-1.0 composite score for the session.
    pub overall: f64,
    /// Component breakdown for the score.
    pub components: FleetCFactorComponents,
    /// Number of distinct plans represented in the session.
    pub plan_count: usize,
    /// Number of distinct agents represented in the session.
    pub agent_count: usize,
    /// Number of efficiency events contributing to the snapshot.
    pub observation_count: usize,
}

/// Individual fleet C-Factor components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetCFactorComponents {
    /// Fraction of plans that used more than one agent.
    pub multi_agent_coverage: f64,
    /// Fraction of plan groups that passed at least one gate.
    pub pass_rate: f64,
    /// Inverse of cost per successful plan, normalized against the run's baseline.
    pub cost_efficiency: f64,
    /// Inverse of duration per successful plan, normalized against the run's baseline.
    pub speed: f64,
    /// Evenness of agent participation inside plan groups, normalized to `[0..1]`.
    pub turn_taking_equality: f64,
}

impl Default for FleetCFactorComponents {
    fn default() -> Self {
        Self {
            multi_agent_coverage: 0.0,
            pass_rate: 0.0,
            cost_efficiency: 0.0,
            speed: 0.0,
            turn_taking_equality: 0.0,
        }
    }
}

impl Default for FleetCFactor {
    fn default() -> Self {
        Self {
            overall: 0.0,
            components: FleetCFactorComponents::default(),
            plan_count: 0,
            agent_count: 0,
            observation_count: 0,
        }
    }
}

/// Compute a [`RoleCostProfile`] for each distinct role in the given events.
///
/// Cost figures come only from events whose cost was measured; the rest are
/// counted in [`RoleCostProfile::cost_unknown`], never averaged in as $0.
#[allow(clippy::cast_precision_loss)]
pub fn compute_role_profiles(events: &[AgentEfficiencyEvent]) -> Vec<RoleCostProfile> {
    let mut groups: HashMap<String, Vec<&AgentEfficiencyEvent>> = HashMap::new();
    for e in events {
        groups.entry(e.role.clone()).or_default().push(e);
    }

    let mut profiles: Vec<RoleCostProfile> = groups
        .into_iter()
        .map(|(role, evts)| {
            let n = evts.len() as f64;
            let n_u64 = evts.len() as u64;

            let avg_input = evts.iter().map(|e| e.input_tokens as f64).sum::<f64>() / n;
            let avg_output = evts.iter().map(|e| e.output_tokens as f64).sum::<f64>() / n;
            let avg_cache = evts.iter().map(|e| e.cache_hit_rate()).sum::<f64>() / n;
            let avg_wall = evts.iter().map(|e| e.wall_time_ms as f64).sum::<f64>() / n;
            let avg_tool = evts.iter().map(|e| e.tool_utilization()).sum::<f64>() / n;

            let warm_count = evts.iter().filter(|e| e.was_warm_start).count();
            let warm_pct = warm_count as f64 / n;

            let pass_count = evts.iter().filter(|e| e.gate_passed == Some(true)).count();
            let pass_rate = pass_count as f64 / n;

            // Cost figures come only from turns whose cost was measured.
            let costed: Vec<&AgentEfficiencyEvent> = evts
                .iter()
                .copied()
                .filter(|e| e.has_known_cost())
                .collect();
            let cost_unknown = n_u64.saturating_sub(costed.len() as u64);
            let total_cost: f64 = costed.iter().map(|e| e.cost_usd).sum();
            let avg_cost = if costed.is_empty() {
                None
            } else {
                Some(total_cost / costed.len() as f64)
            };
            let costed_passes = costed
                .iter()
                .filter(|e| e.gate_passed == Some(true))
                .count();
            let cost_per_pass = if costed_passes > 0 {
                Some(total_cost / costed_passes as f64)
            } else {
                None
            };

            // P95 cost: sort the measured costs and take the 95th percentile.
            let mut costs: Vec<f64> = costed.iter().map(|e| e.cost_usd).collect();
            costs.sort_by(f64::total_cmp);
            // P95 index: 95% of the way through the sorted cost list.
            let p95_idx = (costs.len() * 95 / 100).min(costs.len().saturating_sub(1));
            let p95_cost = costs.get(p95_idx).copied();

            RoleCostProfile {
                role,
                observations: n_u64,
                avg_input_tokens: avg_input,
                avg_output_tokens: avg_output,
                avg_cache_hit_rate: avg_cache,
                avg_cost_usd: avg_cost,
                p95_cost_usd: p95_cost,
                cost_per_pass,
                cost_unknown,
                avg_tool_utilization: avg_tool,
                avg_wall_time_ms: avg_wall,
                warm_start_pct: warm_pct,
                pass_rate,
            }
        })
        .collect();

    profiles.sort_by(|a, b| a.role.cmp(&b.role));
    profiles
}

/// Compute a [`FrequencyCostProfile`] for each distinct operating frequency.
///
/// Like [`compute_role_profiles`], cost figures come only from events whose
/// cost was measured; the rest are counted in
/// [`FrequencyCostProfile::cost_unknown`], never averaged in as $0.
#[allow(clippy::cast_precision_loss)]
pub fn compute_frequency_profiles(events: &[AgentEfficiencyEvent]) -> Vec<FrequencyCostProfile> {
    let mut groups: HashMap<OperatingFrequency, Vec<&AgentEfficiencyEvent>> = HashMap::new();
    for e in events {
        groups.entry(e.frequency).or_default().push(e);
    }

    let mut profiles: Vec<FrequencyCostProfile> = groups
        .into_iter()
        .map(|(frequency, evts)| {
            let n = evts.len() as f64;
            let n_u64 = evts.len() as u64;
            let pass_count = evts.iter().filter(|e| e.gate_passed == Some(true)).count();
            let pass_rate = if n == 0.0 { 0.0 } else { pass_count as f64 / n };

            // Cost figures come only from turns whose cost was measured.
            let costed: Vec<&AgentEfficiencyEvent> = evts
                .iter()
                .copied()
                .filter(|e| e.has_known_cost())
                .collect();
            let total_cost = if costed.is_empty() {
                None
            } else {
                Some(costed.iter().map(|e| e.cost_usd).sum::<f64>())
            };
            let avg_cost_usd = total_cost.map(|total| total / costed.len() as f64);
            let costed_passes = costed
                .iter()
                .filter(|e| e.gate_passed == Some(true))
                .count();
            let cost_per_pass = if costed_passes > 0 {
                total_cost.map(|total| total / costed_passes as f64)
            } else {
                None
            };

            FrequencyCostProfile {
                frequency,
                observations: n_u64,
                avg_cost_usd,
                total_cost_usd: total_cost,
                pass_rate,
                cost_per_pass,
                cost_unknown: n_u64.saturating_sub(costed.len() as u64),
            }
        })
        .collect();

    profiles.sort_by_key(|profile| profile.frequency);
    profiles
}

/// Compute a fleet-level C-Factor for a single plan-run session.
///
/// The snapshot groups efficiency events by `plan_id`, then scores the session
/// using pass rate, baseline-relative cost and speed, multi-agent coverage, and
/// turn-taking equality across agents.
#[allow(clippy::cast_precision_loss)]
pub fn compute_fleet_cfactor(events: &[AgentEfficiencyEvent]) -> FleetCFactor {
    if events.is_empty() {
        return FleetCFactor::default();
    }

    let mut groups: HashMap<String, FleetPlanAggregate> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut agent_ids: HashSet<String> = HashSet::new();

    for event in events {
        let plan_id = event.plan_id.trim();
        if plan_id.is_empty() {
            continue;
        }

        agent_ids.insert(event.agent_id.clone());
        let entry = groups.entry(plan_id.to_string()).or_insert_with(|| {
            order.push(plan_id.to_string());
            FleetPlanAggregate::default()
        });
        entry.observe(event);
    }

    let plans: Vec<(String, FleetPlanAggregate)> = order
        .into_iter()
        .filter_map(|plan_id| groups.remove_entry(&plan_id))
        .collect();
    if plans.is_empty() {
        return FleetCFactor::default();
    }

    let plan_count = plans.len();
    let multi_agent_plan_count = plans
        .iter()
        .filter(|(_, plan)| plan.distinct_agents.len() > 1)
        .count();
    let passed_plan_count = plans.iter().filter(|(_, plan)| plan.passed_gate).count();
    let successful_plans: Vec<&FleetPlanAggregate> = plans
        .iter()
        .filter_map(|(_, plan)| plan.passed_gate.then_some(plan))
        .collect();

    let pass_rate = passed_plan_count as f64 / plan_count as f64;
    let multi_agent_coverage = multi_agent_plan_count as f64 / plan_count as f64;
    let turn_taking_equality = {
        let mut total = 0.0;
        let mut counted = 0.0;
        for (_, plan) in &plans {
            if plan.distinct_agents.len() < 2 {
                continue;
            }
            let equality =
                turn_taking_equality_for_counts(plan.agent_turn_counts.values().copied().collect());
            total += equality;
            counted += 1.0;
        }
        if counted == 0.0 {
            0.0
        } else {
            (total / counted).clamp(0.0, 1.0)
        }
    };

    let (avg_cost_per_successful_plan, avg_duration_per_successful_plan) =
        if successful_plans.is_empty() {
            (0.0, 0.0)
        } else {
            let count = successful_plans.len() as f64;
            let total_cost: f64 = successful_plans.iter().map(|plan| plan.cost_usd).sum();
            let total_duration: f64 = successful_plans.iter().map(|plan| plan.duration_ms).sum();
            (total_cost / count, total_duration / count)
        };

    let baseline_plan_count = plans.len().min(BASELINE_PLAN_COUNT);
    let (baseline_cost, baseline_duration) = if baseline_plan_count == 0 {
        (0.0, 0.0)
    } else {
        let baseline_plans: Vec<&(String, FleetPlanAggregate)> =
            plans.iter().take(baseline_plan_count).collect();
        let total_cost: f64 = baseline_plans.iter().map(|(_, plan)| plan.cost_usd).sum();
        let total_duration: f64 = baseline_plans
            .iter()
            .map(|(_, plan)| plan.duration_ms)
            .sum();
        (
            total_cost / baseline_plan_count as f64,
            total_duration / baseline_plan_count as f64,
        )
    };

    let cost_efficiency = if baseline_cost > 0.0 && avg_cost_per_successful_plan > 0.0 {
        baseline_cost / avg_cost_per_successful_plan
    } else {
        0.0
    };

    let speed = if baseline_duration > 0.0 && avg_duration_per_successful_plan > 0.0 {
        baseline_duration / avg_duration_per_successful_plan
    } else {
        0.0
    };

    let overall = (pass_rate * 0.30
        + cost_efficiency * 0.20
        + speed * 0.15
        + multi_agent_coverage * 0.15
        + turn_taking_equality * 0.20)
        .clamp(0.0, 1.0);

    FleetCFactor {
        overall,
        components: FleetCFactorComponents {
            multi_agent_coverage,
            pass_rate,
            cost_efficiency,
            speed,
            turn_taking_equality,
        },
        plan_count,
        agent_count: agent_ids.len(),
        observation_count: events.len(),
    }
}

/// Audit #80 / Step 6 — Data-quality migration for historical efficiency events.
///
/// Before item 78 (P0-GA-1) fixed the gate_passed emission ordering, every
/// efficiency event was written with `gate_passed: Some(false)` regardless of
/// the actual gate outcome.  Those stale `false` values degrade model routing
/// because the cascade router treats them as genuine failures.
///
/// This function reads `efficiency.jsonl`, rewrites any line whose
/// `gate_passed` is `Some(false)` AND whose `outcome` is `"success"` (or
/// empty) to `gate_passed: null`, and atomically replaces the file.
/// Lines that cannot be parsed are preserved verbatim.
///
/// # Returns
///
/// The number of lines that were patched.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the replacement write fails.
pub fn null_ambiguous_gate_failed_entries(
    path: &std::path::Path,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write as _;

    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(err) => return Err(Box::new(err)),
    };

    let mut patched = 0usize;
    let mut output = Vec::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            writeln!(output, "{line}")?;
            continue;
        }

        // Parse as a generic JSON object so we can inspect and modify fields
        // without round-tripping through the strongly-typed struct (which would
        // drop any unknown fields added in newer schema versions).
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            writeln!(output, "{line}")?;
            continue;
        };

        // Only patch entries where gate_passed was written as false but the
        // outcome field signals success (or is absent/empty).  This targets
        // exactly the pre-P0-GA-1 pattern where the field was always false.
        let is_false_gate_passed = value
            .get("gate_passed")
            .and_then(|v| v.as_bool())
            .is_some_and(|b| !b);

        if is_false_gate_passed {
            let outcome = value.get("outcome").and_then(|v| v.as_str()).unwrap_or("");
            // If the outcome explicitly records failure or retry, keep the
            // gate_passed=false signal as it may be genuine.
            let is_genuine_failure = matches!(
                outcome,
                "failure" | "gate_failure" | "cancelled" | "timeout"
            );
            if !is_genuine_failure {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("gate_passed".to_string(), serde_json::Value::Null);
                }
                patched += 1;
            }
        }

        writeln!(output, "{}", serde_json::to_string(&value)?)?;
    }

    // Write atomically using a .tmp sibling.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &output)?;
    std::fs::rename(&tmp, path)?;

    Ok(patched)
}

#[derive(Debug, Default)]
struct FleetPlanAggregate {
    cost_usd: f64,
    duration_ms: f64,
    passed_gate: bool,
    distinct_agents: HashSet<String>,
    agent_turn_counts: HashMap<String, u64>,
}

impl FleetPlanAggregate {
    fn observe(&mut self, event: &AgentEfficiencyEvent) {
        self.cost_usd += event.cost_usd;
        self.duration_ms += event.wall_time_ms as f64;
        self.passed_gate |= event.gate_passed == Some(true);
        self.distinct_agents.insert(event.agent_id.clone());
        *self
            .agent_turn_counts
            .entry(event.agent_id.clone())
            .or_default() += 1;
    }
}

fn turn_taking_equality_for_counts(counts: Vec<u64>) -> f64 {
    if counts.len() < 2 {
        return 0.0;
    }

    let gini = gini_coefficient(&counts);
    (1.0 - gini).clamp(0.0, 1.0)
}

fn gini_coefficient(counts: &[u64]) -> f64 {
    if counts.len() < 2 {
        return 0.0;
    }

    let mut values: Vec<f64> = counts.iter().map(|&count| count as f64).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }

    let weighted_sum: f64 = values
        .iter()
        .enumerate()
        .map(|(index, value)| (index as f64 + 1.0) * value)
        .sum();
    let n = values.len() as f64;
    let gini = (2.0 * weighted_sum) / (n * total) - (n + 1.0) / n;
    gini.clamp(0.0, 1.0)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Create a minimal test fixture [`AgentEfficiencyEvent`].
#[cfg(test)]
fn make_test_event(
    role: &str,
    cost: f64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read: u64,
    wall_time_ms: u64,
    tools_available: u32,
    tools_used: u32,
    warm: bool,
    passed: bool,
) -> AgentEfficiencyEvent {
    AgentEfficiencyEvent {
        agent_id: "agent-1".into(),
        role: role.into(),
        backend: "claude".into(),
        model: "claude-sonnet-4-5".into(),
        plan_id: "plan-1".into(),
        task_id: "t1".into(),
        attempt_id: "test-attempt".into(),
        input_tokens,
        output_tokens,
        reasoning_tokens: 0,
        cache_read_tokens: cache_read,
        cache_write_tokens: 0,
        cost_usd: cost,
        cost_usd_without_cache: cost * 1.5,
        api_equiv_usd: None,
        price_snapshot_id: None,
        prompt_sections: Vec::new(),
        total_prompt_tokens: input_tokens,
        system_prompt_tokens: 200,
        tools_available,
        tools_used,
        tool_calls: Vec::new(),
        wall_time_ms,
        duration_ms: wall_time_ms,
        time_to_first_token_ms: 500,
        was_warm_start: warm,
        iteration: 1,
        turn_number: 0,
        is_final_turn: true,
        gate_passed: Some(passed),
        outcome: if passed {
            "success".into()
        } else {
            "failure".into()
        },
        gate_errors: if passed {
            Vec::new()
        } else {
            vec!["test gate failed".into()]
        },
        model_used: "claude-sonnet-4-5".into(),
        frequency: OperatingFrequency::Theta,
        strategy_attempted: if passed {
            "none".into()
        } else {
            "retry_same".into()
        },
        timestamp: "2026-04-06T12:00:00Z".into(),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use tempfile;

    // ── AgentEfficiencyEvent tests ──────────────────────────────────

    #[test]
    fn efficiency_event_cache_hit_rate() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 300, 5000, 10, 5, false, true);
        assert!((e.cache_hit_rate() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_cache_hit_rate_zero_input() {
        let e = make_test_event("Impl", 0.0, 0, 0, 0, 0, 0, 0, false, false);
        assert!((e.cache_hit_rate()).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_tool_utilization() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 0, 5000, 10, 4, false, true);
        assert!((e.tool_utilization() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_tool_utilization_zero_available() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 0, 5000, 0, 0, false, true);
        assert!((e.tool_utilization()).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_cache_savings() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 300, 5000, 10, 5, false, true);
        // cost_usd_without_cache = 0.50 * 1.5 = 0.75
        assert!((e.cache_savings_usd() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_total_tokens() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 0, 5000, 10, 5, false, true);
        assert_eq!(e.total_tokens(), 1200);
    }

    #[test]
    fn efficiency_event_serialization_roundtrip() {
        let e = make_test_event("Implementer", 0.42, 1500, 300, 200, 45000, 8, 3, true, true);
        let json = serde_json::to_string(&e).expect("serialize");
        let e2: AgentEfficiencyEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, e2);
    }

    #[test]
    fn executed_row_sits_beside_the_row_it_extends() {
        let event = AgentEfficiencyEvent {
            model: "gpt-oss-120b".to_string(),
            ..Default::default()
        };
        let executed = crate::telemetry::ExecutedModel {
            model_reported: Some("glm-4.7".to_string()),
            model_mismatch: true,
            failover_chain: vec!["claude-sonnet".to_string()],
            failover_reason: Some("`claude-sonnet` on `claude_cli`: out of usage".to_string()),
            turns: Some(3),
            ..Default::default()
        };
        let row = ExecutedRow::new(&event, &executed);

        let json = serde_json::to_value(&row).expect("serialize");
        assert_eq!(json["schema"], AGENT_EFFICIENCY_EVENT_SCHEMA);
        assert_eq!(json["model"], "gpt-oss-120b");
        assert_eq!(json["model_reported"], "glm-4.7");
        assert_eq!(json["model_mismatch"], true);
        assert_eq!(json["substituted_from"], "claude-sonnet");
        assert!(json.get("models_reported").is_none());
        assert!(json.get("turns_unknown").is_none());
        let alone: AgentEfficiencyEvent =
            serde_json::from_value(json.clone()).expect("parse the row alone");
        assert_eq!(alone, event);
        let back: ExecutedRow<AgentEfficiencyEvent> =
            serde_json::from_value(json).expect("parse the served row");
        assert_eq!(back.model_reported.as_deref(), Some("glm-4.7"));
        assert_eq!(back.row, event);

        let unreported = ExecutedRow::new(&event, &crate::telemetry::ExecutedModel::default());
        let json = serde_json::to_value(&unreported).expect("serialize");
        assert!(json["model_reported"].is_null(), "unknown is null: {json}");
        assert!(json.get("substituted_from").is_none());
        assert_eq!(json["turns_unknown"], true);
    }

    #[test]
    fn efficiency_event_serializes_resolved_model() {
        let event = AgentEfficiencyEvent {
            model_used: "claude-sonnet-4-6".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(json["resolved_model"], "claude-sonnet-4-6");
        assert!(json.get("model_used").is_none());

        let roundtrip: AgentEfficiencyEvent =
            serde_json::from_value(json).expect("deserialize event");
        assert_eq!(roundtrip.model_used, "claude-sonnet-4-6");
    }

    #[test]
    fn efficiency_reasoning_tokens() {
        let mut e = make_test_event("Implementer", 0.42, 1500, 300, 200, 45000, 8, 3, true, true);
        e.reasoning_tokens = 120;

        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"reasoning_tokens\":120"));

        let e2: AgentEfficiencyEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e2.reasoning_tokens, 120);
        assert_eq!(e, e2);
    }

    // ── Schema discrimination tests (audit #23) ─────────────────────

    #[test]
    fn efficiency_event_serialization_includes_schema_discriminator() {
        let event = AgentEfficiencyEvent::default_event();
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["schema"], AGENT_EFFICIENCY_EVENT_SCHEMA);
        // The `resolved_model` rename is preserved by the manual Serialize impl.
        assert!(json.get("model_used").is_none());
        assert!(json.get("resolved_model").is_some());
    }

    #[test]
    fn legacy_event_row_without_schema_still_loads() {
        let event = make_test_event("Implementer", 0.42, 1500, 300, 200, 45000, 8, 3, true, true);
        let mut json = serde_json::to_value(&event).expect("serialize");
        json.as_object_mut().expect("object").remove("schema");
        let parsed: AgentEfficiencyEvent = serde_json::from_value(json).expect("legacy row loads");
        assert_eq!(parsed, event);
    }

    #[test]
    fn classify_efficiency_row_by_tag_and_shape() {
        // Explicit discriminator wins.
        assert_eq!(
            classify_efficiency_row(&serde_json::json!({"schema": AGENT_EFFICIENCY_EVENT_SCHEMA})),
            EfficiencyRowSchema::AgentEfficiencyEvent
        );
        assert_eq!(
            classify_efficiency_row(
                &serde_json::json!({"schema": FEEDBACK_EVENT_SCHEMA, "kind": "model_call"})
            ),
            EfficiencyRowSchema::FeedbackEvent
        );
        assert_eq!(
            classify_efficiency_row(&serde_json::json!({"schema": "future/v9"})),
            EfficiencyRowSchema::Unknown
        );
        // Legacy rows (no discriminator) classified by shape.
        assert_eq!(
            classify_efficiency_row(&serde_json::json!({"kind": "gate_result", "run_id": "r1"})),
            EfficiencyRowSchema::FeedbackEvent
        );
        assert_eq!(
            classify_efficiency_row(&serde_json::json!({"agent_id": "a1", "input_tokens": 10})),
            EfficiencyRowSchema::AgentEfficiencyEvent
        );
        assert_eq!(
            classify_efficiency_row(&serde_json::json!({"unrelated": true})),
            EfficiencyRowSchema::Unknown
        );
    }

    #[test]
    fn parse_efficiency_events_jsonl_counts_skipped_rows() {
        let event = make_test_event("Implementer", 0.42, 1500, 300, 200, 45000, 8, 3, true, true);
        let tagged_row = serde_json::to_string(&event).expect("serialize");
        let mut legacy_value = serde_json::to_value(&event).expect("serialize");
        legacy_value
            .as_object_mut()
            .expect("object")
            .remove("schema");
        let legacy_row = serde_json::to_string(&legacy_value).expect("serialize");
        let feedback_row = serde_json::json!({"kind": "model_call", "model": "sonnet"}).to_string();

        let contents = format!("{tagged_row}\n{legacy_row}\n{feedback_row}\nnot json\n");
        let parsed = parse_efficiency_events_jsonl(&contents);

        assert_eq!(parsed.events.len(), 2, "tagged + legacy rows both load");
        assert_eq!(parsed.skipped_foreign_rows, 1, "kind-tagged row counted");
        assert_eq!(parsed.skipped_unknown_rows, 1, "invalid line counted");
    }

    // ── RoleCostProfile tests ───────────────────────────────────────

    #[test]
    fn efficiency_role_profile_single_role() {
        let events = vec![
            make_test_event(
                "Implementer",
                0.50,
                1000,
                200,
                300,
                10000,
                10,
                5,
                true,
                true,
            ),
            make_test_event("Implementer", 0.30, 800, 150, 200, 8000, 10, 3, false, true),
            make_test_event(
                "Implementer",
                0.70,
                1200,
                250,
                400,
                12000,
                10,
                7,
                true,
                false,
            ),
        ];

        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles.len(), 1);

        let p = &profiles[0];
        assert_eq!(p.role, "Implementer");
        assert_eq!(p.observations, 3);
        assert!((p.avg_cost_usd.expect("measured cost") - 0.5).abs() < 1e-9);
        assert!((p.pass_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!((p.warm_start_pct - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn efficiency_role_profile_multiple_roles() {
        let events = vec![
            make_test_event("Implementer", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Reviewer", 0.20, 500, 100, 0, 5000, 5, 2, false, true),
        ];

        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles.len(), 2);
        // Sorted by role name
        assert_eq!(profiles[0].role, "Implementer");
        assert_eq!(profiles[1].role, "Reviewer");
    }

    #[test]
    fn efficiency_role_profile_cost_per_pass() {
        // 3 events: 2 pass, total cost = 1.50 → cost_per_pass = 0.75
        let events = vec![
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, false),
        ];

        let profiles = compute_role_profiles(&events);
        assert!((profiles[0].cost_per_pass.expect("measured passes") - 0.75).abs() < 1e-9);
    }

    #[test]
    fn efficiency_role_profile_no_passes() {
        let events = vec![make_test_event(
            "Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, false,
        )];

        // No pass to divide by: the cost per pass is undefined, not $0.
        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles[0].cost_per_pass, None);
    }

    #[test]
    fn efficiency_role_profile_cost_per_successful_task() {
        let events = vec![
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, false),
        ];

        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles.len(), 1);

        let p = &profiles[0];
        assert!((p.avg_cost_usd.expect("measured cost") - 0.50).abs() < 1e-9);
        assert!((p.pass_rate - 0.80).abs() < 1e-9);
        let per_success = p.cost_per_successful_task().expect("measured cost");
        assert!((per_success - 0.625).abs() < 1e-9);
    }

    #[test]
    fn efficiency_frequency_profile_groups_by_frequency() {
        let mut gamma = make_test_event("Impl", 1.0, 100, 20, 0, 1000, 5, 2, false, false);
        gamma.frequency = OperatingFrequency::Gamma;
        let mut delta = make_test_event("Reviewer", 3.0, 150, 40, 0, 1500, 5, 3, false, true);
        delta.frequency = OperatingFrequency::Delta;

        let profiles = compute_frequency_profiles(&[gamma, delta]);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].frequency, OperatingFrequency::Gamma);
        assert_eq!(profiles[0].observations, 1);
        assert_eq!(profiles[0].pass_rate, 0.0);
        assert_eq!(profiles[1].frequency, OperatingFrequency::Delta);
        assert_eq!(profiles[1].observations, 1);
        assert_eq!(profiles[1].pass_rate, 1.0);
    }

    #[test]
    fn efficiency_fleet_cfactor_groups_by_plan_and_agents() {
        let mut a1 = make_test_event("Implementer", 0.40, 900, 100, 0, 4000, 8, 4, false, true);
        a1.plan_id = "plan-a".into();
        a1.agent_id = "agent-a".into();
        let mut a2 = make_test_event("Reviewer", 0.20, 600, 80, 0, 3000, 8, 2, false, true);
        a2.plan_id = "plan-a".into();
        a2.agent_id = "agent-b".into();
        let mut b1 = make_test_event("Implementer", 0.10, 400, 50, 0, 1500, 8, 2, false, false);
        b1.plan_id = "plan-b".into();
        b1.agent_id = "agent-c".into();

        let fleet = compute_fleet_cfactor(&[a1, a2, b1]);
        assert_eq!(fleet.plan_count, 2);
        assert_eq!(fleet.agent_count, 3);
        assert_eq!(fleet.observation_count, 3);
        assert!(fleet.components.multi_agent_coverage > 0.0);
        assert!(fleet.components.turn_taking_equality > 0.0);
        assert!(fleet.overall >= 0.0);
    }

    #[test]
    fn efficiency_prompt_section_meta_serialization() {
        let s = PromptSectionMeta {
            name: "plan_brief".into(),
            tokens: 500,
            priority: 1,
            was_truncated: false,
            was_dropped: false,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let s2: PromptSectionMeta = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, s2);
    }

    #[test]
    fn efficiency_tool_call_meta_serialization() {
        let t = ToolCallMeta {
            tool_name: "Read".into(),
            duration_ms: 150,
            result_tokens: 800,
            succeeded: Some(true),
            advanced_task: true,
            was_redundant: false,
            error_category: None,
        };
        let json = serde_json::to_string(&t).expect("serialize");
        let t2: ToolCallMeta = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t, t2);
    }

    #[test]
    fn tool_call_reward_roundtrip_preserves_reward_indicators() {
        let meta = ToolCallMeta {
            tool_name: "Bash".into(),
            duration_ms: 875,
            result_tokens: 120,
            succeeded: Some(false),
            advanced_task: false,
            was_redundant: true,
            error_category: Some("timeout".into()),
        };

        let json = serde_json::to_string(&meta).expect("serialize");
        let restored: ToolCallMeta = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, meta);
    }

    #[test]
    fn tool_call_reward_defaults_for_legacy_payloads() {
        let json = r#"{
            "tool_name":"Read",
            "duration_ms":150,
            "result_tokens":800,
            "succeeded":true
        }"#;

        let restored: ToolCallMeta = serde_json::from_str(json).expect("deserialize");
        assert_eq!(restored.tool_name, "Read");
        assert_eq!(restored.succeeded, Some(true));
        assert!(!restored.advanced_task);
        assert!(!restored.was_redundant);
        assert_eq!(restored.error_category, None);
    }

    #[test]
    fn efficiency_profile_p95_cost() {
        // 20 events with increasing cost: 0.01, 0.02, ..., 0.20
        let events: Vec<AgentEfficiencyEvent> = (1..=20)
            .map(|i| {
                let cost = i as f64 * 0.01;
                make_test_event("Impl", cost, 1000, 200, 0, 10000, 10, 5, false, true)
            })
            .collect();

        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles.len(), 1);
        // P95 index for 20 elements: 20 * 95 / 100 = 19 → costs[19] = 0.20
        assert!((profiles[0].p95_cost_usd.expect("measured cost") - 0.20).abs() < 1e-9);
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn efficiency_event_zero_everything() {
        let e = AgentEfficiencyEvent::default();
        assert_eq!(e.total_tokens(), 0);
        assert!((e.cache_hit_rate()).abs() < 1e-9);
        assert!((e.tool_utilization()).abs() < 1e-9);
        assert!((e.cache_savings_usd()).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_max_tokens() {
        let mut e = AgentEfficiencyEvent::default();
        e.input_tokens = u64::MAX / 2;
        e.output_tokens = u64::MAX / 2;
        // Verify large token counts don't overflow (both halves of MAX)
        assert_eq!(e.total_tokens(), u64::MAX - 1);
    }

    #[test]
    fn efficiency_event_max_cost() {
        let mut e = AgentEfficiencyEvent::default();
        e.cost_usd = f64::MAX;
        e.cost_usd_without_cache = f64::MAX;
        assert!((e.cache_savings_usd()).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_cache_hit_rate_full_cache() {
        let e = make_test_event("Impl", 0.10, 1000, 200, 1000, 5000, 10, 5, false, true);
        assert!((e.cache_hit_rate() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn efficiency_event_tool_utilization_full() {
        let e = make_test_event("Impl", 0.10, 1000, 200, 0, 5000, 10, 10, false, true);
        assert!((e.tool_utilization() - 1.0).abs() < 1e-9);
    }

    // ── Serialization ───────────────────────────────────────────────

    #[test]
    fn efficiency_role_cost_profile_serialization_roundtrip() {
        let profile = RoleCostProfile {
            role: "Implementer".into(),
            observations: 42,
            avg_input_tokens: 1500.5,
            avg_output_tokens: 300.2,
            avg_cache_hit_rate: 0.35,
            avg_cost_usd: Some(0.55),
            p95_cost_usd: Some(1.20),
            cost_per_pass: Some(0.75),
            cost_unknown: 0,
            avg_tool_utilization: 0.6,
            avg_wall_time_ms: 12000.0,
            warm_start_pct: 0.4,
            pass_rate: 0.8,
        };
        let json = serde_json::to_string(&profile).expect("serialize");
        let p2: RoleCostProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, p2);
    }

    #[test]
    fn efficiency_fleet_cfactor_serialization_roundtrip() {
        let fleet = FleetCFactor {
            overall: 0.72,
            components: FleetCFactorComponents {
                multi_agent_coverage: 0.5,
                pass_rate: 0.8,
                cost_efficiency: 0.6,
                speed: 0.7,
                turn_taking_equality: 0.9,
            },
            plan_count: 3,
            agent_count: 5,
            observation_count: 12,
        };
        let json = serde_json::to_string(&fleet).expect("serialize");
        let f2: FleetCFactor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(fleet, f2);
    }

    #[test]
    fn efficiency_frequency_cost_profile_serialization_roundtrip() {
        let profile = FrequencyCostProfile {
            frequency: OperatingFrequency::Gamma,
            observations: 10,
            avg_cost_usd: Some(0.42),
            total_cost_usd: Some(4.20),
            pass_rate: 0.7,
            cost_per_pass: Some(0.60),
            cost_unknown: 0,
        };
        let json = serde_json::to_string(&profile).expect("serialize");
        let p2: FrequencyCostProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(profile, p2);
    }

    // ── Role cost profile edge cases ────────────────────────────────

    #[test]
    fn efficiency_role_cost_per_successful_task_zero_pass_rate() {
        let profile = RoleCostProfile {
            role: "Broken".into(),
            observations: 5,
            avg_input_tokens: 1000.0,
            avg_output_tokens: 200.0,
            avg_cache_hit_rate: 0.0,
            avg_cost_usd: Some(1.0),
            p95_cost_usd: Some(1.5),
            cost_per_pass: None,
            cost_unknown: 0,
            avg_tool_utilization: 0.5,
            avg_wall_time_ms: 10000.0,
            warm_start_pct: 0.0,
            pass_rate: 0.0,
        };
        assert_eq!(profile.cost_per_successful_task(), Some(f64::INFINITY));
    }

    #[test]
    fn efficiency_role_cost_per_successful_task_negative_pass_rate() {
        // Shouldn't happen in practice but the code handles it
        let profile = RoleCostProfile {
            role: "Weird".into(),
            observations: 1,
            avg_input_tokens: 0.0,
            avg_output_tokens: 0.0,
            avg_cache_hit_rate: 0.0,
            avg_cost_usd: Some(1.0),
            p95_cost_usd: Some(0.0),
            cost_per_pass: None,
            cost_unknown: 0,
            avg_tool_utilization: 0.0,
            avg_wall_time_ms: 0.0,
            warm_start_pct: 0.0,
            pass_rate: -0.1,
        };
        assert_eq!(profile.cost_per_successful_task(), Some(f64::INFINITY));
    }

    // ── Fleet C-Factor edge cases ───────────────────────────────────

    #[test]
    fn efficiency_fleet_cfactor_empty_events() {
        let fleet = compute_fleet_cfactor(&[]);
        assert_eq!(fleet.plan_count, 0);
        assert_eq!(fleet.agent_count, 0);
        assert_eq!(fleet.observation_count, 0);
        assert!((fleet.overall).abs() < 1e-9);
    }

    #[test]
    fn efficiency_fleet_cfactor_empty_plan_ids_skipped() {
        let mut e = make_test_event("Impl", 0.10, 100, 20, 0, 1000, 5, 2, false, true);
        e.plan_id = String::new();
        let fleet = compute_fleet_cfactor(&[e]);
        assert_eq!(fleet.plan_count, 0);
    }

    #[test]
    fn efficiency_fleet_cfactor_single_agent_single_plan() {
        let e = make_test_event("Impl", 0.50, 1000, 200, 0, 5000, 10, 5, false, true);
        let fleet = compute_fleet_cfactor(&[e]);
        assert_eq!(fleet.plan_count, 1);
        assert_eq!(fleet.agent_count, 1);
        assert_eq!(fleet.observation_count, 1);
        // Single agent → multi_agent_coverage = 0, turn_taking_equality = 0
        assert!((fleet.components.multi_agent_coverage).abs() < 1e-9);
        assert!((fleet.components.turn_taking_equality).abs() < 1e-9);
        assert!((fleet.components.pass_rate - 1.0).abs() < 1e-9);
    }

    // ── Compute helpers ─────────────────────────────────────────────

    #[test]
    fn has_known_cost_reads_a_zero_after_tokens_as_unknown() {
        let mut event = AgentEfficiencyEvent::default_event();
        assert!(
            event.has_known_cost(),
            "no tokens at $0 is a confirmed free turn"
        );
        event.input_tokens = 10;
        assert!(!event.has_known_cost(), "tokens at $0 were never priced");
        event.cost_usd = 0.01;
        assert!(event.has_known_cost());
    }

    #[test]
    fn role_profiles_skip_unknown_costs() {
        // Two priced turns and one that consumed tokens but recorded $0.
        let events = vec![
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.30, 1000, 200, 0, 10000, 10, 5, false, false),
            make_test_event("Impl", 0.0, 1000, 200, 0, 10000, 10, 5, false, true),
        ];
        let p = &compute_role_profiles(&events)[0];
        assert_eq!(p.observations, 3);
        assert_eq!(p.cost_unknown, 1);
        // $0.40, not the $0.2667 that averaging the unmeasured turn as free gives.
        assert!((p.avg_cost_usd.expect("measured cost") - 0.40).abs() < 1e-9);
        // One measured turn passed: $0.80 per pass.
        assert!((p.cost_per_pass.expect("a measured turn passed") - 0.80).abs() < 1e-9);
        // The pass rate still covers every turn.
        assert!((p.pass_rate - 2.0 / 3.0).abs() < 1e-9);

        // A role whose costs were never measured has no cost figures at all.
        let unmeasured = vec![make_test_event(
            "Bench", 0.0, 100, 20, 0, 1000, 0, 0, false, true,
        )];
        let p = &compute_role_profiles(&unmeasured)[0];
        assert_eq!(p.cost_unknown, 1);
        assert_eq!(p.avg_cost_usd, None);
        assert_eq!(p.p95_cost_usd, None);
        assert_eq!(p.cost_per_pass, None);
        assert_eq!(p.cost_per_successful_task(), None);
    }

    #[test]
    fn efficiency_role_profiles_empty_input() {
        let profiles = compute_role_profiles(&[]);
        assert!(profiles.is_empty());
    }

    #[test]
    fn frequency_profiles_skip_unknown_costs() {
        // Two priced turns and one that consumed tokens but recorded $0.
        let events = vec![
            make_test_event("Impl", 0.50, 1000, 200, 0, 10000, 10, 5, false, true),
            make_test_event("Impl", 0.30, 1000, 200, 0, 10000, 10, 5, false, false),
            make_test_event("Impl", 0.0, 1000, 200, 0, 10000, 10, 5, false, true),
        ];
        let p = &compute_frequency_profiles(&events)[0];
        assert_eq!(p.observations, 3);
        assert_eq!(p.cost_unknown, 1);
        assert!((p.total_cost_usd.expect("measured cost") - 0.80).abs() < 1e-9);
        assert!((p.avg_cost_usd.expect("measured cost") - 0.40).abs() < 1e-9);
        assert!((p.cost_per_pass.expect("a measured turn passed") - 0.80).abs() < 1e-9);
        assert!((p.pass_rate - 2.0 / 3.0).abs() < 1e-9);

        // No measured turn: no cost figures, never $0.
        let unmeasured = vec![make_test_event(
            "Bench", 0.0, 100, 20, 0, 1000, 0, 0, false, true,
        )];
        let p = &compute_frequency_profiles(&unmeasured)[0];
        assert_eq!(p.cost_unknown, 1);
        assert_eq!(p.total_cost_usd, None);
        assert_eq!(p.avg_cost_usd, None);
        assert_eq!(p.cost_per_pass, None);
    }

    #[test]
    fn efficiency_frequency_profiles_empty_input() {
        let profiles = compute_frequency_profiles(&[]);
        assert!(profiles.is_empty());
    }

    #[test]
    fn efficiency_role_profile_single_event() {
        let events = vec![make_test_event(
            "Solo", 0.42, 900, 180, 450, 7500, 8, 3, true, true,
        )];
        let profiles = compute_role_profiles(&events);
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.role, "Solo");
        assert_eq!(p.observations, 1);
        assert!((p.avg_cost_usd.expect("measured cost") - 0.42).abs() < 1e-9);
        assert!((p.avg_input_tokens - 900.0).abs() < 1e-9);
        assert!((p.avg_output_tokens - 180.0).abs() < 1e-9);
        assert!((p.avg_cache_hit_rate - 0.5).abs() < 1e-9); // 450/900
        assert!((p.pass_rate - 1.0).abs() < 1e-9);
        assert!((p.warm_start_pct - 1.0).abs() < 1e-9);
        assert!((p.cost_per_pass.expect("measured pass") - 0.42).abs() < 1e-9);
    }

    // ── Default event helper ────────────────────────────────────────

    #[test]
    fn efficiency_default_event_is_zeroed() {
        let e = AgentEfficiencyEvent::default_event();
        assert!(e.agent_id.is_empty());
        assert_eq!(e.input_tokens, 0);
        assert_eq!(e.output_tokens, 0);
        assert_eq!(e.gate_passed, None);
        assert!(!e.was_warm_start);
        assert!(e.prompt_sections.is_empty());
        assert!(e.tool_calls.is_empty());
    }

    // ── Gini coefficient internals ──────────────────────────────────

    #[test]
    fn efficiency_gini_perfect_equality() {
        // All agents have equal turns → Gini = 0
        let g = gini_coefficient(&[10, 10, 10, 10]);
        assert!(g.abs() < 1e-9);
    }

    #[test]
    fn efficiency_gini_maximum_inequality() {
        // One agent does everything → Gini close to 1
        let g = gini_coefficient(&[0, 0, 0, 100]);
        assert!(g > 0.7); // approaches 0.75 for n=4
    }

    #[test]
    fn efficiency_gini_single_element() {
        let g = gini_coefficient(&[42]);
        assert!(g.abs() < 1e-9);
    }

    #[test]
    fn efficiency_gini_empty() {
        let g = gini_coefficient(&[]);
        assert!(g.abs() < 1e-9);
    }

    #[test]
    fn efficiency_gini_all_zeros() {
        let g = gini_coefficient(&[0, 0, 0]);
        assert!(g.abs() < 1e-9);
    }

    #[test]
    fn efficiency_turn_taking_equality_single_agent() {
        let eq = turn_taking_equality_for_counts(vec![10]);
        assert!(eq.abs() < 1e-9); // < 2 agents → 0
    }

    #[test]
    fn efficiency_turn_taking_equality_even_split() {
        let eq = turn_taking_equality_for_counts(vec![5, 5, 5]);
        assert!((eq - 1.0).abs() < 1e-9);
    }

    // ── null_ambiguous_gate_failed_entries ──────────────────────────

    /// Audit #80 Step 6: entries with gate_passed=false AND a success-like
    /// outcome must be patched to gate_passed=null.
    #[test]
    fn null_ambiguous_gate_failed_rewrites_false_for_empty_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("efficiency.jsonl");

        // Write three entries:
        //   1. gate_passed=false, outcome="" → should be patched
        //   2. gate_passed=false, outcome="failure" → genuine; keep
        //   3. gate_passed=true  → already correct; keep
        let lines = vec![
            serde_json::json!({"gate_passed": false, "outcome": ""}),
            serde_json::json!({"gate_passed": false, "outcome": "failure"}),
            serde_json::json!({"gate_passed": true,  "outcome": "success"}),
        ];
        let content: String = lines.iter().map(|v| format!("{v}\n")).collect();
        std::fs::write(&path, &content).unwrap();

        let patched = null_ambiguous_gate_failed_entries(&path).unwrap();
        assert_eq!(patched, 1, "exactly one ambiguous entry should be patched");

        let result = std::fs::read_to_string(&path).unwrap();
        let parsed_lines: Vec<serde_json::Value> = result
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(parsed_lines.len(), 3);
        // Entry 1: gate_passed must now be null.
        assert!(
            parsed_lines[0]["gate_passed"].is_null(),
            "ambiguous entry must become null, got {:?}",
            parsed_lines[0]["gate_passed"]
        );
        // Entry 2: gate_passed must remain false (genuine failure).
        assert_eq!(parsed_lines[1]["gate_passed"], serde_json::json!(false));
        // Entry 3: gate_passed must remain true.
        assert_eq!(parsed_lines[2]["gate_passed"], serde_json::json!(true));
    }

    /// Audit #80 Step 6: missing file should return Ok(0) without error.
    #[test]
    fn null_ambiguous_gate_failed_missing_file_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.jsonl");
        let result = null_ambiguous_gate_failed_entries(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    /// Audit #80 Step 6: running the migration twice must be idempotent
    /// (second run patches 0 entries).
    #[test]
    fn null_ambiguous_gate_failed_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("efficiency.jsonl");
        let content = serde_json::json!({"gate_passed": false, "outcome": ""}).to_string() + "\n";
        std::fs::write(&path, &content).unwrap();

        let first = null_ambiguous_gate_failed_entries(&path).unwrap();
        assert_eq!(first, 1);

        // Second run should see gate_passed=null and patch nothing.
        let second = null_ambiguous_gate_failed_entries(&path).unwrap();
        assert_eq!(second, 0, "second migration run must be idempotent");
    }
}
