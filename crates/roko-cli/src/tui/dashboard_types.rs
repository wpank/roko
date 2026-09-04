//! Dashboard data types used across the TUI.
//!
//! These are the shared structs and enums consumed by views, widgets, state,
//! and loaders. Factored out of `dashboard.rs` to keep that file focused on
//! scaffold construction and data loading.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use roko_core::config::model_registry::model_meta;
use roko_learn::efficiency::AgentEfficiencyEvent;

use super::display_utils::event_model_slug;

// ── Task / Agent summaries ──────────────────────────────────────────────

/// Summary of a task that is currently active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSummary {
    pub plan_id: String,
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub iteration: u32,
    #[serde(default)]
    pub assigned_agents: Vec<String>,
    #[serde(default)]
    pub latest_gate: Option<String>,
}

/// Summary of an agent tracked by the process supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSummary {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub plan_id: Option<String>,
    pub status: String,
}

// ── Agent activity ──────────────────────────────────────────────────────

/// Aggregated agent-activity row used by the dashboard page.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AgentActivityRow {
    pub agent_id: String,
    pub model: String,
    pub task: String,
    pub role: String,
    pub turns: usize,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub uptime_ms: u64,
}

/// Model usage count for the bar chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelUsageRow {
    pub label: String,
    pub count: u64,
}

/// Per-model cost row for the breakdown table.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelCostRow {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub input_rate: f64,
    pub output_rate: f64,
    pub cost_usd: f64,
    /// True when at least one event's cost was estimated from registry rates
    /// because the event recorded `cost_usd: 0.0` despite consuming tokens
    /// (see `Usage::has_known_cost` semantics in roko-core).
    pub cost_estimated: bool,
}

/// Aggregated agent activity snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AgentActivitySnapshot {
    pub active_agents: Vec<AgentActivityRow>,
    pub model_usage: Vec<ModelUsageRow>,
    pub cost_rows: Vec<ModelCostRow>,
    pub total_session_cost: f64,
}

// ── Gate types ───────────────────────────────────────────────────────────

/// Summary of one gate verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResultSummary {
    pub plan_id: String,
    pub gate_name: String,
    pub passed: bool,
    pub rung: u32,
    pub duration_ms: u64,
    pub summary: String,
}

/// Verify signal summary used to derive the gate-results page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateSignalSummary {
    pub id: String,
    pub created_at_ms: i64,
    pub plan_id: Option<String>,
    pub task_id: Option<String>,
    pub gate_name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub excerpt: String,
}

/// Shared gate-results dashboard data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GateResultsPageData {
    pub gate_rows: Vec<GateSummaryRow>,
    pub threshold_rows: Vec<GateThresholdRow>,
    pub failure_rows: Vec<GateFailureRow>,
}

/// Aggregate row for the gate summary table.
#[derive(Debug, Clone, PartialEq)]
pub struct GateSummaryRow {
    pub gate_name: String,
    pub total_runs: u64,
    pub pass_rate: f64,
    pub avg_duration_ms: f64,
    pub last_run: String,
}

/// Row for the adaptive threshold table.
#[derive(Debug, Clone, PartialEq)]
pub struct GateThresholdRow {
    pub rung: u32,
    pub current_threshold: u32,
    pub ema_pass_rate: f64,
    pub trend: GateTrend,
}

/// Recent failing gate row.
#[derive(Debug, Clone, PartialEq)]
pub struct GateFailureRow {
    pub created_at_ms: i64,
    pub task_id: String,
    pub gate_name: String,
    pub error_excerpt: String,
}

/// Derived EMA trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateTrend {
    Up,
    Flat,
    Down,
}

// ── Plan execution types ────────────────────────────────────────────────

/// Snapshot of the currently executing plan.
#[derive(Debug, Clone, Default)]
pub struct PlanExecutionSnapshot {
    pub plan_id: String,
    pub plan_title: String,
    pub tasks_done: usize,
    pub tasks_total: usize,
    pub tasks: Vec<PlanExecutionTaskRow>,
    pub current_task: Option<PlanExecutionTaskDetail>,
    pub agent_output_tail: Vec<String>,
}

/// One row in the execution task table.
#[derive(Debug, Clone)]
pub struct PlanExecutionTaskRow {
    pub task_id: String,
    pub title: String,
    pub phase: String,
    pub model: String,
    pub duration: String,
    pub is_current: bool,
}

/// Detail block for the current task.
#[derive(Debug, Clone)]
pub struct PlanExecutionTaskDetail {
    pub task_id: String,
    pub description: String,
    pub read_files: Vec<ReadFileSnapshot>,
    pub write_files: Vec<String>,
}

/// Flattened read-file context for display.
#[derive(Debug, Clone)]
pub struct ReadFileSnapshot {
    pub path: String,
    pub lines: Option<String>,
    pub why: String,
}

/// Lightweight task snapshot used by the interactive TUI plan views.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct PlanTaskSnapshot {
    pub id: String,
    pub title: String,
    pub tier: String,
    pub model_hint: Option<String>,
    pub status: String,
    pub agent_id: Option<String>,
    pub model: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub wave: Option<u32>,
    /// Task IDs this task depends on (from tasks.toml).
    pub dependencies: Vec<String>,
    /// Free-form acceptance criteria text (from tasks.toml).
    pub acceptance_text: Option<String>,
    /// First verify command, if any (from tasks.toml).
    pub verify_command: Option<String>,
    /// Files this task will create or modify (from tasks.toml).
    pub files: Vec<String>,
}

/// Per-plan snapshot used to hydrate `TuiState::plans`.
#[derive(Debug, Clone, Default)]
pub(crate) struct PlanTaskListSnapshot {
    pub phase: String,
    pub active: bool,
    pub tasks_done: usize,
    pub tasks_failed: usize,
    pub elapsed_ms: u64,
    pub elapsed_secs: f64,
    /// Current wave number for the plan.
    pub wave: u32,
    /// Count of failed tasks, including gate rejections.
    pub failed_count: u32,
    pub tasks: Vec<PlanTaskSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PlanTaskRuntimeFields {
    pub model: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub wave: Option<u32>,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedPlanTasksFile {
    pub tasks_file: crate::task_parser::TasksFile,
    pub task_runtime_fields: Vec<PlanTaskRuntimeFields>,
    pub plan_wave: Option<u32>,
}

// ── Learning types ──────────────────────────────────────────────────────

/// Aggregate learning efficiency snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EfficiencySummary {
    pub event_count: usize,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub passed_count: usize,
    pub average_wall_time_ms: f64,
}

impl Default for EfficiencySummary {
    fn default() -> Self {
        Self {
            event_count: 0,
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            passed_count: 0,
            average_wall_time_ms: 0.0,
        }
    }
}

/// Cascade router snapshot from `.roko/learn/cascade-router.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CascadeRouterState {
    #[serde(default)]
    pub model_slugs: Vec<String>,
    #[serde(default)]
    pub confidence_stats: HashMap<String, CascadeRouterModelStats>,
}

/// Per-model cascade-router stats.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeRouterModelStats {
    pub trials: u64,
    pub successes: u64,
}

/// Prompt experiment summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub experiment_id: String,
    pub section_name: String,
    pub status: String,
    #[serde(default)]
    pub winner_id: Option<String>,
    pub active_variants: usize,
    pub total_trials: u64,
}

/// Recent signal summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalSummary {
    pub id: String,
    pub kind: String,
    pub created_at_ms: i64,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub parent_hash: Option<String>,
    #[serde(default)]
    pub lineage: Vec<String>,
    #[serde(default)]
    pub payload_preview: String,
}

/// Conductor alert summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertSummary {
    pub id: String,
    pub kind: String,
    pub created_at_ms: i64,
    pub severity: String,
    pub message: String,
}

impl AlertSummary {
    pub(super) fn from_signal(signal: &SignalSummary) -> Self {
        let severity = signal
            .kind
            .split(':')
            .nth(2)
            .unwrap_or("warning")
            .to_string();
        Self {
            id: signal.id.clone(),
            kind: signal.kind.clone(),
            created_at_ms: signal.created_at_ms,
            severity,
            message: signal.kind.clone(),
        }
    }
}

/// Lightweight knowledge-entry summary for the Inspect tab's KnowledgeBrowse sub-view.
///
/// Projected from `roko_neuro::KnowledgeEntry` to avoid pulling HDC vectors
/// and other heavy fields into the TUI state.
#[derive(Debug, Clone)]
pub struct KnowledgeBrowseEntry {
    /// Entry identifier.
    pub id: String,
    /// Knowledge category label (e.g. "insight", "heuristic", "pattern").
    pub kind: String,
    /// Truncated content for preview.
    pub content_preview: String,
    /// Confidence score 0.0..=1.0.
    pub confidence: f64,
    /// Tier label (transient, working, consolidated, persistent).
    pub tier: String,
    /// Topic tags.
    pub tags: Vec<String>,
    /// When the entry was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether the entry is frozen (cold storage).
    pub frozen: bool,
}

// ── Agent activity builder ──────────────────────────────────────────────

/// Build the agent activity snapshot from active agents and efficiency events.
pub(crate) fn build_agent_activity_snapshot(
    active_agents: &[AgentSummary],
    efficiency_events: &[AgentEfficiencyEvent],
) -> Option<AgentActivitySnapshot> {
    let agents = if active_agents.is_empty() {
        synthesize_agents_from_events(efficiency_events)
    } else {
        active_agents.to_vec()
    };

    if agents.is_empty() && efficiency_events.is_empty() {
        return None;
    }

    let mut rows_by_agent: HashMap<String, AgentActivityAggregate> = HashMap::new();
    for agent in &agents {
        rows_by_agent
            .entry(agent.id.clone())
            .or_insert_with(AgentActivityAggregate::default);
    }

    for event in efficiency_events {
        let entry = rows_by_agent
            .entry(event.agent_id.clone())
            .or_insert_with(AgentActivityAggregate::default);
        entry.turns += 1;
        entry.tokens_used += event.input_tokens + event.output_tokens;
        entry.cost_usd += event.cost_usd;
        entry.update_from_event(event);
    }

    let now = Utc::now();
    let mut active_rows = agents
        .iter()
        .map(|agent| {
            let aggregate = rows_by_agent
                .entry(agent.id.clone())
                .or_insert_with(AgentActivityAggregate::default);
            aggregate.render_row(agent, now)
        })
        .collect::<Vec<_>>();
    active_rows.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));

    // Group by model family + exact slug (same pattern as
    // `views/learning_view.rs`), not the legacy haiku/sonnet/opus trio.
    let mut model_usage: BTreeMap<(&'static str, String), u64> = BTreeMap::new();
    let mut cost_groups: BTreeMap<String, ModelCostAggregate> = BTreeMap::new();
    for event in efficiency_events {
        let slug = event_model_slug(event);
        let meta = model_meta(&slug);
        *model_usage.entry((meta.family, slug.clone())).or_default() += 1;
        let (input_rate, output_rate) = meta.pricing.map_or((0.0, 0.0), |pricing| {
            (
                pricing.input_per_m / 1_000_000.0,
                pricing.output_per_m / 1_000_000.0,
            )
        });
        let aggregate = cost_groups
            .entry(slug.clone())
            .or_insert_with(|| ModelCostAggregate {
                model: slug.clone(),
                input_rate,
                output_rate,
                ..ModelCostAggregate::default()
            });
        aggregate.input_tokens += event.input_tokens;
        aggregate.output_tokens += event.output_tokens;
        if event_has_known_cost(event) {
            aggregate.real_cost_usd += event.cost_usd;
        } else {
            aggregate.events_with_unknown_cost += 1;
            if let Some(pricing) = meta.pricing {
                aggregate.estimated_cost_usd += (event.input_tokens as f64 * pricing.input_per_m
                    + event.output_tokens as f64 * pricing.output_per_m
                    + event.cache_read_tokens as f64 * pricing.cache_read_per_m
                    + event.cache_write_tokens as f64 * pricing.cache_write_per_m)
                    / 1_000_000.0;
            }
        }
    }

    let mut cost_rows = cost_groups
        .into_values()
        .map(|group| group.into_row())
        .collect::<Vec<_>>();
    cost_rows.sort_by(|a, b| {
        b.cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.model.cmp(&b.model))
    });

    let model_usage = model_usage
        .into_iter()
        .map(|((_family, slug), count)| ModelUsageRow { label: slug, count })
        .collect::<Vec<_>>();

    let total_session_cost = cost_rows.iter().map(|row| row.cost_usd).sum();

    Some(AgentActivitySnapshot {
        active_agents: active_rows,
        model_usage,
        cost_rows,
        total_session_cost,
    })
}

fn synthesize_agents_from_events(efficiency_events: &[AgentEfficiencyEvent]) -> Vec<AgentSummary> {
    let mut agents = BTreeMap::<String, AgentSummary>::new();
    for event in efficiency_events {
        agents
            .entry(event.agent_id.clone())
            .or_insert_with(|| AgentSummary {
                id: event.agent_id.clone(),
                label: event.agent_id.clone(),
                plan_id: Some(event.plan_id.clone()),
                status: String::from("active"),
            });
    }
    agents.into_values().collect()
}

/// Whether an efficiency event's cost is known, mirroring the wave-1
/// `Usage::has_known_cost` semantics (`roko-core/src/chat_types.rs`): cost is
/// known when it is non-zero or when no tokens were consumed (a confirmed
/// free turn). Token-consuming turns recorded with `cost_usd: 0.0` have
/// *unknown* cost and must be estimated -- never silently priced as $0.00.
fn event_has_known_cost(event: &AgentEfficiencyEvent) -> bool {
    event.cost_usd.abs() > f64::EPSILON
        || (event.input_tokens + event.output_tokens + event.cache_write_tokens == 0
            && event.cache_read_tokens == 0)
}

#[derive(Debug, Default)]
struct AgentActivityAggregate {
    model: String,
    task: String,
    role: String,
    turns: usize,
    tokens_used: u64,
    cost_usd: f64,
    first_seen_at: Option<DateTime<Utc>>,
    latest_event_at: Option<DateTime<Utc>>,
}

impl AgentActivityAggregate {
    fn update_from_event(&mut self, event: &AgentEfficiencyEvent) {
        let Some(timestamp) = parse_efficiency_timestamp(&event.timestamp) else {
            return;
        };
        if self.first_seen_at.map_or(true, |first| timestamp < first) {
            self.first_seen_at = Some(timestamp);
        }
        if self
            .latest_event_at
            .map_or(true, |latest| timestamp > latest)
        {
            self.latest_event_at = Some(timestamp);
            self.model = event.model.clone();
            self.task = event.task_id.clone();
            self.role = event.role.clone();
        }
    }

    fn render_row(&self, agent: &AgentSummary, now: DateTime<Utc>) -> AgentActivityRow {
        let uptime_ms = self
            .first_seen_at
            .and_then(|first| {
                now.signed_duration_since(first)
                    .num_milliseconds()
                    .try_into()
                    .ok()
            })
            .unwrap_or_default();
        AgentActivityRow {
            agent_id: agent.id.clone(),
            model: if self.model.is_empty() {
                String::from("-")
            } else {
                self.model.clone()
            },
            task: if self.task.is_empty() {
                agent.plan_id.clone().unwrap_or_else(|| String::from("-"))
            } else {
                self.task.clone()
            },
            role: if self.role.is_empty() {
                agent.status.clone()
            } else {
                self.role.clone()
            },
            turns: self.turns,
            tokens_used: self.tokens_used,
            cost_usd: self.cost_usd,
            uptime_ms,
        }
    }
}

#[derive(Debug, Default)]
struct ModelCostAggregate {
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    input_rate: f64,
    output_rate: f64,
    /// Sum of `event.cost_usd` across events whose cost is known.
    real_cost_usd: f64,
    /// Registry-priced estimate across events whose cost is unknown.
    estimated_cost_usd: f64,
    /// Number of events whose cost had to be estimated.
    events_with_unknown_cost: u64,
}

impl ModelCostAggregate {
    fn into_row(self) -> ModelCostRow {
        ModelCostRow {
            model: self.model,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            input_rate: self.input_rate,
            output_rate: self.output_rate,
            cost_usd: self.real_cost_usd + self.estimated_cost_usd,
            cost_estimated: self.events_with_unknown_cost > 0,
        }
    }
}

pub(super) fn parse_efficiency_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}
