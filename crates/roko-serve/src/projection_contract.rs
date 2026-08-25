//! Projection contract hardening (SURF-GAP-05).
//!
//! Defines versioning, invalidation rules, and restart recovery behavior
//! for StateHub projections consumed by TUI and dashboard surfaces.
//!
//! Every projection response can be wrapped in a [`ProjectionEnvelope`] that
//! carries schema version, monotonic cursor, staleness information, and a
//! flag indicating whether the data was recovered from disk after a restart.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
pub use roko_core::agent::AutonomyConfig;
use roko_core::dashboard_snapshot::{
    DashboardEvent, DashboardSnapshot, InboxCategory, UrgencyLevel,
};
use roko_core::telemetry_projections::{
    ActiveTasksProjection, AgentVitalityProjection, AgentVitalitySnapshot, CFactorProjection,
    CohortHealthProjection, CostMeterProjection, GatePipelineProjection, KnowledgeHealthProjection,
    RungSnapshot, TaskSnapshot,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::costs_log::CostsLog;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::episode_logger::Episode;
use roko_learn::provider_health::{HealthState, ProviderStatus};
use roko_learn::provider_model_outcome::{
    ProviderModelOutcomeRecord, ProviderModelOutcomeStatus, read_provider_model_outcomes,
};
use roko_learn::runtime_feedback::{
    read_project_efficiency_events, read_project_episodes_lossy, resolve_project_episode_path,
};
use roko_runtime::{DurableRunnerProjection, RunnerProjectionSource};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use crate::error::ApiError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/// Metadata envelope for a projection response.
///
/// Wraps the actual projection data with versioning and freshness metadata
/// so that consumers can detect stale data, handle schema changes, and
/// distinguish recovered (post-restart) state from live state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionEnvelope<T> {
    /// Projection name (e.g., `"marketplace_jobs"`, `"active_tasks"`).
    pub name: String,
    /// Schema version for this projection format.
    ///
    /// Bumped when the shape of the inner `data` changes in a breaking way.
    /// Consumers should reject versions they do not understand.
    pub version: u32,
    /// Monotonic sequence number from the StateHub event bus.
    pub cursor: u64,
    /// When this snapshot was computed (RFC 3339).
    pub computed_at: String,
    /// Whether this projection was loaded from recovery (restart) state
    /// rather than built from a live event stream.
    pub recovered: bool,
    /// The actual projection data.
    pub data: T,
}

/// Status of a Flow rendered in the Workbench.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FlowStatus {
    Running,
    Paused,
    WaitingHuman,
    Completed { verdict: String },
    Failed { error: String },
    Cancelled,
}

/// Human response requested by one Flow cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanInputRequest {
    pub run_id: String,
    pub cell_id: String,
    pub prompt: String,
    pub urgency: UrgencyLevel,
    pub deadline: Option<DateTime<Utc>>,
}

/// Workbench summary for one active or recently completed Flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowSummary {
    pub run_id: String,
    pub graph_name: String,
    pub progress_pct: f64,
    pub cost_usd: f64,
    pub elapsed: Duration,
    pub status: FlowStatus,
    pub active_nodes: Vec<String>,
    pub pending_human: Option<HumanInputRequest>,
}

/// Current assignment and health for one agent slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotState {
    pub agent_id: String,
    pub slot_index: usize,
    pub occupied: bool,
    pub current_task: Option<String>,
    pub vitality: f64,
}

/// Complete typed input consumed by the Workbench surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchProjection {
    pub flows: Vec<FlowSummary>,
    pub slots: Vec<SlotState>,
    pub gate_pipeline: GatePipelineProjection,
    pub cost_meter: CostMeterProjection,
}

/// One unresolved item rendered by the Agent Inbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItem {
    pub id: String,
    pub urgency: UrgencyLevel,
    pub category: InboxCategory,
    pub summary: String,
    pub detail: Value,
    pub source_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub blocking_run: Option<String>,
}

/// Complete typed input consumed by the Agent Inbox surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboxProjection {
    pub items: Vec<InboxItem>,
    pub pending_count: usize,
    pub agent_vitality: AgentVitalityProjection,
    pub cohort_health: CohortHealthProjection,
}

/// Typed live overlay for the Generative Canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasProjection {
    pub graph_names: Vec<String>,
    /// Source used for graph identities until GraphRegistry joins AppState.
    pub graph_source: String,
    pub active_tasks: ActiveTasksProjection,
    pub gate_pipeline: GatePipelineProjection,
}

/// One agent's deterministic position and live health in the Minimap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPosition {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub status: String,
    pub vitality: f64,
    pub profile: String,
    pub current_task: Option<String>,
}

/// Named c-factor components used by the coordination surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CFactorSummary {
    pub overall: f64,
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub hdc_diversity: f64,
}

/// Complete typed input consumed by the Stigmergy Minimap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinimapProjection {
    pub agents: Vec<AgentPosition>,
    /// Coordinate source; currently a stable layout, not an HDC embedding.
    pub position_source: String,
    pub c_factor: CFactorSummary,
    pub cohort_health: CohortHealthProjection,
    pub knowledge_health: KnowledgeHealthProjection,
}

/// Complete typed input consumed by the Autonomy Slider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyProjection {
    pub configs: Vec<AutonomyConfig>,
    /// Explicit source state so an empty list is never mistaken for defaults.
    pub config_source: String,
    pub agent_vitality: AgentVitalityProjection,
    pub c_factor: CFactorSummary,
}

/// Data quality summary attached to responses that read from on-disk telemetry.
///
/// Consumers should check `has_real_data` before rendering charts or metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataQuality {
    /// Whether the source file existed and contained at least one entry.
    pub has_real_data: bool,
    /// Total number of entries read from the source.
    pub entry_count: usize,
    /// Number of entries where `cost_usd` was zero or absent.
    pub null_cost_count: usize,
}

impl DataQuality {
    /// Compute quality from a slice of efficiency events.
    pub fn from_efficiency_events(events: &[AgentEfficiencyEvent]) -> Self {
        Self::from_counts(
            !events.is_empty(),
            events.len(),
            events.iter().filter(|event| event.cost_usd == 0.0).count(),
        )
    }

    /// Construct a summary from explicit counts.
    pub const fn from_counts(
        has_real_data: bool,
        entry_count: usize,
        null_cost_count: usize,
    ) -> Self {
        Self {
            has_real_data,
            entry_count,
            null_cost_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Invalidation policy
// ---------------------------------------------------------------------------

/// Cache invalidation policy for a projection.
///
/// Describes how long a projection's cached value remains valid and which
/// events trigger eager invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationPolicy {
    /// Maximum age in seconds before the projection is considered stale.
    pub max_age_secs: u64,
    /// Whether the projection supports incremental (delta) updates.
    pub incremental: bool,
    /// Event types that trigger invalidation of this projection.
    pub invalidation_triggers: Vec<String>,
}

/// Catalog entry returned by `GET /api/projections/catalog`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionCatalogEntry {
    /// Projection name.
    pub name: String,
    /// Current schema version.
    pub version: u32,
    /// Invalidation policy.
    pub policy: InvalidationPolicy,
}

// ---------------------------------------------------------------------------
// Known projections
// ---------------------------------------------------------------------------

/// Returns the known projection names with their current schema versions
/// and invalidation policies.
pub fn projection_policies() -> Vec<ProjectionCatalogEntry> {
    vec![
        ProjectionCatalogEntry {
            name: "workbench".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: false,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "task_started".into(),
                    "task_completed".into(),
                    "agent_spawned".into(),
                    "agent_completed".into(),
                    "gate_result".into(),
                    "projection_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "inbox".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: false,
                invalidation_triggers: vec![
                    "inbox_item_received".into(),
                    "inbox_approve".into(),
                    "inbox_reject".into(),
                    "inbox_defer".into(),
                    "inbox_dismiss".into(),
                    "projection_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "canvas".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: false,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "plan_completed".into(),
                    "task_started".into(),
                    "task_completed".into(),
                    "gate_result".into(),
                    "projection_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "minimap".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: false,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "agent_completed".into(),
                    "projection_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "autonomy".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: false,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "agent_completed".into(),
                    "projection_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "dashboard".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec!["*".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "agent_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "agent_output".into(),
                    "agent_completed".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "plan_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "plan_completed".into(),
                    "task_started".into(),
                    "task_completed".into(),
                    "task_phase_changed".into(),
                    "phase_transition".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "gate_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec!["gate_result".into(), "gate_thresholds_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "learning_policy_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 30,
                incremental: true,
                invalidation_triggers: vec![
                    "experiment_winners_updated".into(),
                    "cfactor_trend_updated".into(),
                    "efficiency_event".into(),
                    "episode_recorded".into(),
                    "cascade_router_updated".into(),
                    "gate_thresholds_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "cohort_health".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "plan_completed".into(),
                    "task_started".into(),
                    "task_completed".into(),
                    "agent_spawned".into(),
                    "efficiency_event".into(),
                    "cfactor_trend_updated".into(),
                    "diagnosis".into(),
                    "error".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "active_tasks".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "task_started".into(),
                    "task_completed".into(),
                    "task_phase_changed".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "gate_pipeline".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec!["gate_result".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "agent_trails".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec!["agent_spawned".into(), "agent_output".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "alerts".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 15,
                incremental: true,
                invalidation_triggers: vec![
                    "diagnosis".into(),
                    "gate_result".into(),
                    "error".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "plans_list".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "plan_completed".into(),
                    "phase_transition".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "recent_episodes".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 30,
                incremental: true,
                invalidation_triggers: vec!["episode_recorded".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "event_log".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec!["event_log_entry".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "task_outputs".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec!["task_output_appended".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "cost_meter".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "efficiency_event".into(),
                    "agent_completed".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "knowledge_health".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 60,
                incremental: true,
                invalidation_triggers: vec!["knowledge_entries_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "c_factor".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec!["cfactor_trend_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "agent_vitality".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "agent_completed".into(),
                    "efficiency_event".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "cost_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "efficiency_event".into(),
                    "episode_recorded".into(),
                    "agent_completed".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "provider_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "efficiency_event".into(),
                    "episode_recorded".into(),
                    "agent_completed".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "retry_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "gate_result".into(),
                    "efficiency_event".into(),
                    "episode_recorded".into(),
                    "diagnosis".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "execution_trace".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec!["*".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "runtime_feedback".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 10,
                incremental: true,
                invalidation_triggers: vec![
                    "efficiency_event".into(),
                    "episode_recorded".into(),
                    "gate_result".into(),
                    "cascade_router_updated".into(),
                    "gate_thresholds_updated".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "executor_state".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "plan_started".into(),
                    "plan_completed".into(),
                    "task_started".into(),
                    "task_completed".into(),
                    "task_phase_changed".into(),
                    "agent_spawned".into(),
                    "agent_output".into(),
                    "agent_completed".into(),
                    "gate_result".into(),
                ],
            },
        },
        ProjectionCatalogEntry {
            name: "marketplace_jobs".into(),
            version: 2, // v2: includes `source` field (SURF-GAP-04)
            policy: InvalidationPolicy {
                max_age_secs: 30,
                incremental: true,
                invalidation_triggers: vec!["marketplace_jobs_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "prds".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 60,
                incremental: true,
                invalidation_triggers: vec!["prds_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "knowledge".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 60,
                incremental: true,
                invalidation_triggers: vec!["knowledge_entries_updated".into()],
            },
        },
        ProjectionCatalogEntry {
            name: "telemetry".into(),
            version: 1,
            policy: InvalidationPolicy {
                max_age_secs: 5,
                incremental: true,
                invalidation_triggers: vec![
                    "agent_spawned".into(),
                    "agent_completed".into(),
                    "efficiency_event".into(),
                    "diagnosis".into(),
                    "error".into(),
                    "gate_result".into(),
                ],
            },
        },
    ]
}

/// Look up the schema version for a named projection.
///
/// Returns `None` for unknown projections.
pub fn projection_version(name: &str) -> Option<u32> {
    // Normalise aliases.
    let canonical = match name {
        "dashboard_snapshot" => "dashboard",
        "agents" | "agent_trails" => "agent_state",
        "plans" | "plans_list" => "plan_state",
        "gates" => "gate_state",
        "learning" | "learning_policy" => "learning_policy_state",
        "events" => "event_log",
        "providers" | "provider_outcomes" => "provider_state",
        "retries" => "retry_state",
        "costs" => "cost_state",
        "trace" | "proof" => "execution_trace",
        "feedback" => "runtime_feedback",
        "jobs" => "marketplace_jobs",
        "atelier" => "prds",
        "knowledge_entries" => "knowledge",
        "watchers" | "circuit_breakers" | "observations" => "telemetry",
        _ => name,
    };
    projection_policies()
        .iter()
        .find(|entry| entry.name == canonical)
        .map(|entry| entry.version)
}

/// Query parameters shared by projection, status, and learning endpoints.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectionQuery {
    /// General-purpose filter string. Supports `key:value`, `key=value`, and bare identifiers.
    #[serde(default)]
    pub filter: Option<String>,
    /// Maximum number of rows to return for list-like projection fields.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Run identifier alias used by durable feedback records.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Plan identifier from StateHub or durable feedback records.
    #[serde(default)]
    pub plan_id: Option<String>,
    /// Task identifier from StateHub or durable feedback records.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Agent identifier from StateHub or durable feedback records.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Gate name to filter gate proof rows.
    #[serde(default)]
    pub gate: Option<String>,
    /// Agent role or template label.
    #[serde(default)]
    pub role: Option<String>,
    /// Dashboard event type filter.
    #[serde(default, alias = "type")]
    pub event_type: Option<String>,
    /// Active/inactive filter for runtime entities.
    #[serde(default)]
    pub active: Option<bool>,
    /// Provider/backend filter.
    #[serde(default)]
    pub provider: Option<String>,
    /// Model slug filter.
    #[serde(default)]
    pub model: Option<String>,
    /// Status filter such as `passed`, `failed`, `success`, or `error`.
    #[serde(default)]
    pub status: Option<String>,
    /// Episode identifier filter.
    #[serde(default)]
    pub episode_id: Option<String>,
}

/// Canonical runtime and learning-feedback data used by observability endpoints.
#[derive(Debug, Clone)]
pub struct RuntimeProjectionSet {
    /// Live or recovered dashboard snapshot.
    pub snapshot: DashboardSnapshot,
    /// Current StateHub sequence cursor.
    pub cursor: u64,
    /// Number of retained StateHub events.
    pub ring_len: usize,
    /// Wall-clock timestamp when this projection set was built.
    pub computed_at: DateTime<Utc>,
    /// Whether the dashboard snapshot was recovered from disk.
    pub recovered: bool,
    /// Durable runtime feedback loaded from `.roko/learn` and related stores.
    pub feedback: RuntimeFeedbackProjection,
    /// Current provider circuit-breaker health rows.
    pub provider_health: Vec<Value>,
    /// Materialized typed Lens projections supplied by StateHub.
    pub statehub_projections: BTreeMap<String, roko_runtime::ProjectionState>,
    /// Live bounded Lens queues and their operator-visible breaker states.
    pub lens_runtimes: Vec<roko_runtime::LensRuntimeSnapshot>,
}

/// Durable feedback inputs joined into runtime projection responses.
#[derive(Debug, Clone, Default)]
pub struct RuntimeFeedbackProjection {
    /// Durable runner/runtime event records from `.roko/events.jsonl`.
    pub runner_events: Vec<Value>,
    /// Legacy signal records from `.roko/engrams.jsonl`.
    pub signal_events: Vec<Value>,
    /// Episode records read from project learning stores.
    pub episodes: Vec<Episode>,
    /// Agent efficiency events read from `.roko/learn/efficiency.jsonl`.
    pub efficiency_events: Vec<AgentEfficiencyEvent>,
    /// Cost records read from `.roko/learn/costs.jsonl`.
    pub cost_records: Vec<CostRecord>,
    /// Provider/model outcome records read from `.roko/learn/provider-model-outcomes.jsonl`.
    pub provider_model_outcomes: Vec<ProviderModelOutcomeRecord>,
    /// Parsed cascade-router state when present.
    pub cascade_router_json: Option<Value>,
    /// Parsed gate-threshold state when present.
    pub gate_thresholds_json: Option<Value>,
    /// Parsed executor state when present.
    pub executor_state_json: Option<Value>,
    /// Durable format supplying executor state.
    pub runner_projection_source: Option<RunnerProjectionSource>,
    /// Stable identity of the exact Runner generation joined to StateHub.
    pub runner_projection_generation: Option<String>,
    /// Stable source status: state_snapshot, legacy_executor, missing, or invalid.
    pub runner_projection_status: String,
    /// Durable loader failure retained for every HTTP projection surface.
    pub runner_projection_error: Option<String>,
    /// Count of durable neuro knowledge entries.
    pub knowledge_entries: usize,
    /// Episode store paths that contributed durable records.
    pub episode_paths: Vec<PathBuf>,
    /// Efficiency JSONL path.
    pub efficiency_path: PathBuf,
    /// Cost JSONL path.
    pub costs_path: PathBuf,
    /// Provider/model outcome JSONL path.
    pub provider_model_outcomes_path: PathBuf,
    /// Cascade router JSON path.
    pub cascade_router_path: PathBuf,
    /// Gate thresholds JSON path.
    pub gate_thresholds_path: PathBuf,
    /// Executor state JSON path.
    pub executor_state_path: PathBuf,
    /// Durable knowledge JSONL path.
    pub knowledge_path: PathBuf,
    /// Durable runner event JSONL path.
    pub runner_events_path: PathBuf,
    /// Legacy signal JSONL path.
    pub signal_log_path: PathBuf,
}

impl RuntimeProjectionSet {
    /// Load the canonical runtime projection inputs once for a request.
    pub async fn load(state: &AppState) -> Result<Self, ApiError> {
        if !state
            .state_hub
            .current_snapshot_with_provenance()
            .provenance
            .bootstrapped
        {
            let _ = state.state_hub.bootstrap_from_workdir(&state.workdir);
        }
        // A recovered-only hub may advance to a newer atomic disk generation.
        // Once live events exist, retain the captured baseline rather than
        // joining live state to unrelated newer raw files.
        let _ = state
            .state_hub
            .refresh_recovered_from_workdir(&state.workdir);
        let captured = state.state_hub.cursor_snapshot();
        let recovered = captured.provenance.recovered && !captured.provenance.live_events_applied;
        let runner = captured.provenance.runner.clone();
        let source_status = captured.provenance.source_status.clone();
        let source_error = captured.provenance.source_error.clone();

        Ok(Self {
            snapshot: captured.snapshot,
            cursor: captured.next_seq,
            ring_len: captured.ring_len,
            computed_at: Utc::now(),
            recovered,
            feedback: RuntimeFeedbackProjection::load(
                &state.workdir,
                runner.as_ref(),
                source_status.as_deref(),
                source_error.as_deref(),
            )
            .await?,
            provider_health: state
                .provider_health
                .snapshot()
                .iter()
                .map(provider_status_value)
                .collect(),
            statehub_projections: state.state_hub.projections(),
            lens_runtimes: state.state_hub.lens_runtime_snapshots(),
        })
    }

    /// Wrap one projection body with stable metadata and evidence.
    pub fn state_frame(&self, name: &str, state: Value) -> Value {
        let canonical_name = canonical_projection_name(name);
        let cursor = format!("0x{:x}", self.cursor);
        let version = projection_version(canonical_name).unwrap_or(1);
        json!({
            "name": name,
            "canonical_name": canonical_name,
            "version": version,
            "channel": format!("projection:{name}"),
            "cursor": cursor,
            "computed_at": self.computed_at.to_rfc3339(),
            "recovered": self.recovered,
            "freshness": {
                "state": if self.recovered {
                    "recovered"
                } else if self.feedback.runner_projection_status == "invalid" {
                    "invalid"
                } else if self.feedback.runner_projection_status == "missing" {
                    "missing"
                } else {
                    "live"
                },
                "cursor": cursor,
            },
            "evidence": self.evidence(),
            "state": state.clone(),
            "data": state,
        })
    }

    /// Wrap a concrete surface projection in the stable typed envelope.
    pub fn envelope<T>(&self, name: &str, data: T) -> ProjectionEnvelope<T> {
        ProjectionEnvelope {
            name: name.to_string(),
            version: projection_version(name).unwrap_or(1),
            cursor: self.cursor,
            computed_at: self.computed_at.to_rfc3339(),
            recovered: self.recovered,
            data,
        }
    }

    /// Build the Workbench view from live dashboard state and typed Lens data.
    pub fn workbench_surface(&self) -> WorkbenchProjection {
        let vitality = self.agent_vitality();
        let vitality_by_agent = vitality
            .agents
            .iter()
            .map(|agent| (agent.name.as_str(), agent.vitality))
            .collect::<HashMap<_, _>>();
        let mut agents = self.snapshot.agents.values().collect::<Vec<_>>();
        agents.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        let slots = agents
            .into_iter()
            .enumerate()
            .map(|(slot_index, agent)| SlotState {
                agent_id: agent.agent_id.clone(),
                slot_index,
                occupied: agent.active,
                current_task: (!agent.current_task.is_empty()).then(|| agent.current_task.clone()),
                vitality: vitality_by_agent
                    .get(agent.agent_id.as_str())
                    .copied()
                    .unwrap_or(if agent.active { 1.0 } else { 0.0 }),
            })
            .collect();

        WorkbenchProjection {
            flows: self.flow_summaries(),
            slots,
            gate_pipeline: self.gate_pipeline(),
            cost_meter: self.cost_meter(),
        }
    }

    /// Build the Inbox view from its materialized lifecycle and health context.
    pub fn inbox_surface(&self) -> InboxProjection {
        let mut items = self
            .snapshot
            .inbox_items
            .values()
            .map(|item| InboxItem {
                id: item.item_id.clone(),
                urgency: item.urgency,
                category: item.category,
                summary: item.summary.clone(),
                detail: json!({ "defer_until": item.defer_until }),
                source_agent: None,
                timestamp: timestamp_from_millis(item.received_at_ms),
                deadline: None,
                blocking_run: None,
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            inbox_urgency_rank(right.urgency)
                .cmp(&inbox_urgency_rank(left.urgency))
                .then_with(|| right.timestamp.cmp(&left.timestamp))
        });

        InboxProjection {
            pending_count: items.len(),
            items,
            agent_vitality: self.agent_vitality(),
            cohort_health: self.cohort_health(),
        }
    }

    /// Build the Canvas live overlay from current Graph/Task state.
    pub fn canvas_surface(&self) -> CanvasProjection {
        let mut graph_names = self
            .snapshot
            .plans
            .values()
            .map(|plan| plan.plan_id.clone())
            .collect::<Vec<_>>();
        graph_names.sort();
        graph_names.dedup();
        CanvasProjection {
            graph_names,
            graph_source: "dashboard_plan_ids".to_string(),
            active_tasks: self.active_tasks(),
            gate_pipeline: self.gate_pipeline(),
        }
    }

    /// Build the Minimap from live agents and collective-intelligence data.
    pub fn minimap_surface(&self) -> MinimapProjection {
        let vitality = self.agent_vitality();
        let vitality_by_agent = vitality
            .agents
            .iter()
            .map(|agent| (agent.name.as_str(), agent.vitality))
            .collect::<HashMap<_, _>>();
        let mut agents = self.snapshot.agents.values().collect::<Vec<_>>();
        agents.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
        let positions = agents
            .into_iter()
            .enumerate()
            .map(|(index, agent)| AgentPosition {
                id: agent.agent_id.clone(),
                x: (index % 8) as f64,
                y: (index / 8) as f64,
                status: if agent.active { "active" } else { "inactive" }.to_string(),
                vitality: vitality_by_agent
                    .get(agent.agent_id.as_str())
                    .copied()
                    .unwrap_or(if agent.active { 1.0 } else { 0.0 }),
                profile: agent.role.clone(),
                current_task: (!agent.current_task.is_empty()).then(|| agent.current_task.clone()),
            })
            .collect();

        MinimapProjection {
            agents: positions,
            position_source: "deterministic_layout".to_string(),
            c_factor: self.c_factor_summary(),
            cohort_health: self.cohort_health(),
            knowledge_health: self.knowledge_health(),
        }
    }

    /// Build the Autonomy view for every currently known agent.
    pub fn autonomy_surface(&self) -> AutonomyProjection {
        let vitality = self.agent_vitality();
        AutonomyProjection {
            configs: Vec::new(),
            config_source: "unavailable".to_string(),
            agent_vitality: vitality,
            c_factor: self.c_factor_summary(),
        }
    }

    fn typed_lens<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        self.statehub_projections
            .get(name)
            .and_then(|projection| serde_json::from_value(projection.data.clone()).ok())
    }

    fn active_tasks(&self) -> ActiveTasksProjection {
        self.typed_lens("active_tasks").unwrap_or_else(|| {
            let tasks = self
                .snapshot
                .tasks
                .values()
                .map(|task| TaskSnapshot {
                    id: task.task_id.clone(),
                    title: task.title.clone(),
                    status: task.phase.clone(),
                    agent: self
                        .snapshot
                        .agents
                        .values()
                        .find(|agent| agent.current_task == task.task_id)
                        .map(|agent| agent.agent_id.clone()),
                    started_at: None,
                    duration_ms: None,
                })
                .collect();
            ActiveTasksProjection {
                tasks,
                queued: 0,
                completed_last_hour: self.snapshot.stats.tasks_completed,
                avg_task_duration_ms: 0,
            }
        })
    }

    fn agent_vitality(&self) -> AgentVitalityProjection {
        self.typed_lens("agent_vitality").unwrap_or_else(|| {
            let mut agents = self
                .snapshot
                .agents
                .values()
                .map(|agent| AgentVitalitySnapshot {
                    name: agent.agent_id.clone(),
                    vitality: if agent.active { 1.0 } else { 0.0 },
                    phase: if agent.active { "active" } else { "inactive" }.to_string(),
                    regime: agent.role.clone(),
                    slots_active: usize::from(agent.active),
                    slots_total: 1,
                    tasks_completed: 0,
                    current_task: (!agent.current_task.is_empty())
                        .then(|| agent.current_task.clone()),
                })
                .collect::<Vec<_>>();
            agents.sort_by(|left, right| left.name.cmp(&right.name));
            AgentVitalityProjection { agents }
        })
    }

    fn gate_pipeline(&self) -> GatePipelineProjection {
        self.typed_lens("gate_pipeline").unwrap_or_else(|| {
            let mut by_gate = BTreeMap::<String, (u64, u64)>::new();
            for gate in &self.snapshot.gates {
                let counts = by_gate.entry(gate.gate.clone()).or_default();
                if gate.passed {
                    counts.0 += 1;
                } else {
                    counts.1 += 1;
                }
            }
            let rungs = by_gate
                .into_iter()
                .map(|(name, (pass_count, fail_count))| {
                    let total = pass_count + fail_count;
                    RungSnapshot {
                        name,
                        pass_count,
                        fail_count,
                        pass_rate: if total == 0 {
                            0.0
                        } else {
                            pass_count as f64 / total as f64
                        },
                    }
                })
                .collect::<Vec<_>>();
            let total = self.snapshot.stats.gates_passed + self.snapshot.stats.gates_failed;
            GatePipelineProjection {
                rungs,
                overall_pass_rate: if total == 0 {
                    0.0
                } else {
                    self.snapshot.stats.gates_passed as f64 / total as f64
                },
                avg_reward: 0.0,
                hard_criteria_fail_rate: if total == 0 {
                    0.0
                } else {
                    self.snapshot.stats.gates_failed as f64 / total as f64
                },
            }
        })
    }

    fn cost_meter(&self) -> CostMeterProjection {
        self.typed_lens("cost_meter")
            .unwrap_or_else(|| CostMeterProjection {
                total_usd: self.snapshot.stats.cost_usd_total,
                ..CostMeterProjection::default()
            })
    }

    fn cohort_health(&self) -> CohortHealthProjection {
        self.typed_lens("cohort_health")
            .unwrap_or_else(|| CohortHealthProjection {
                agent_count: self.snapshot.agents.len(),
                active_count: self.snapshot.stats.agents_active,
                total_spend_usd: self.snapshot.stats.cost_usd_total,
                avg_vitality: if self.snapshot.agents.is_empty() {
                    0.0
                } else {
                    self.snapshot.stats.agents_active as f64 / self.snapshot.agents.len() as f64
                },
                ..CohortHealthProjection::default()
            })
    }

    fn knowledge_health(&self) -> KnowledgeHealthProjection {
        self.typed_lens("knowledge_health").unwrap_or_else(|| {
            let mut tier_distribution = BTreeMap::new();
            for entry in &self.snapshot.knowledge_entries {
                *tier_distribution.entry(entry.tier.clone()).or_insert(0) += 1;
            }
            KnowledgeHealthProjection {
                total_entries: self.snapshot.knowledge_entries.len() as u64,
                tier_distribution,
                ..KnowledgeHealthProjection::default()
            }
        })
    }

    fn c_factor_summary(&self) -> CFactorSummary {
        let projection = self
            .typed_lens::<CFactorProjection>("c_factor")
            .unwrap_or_else(|| CFactorProjection {
                c_factor: self
                    .snapshot
                    .cfactor_trend
                    .last()
                    .map_or(0.0, |bucket| bucket.avg),
                ..CFactorProjection::default()
            });
        CFactorSummary {
            overall: projection.c_factor,
            turn_taking_entropy: component(&projection.components, "turn_taking_entropy"),
            peer_prediction_accuracy: component(&projection.components, "peer_prediction_accuracy"),
            citation_reciprocity: component(&projection.components, "citation_reciprocity"),
            hdc_diversity: projection
                .components
                .get("hdc_diversity")
                .copied()
                .unwrap_or(projection.agent_diversity),
        }
    }

    fn flow_summaries(&self) -> Vec<FlowSummary> {
        let now_ms = self.computed_at.timestamp_millis().max(0) as u64;
        let mut flows = self
            .snapshot
            .plans
            .values()
            .map(|plan| {
                let tasks = self
                    .snapshot
                    .tasks
                    .values()
                    .filter(|task| task.plan_id == plan.plan_id)
                    .collect::<Vec<_>>();
                let total = plan.tasks_total.max(tasks.len());
                let done = plan.tasks_done + plan.tasks_failed;
                let agents = self
                    .snapshot
                    .agents
                    .values()
                    .filter(|agent| agent.current_plan == plan.plan_id)
                    .collect::<Vec<_>>();
                let started_at_ms = agents
                    .iter()
                    .map(|agent| agent.spawned_at_ms)
                    .filter(|timestamp| *timestamp > 0)
                    .min()
                    .unwrap_or(now_ms);
                let active_nodes = tasks
                    .iter()
                    .filter(|task| task.outcome.is_none())
                    .map(|task| task.task_id.clone())
                    .collect();
                let status = if plan.phase.eq_ignore_ascii_case("cancelled") {
                    FlowStatus::Cancelled
                } else if plan.active && plan.phase.eq_ignore_ascii_case("paused") {
                    FlowStatus::Paused
                } else if plan.active && plan.phase.to_ascii_lowercase().contains("human") {
                    FlowStatus::WaitingHuman
                } else if plan.active {
                    FlowStatus::Running
                } else if plan.tasks_failed > 0 || plan.phase.eq_ignore_ascii_case("failed") {
                    FlowStatus::Failed {
                        error: format!("{} task(s) failed", plan.tasks_failed.max(1)),
                    }
                } else {
                    FlowStatus::Completed {
                        verdict: "success".to_string(),
                    }
                };
                FlowSummary {
                    run_id: plan.plan_id.clone(),
                    graph_name: plan.plan_id.clone(),
                    progress_pct: if total == 0 {
                        0.0
                    } else {
                        (done as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                    },
                    cost_usd: agents.iter().map(|agent| agent.cost_usd).sum(),
                    elapsed: Duration::from_millis(now_ms.saturating_sub(started_at_ms)),
                    status,
                    active_nodes,
                    pending_human: None,
                }
            })
            .collect::<Vec<_>>();
        flows.sort_by(|left, right| left.run_id.cmp(&right.run_id));
        flows
    }

    /// Build a projection body by name from the canonical loaded inputs.
    pub fn project(&self, name: &str, query: &ProjectionQuery) -> Result<Value, ApiError> {
        let canonical = canonical_projection_name(name);
        if let Some(projection) = self.statehub_projections.get(canonical) {
            return Ok(projection.data.clone());
        }

        match canonical {
            "workbench" => serde_json::to_value(self.workbench_surface())
                .map_err(|error| ApiError::internal(format!("serialize workbench: {error}"))),
            "inbox" => serde_json::to_value(self.inbox_surface())
                .map_err(|error| ApiError::internal(format!("serialize inbox: {error}"))),
            "canvas" => serde_json::to_value(self.canvas_surface())
                .map_err(|error| ApiError::internal(format!("serialize canvas: {error}"))),
            "minimap" => serde_json::to_value(self.minimap_surface())
                .map_err(|error| ApiError::internal(format!("serialize minimap: {error}"))),
            "autonomy" => serde_json::to_value(self.autonomy_surface())
                .map_err(|error| ApiError::internal(format!("serialize autonomy: {error}"))),
            "dashboard" => Ok(json!(self.snapshot)),
            "agent_state" => Ok(self.agent_state(query)),
            "plan_state" => Ok(self.plan_state(query)),
            "gate_state" => Ok(self.gate_state(query)),
            "learning_policy_state" => Ok(self.learning_policy_state(query)),
            "cohort_health" => serialize_default_projection::<CohortHealthProjection>(),
            "active_tasks" => serialize_default_projection::<ActiveTasksProjection>(),
            "gate_pipeline" => serialize_default_projection::<GatePipelineProjection>(),
            "alerts" => Ok(json!({
                "diagnoses": self.snapshot.diagnoses,
                "recent_failures": self.snapshot.gate_recent_failures,
                "errors": self.snapshot.errors,
                "retry_state": self.retry_state(query),
                "stats": self.snapshot.stats,
            })),
            "recent_episodes" => Ok(self.recent_episodes(query)),
            "event_log" => {
                let items = if self.feedback.runner_events.is_empty() {
                    self.snapshot
                        .event_log
                        .iter()
                        .filter(|entry| event_log_entry_matches_filter(entry, query))
                        .rev()
                        .take(query.limit.unwrap_or(200))
                        .cloned()
                        .map(|entry| serde_json::to_value(entry).unwrap_or(Value::Null))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                } else {
                    self.feedback
                        .runner_events
                        .iter()
                        .filter(|entry| durable_runner_event_matches_filter(entry, query))
                        .rev()
                        .take(query.limit.unwrap_or(200))
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                };
                Ok(json!({
                    "items": items,
                    "stats": {
                        "events_retained": self.snapshot.event_log.len(),
                        "durable_events": self.feedback.runner_events.len(),
                        "errors_total": self.snapshot.stats.errors_total,
                    },
                }))
            }
            "task_outputs" => Ok(json!({
                "items": self
                    .snapshot
                    .task_outputs
                    .iter()
                    .filter(|(task_id, _)| task_id_matches_filter(task_id, query))
                    .take(query.limit.unwrap_or(usize::MAX))
                    .map(|(task_id, lines)| json!({
                        "task_id": task_id,
                        "lines": lines.iter().cloned().collect::<Vec<_>>(),
                    }))
                    .collect::<Vec<_>>(),
            })),
            "cost_meter" => serialize_default_projection::<CostMeterProjection>(),
            "knowledge_health" => serialize_default_projection::<KnowledgeHealthProjection>(),
            "c_factor" => serialize_default_projection::<CFactorProjection>(),
            "agent_vitality" => serialize_default_projection::<AgentVitalityProjection>(),
            "cost_state" => Ok(self.cost_state(query)),
            "provider_state" => Ok(self.provider_state(query)),
            "retry_state" => Ok(self.retry_state(query)),
            "execution_trace" => Ok(self.execution_trace(query)?),
            "runtime_feedback" => Ok(self.runtime_feedback(query)),
            "executor_state" => Ok(self.executor_state(query)),
            "marketplace_jobs" => Ok(json!({
                "source": "state_hub",
                "items": self.snapshot.marketplace_jobs,
                "total": self.snapshot.marketplace_jobs.len(),
            })),
            "prds" => Ok(json!({
                "source": "state_hub",
                "items": self.snapshot.atelier_prds,
                "tasks": self.snapshot.atelier_tasks,
                "total": self.snapshot.atelier_prds.len(),
            })),
            "knowledge" => Ok(json!({
                "source": "state_hub_and_neuro_store",
                "items": self.snapshot.knowledge_entries,
                "state_hub_total": self.snapshot.knowledge_entries.len(),
                "durable_total": self.feedback.knowledge_entries,
                "durable_path": self.feedback.knowledge_path.display().to_string(),
            })),
            "telemetry" => Ok(self.telemetry_state(query)),
            other => Err(ApiError::not_found(format!("unknown projection '{other}'"))),
        }
    }

    /// Return source paths, counts, StateHub cursor, and recovery metadata.
    pub fn evidence(&self) -> Value {
        json!({
            "state_hub": {
                "cursor": format!("0x{:x}", self.cursor),
                "events_retained": self.ring_len,
                "snapshot_recovered_from_disk": self.recovered,
            },
            "runtime_feedback": {
                "episode_paths": path_list(&self.feedback.episode_paths),
                "episodes": self.feedback.episodes.len(),
                "efficiency": {
                    "path": self.feedback.efficiency_path.display().to_string(),
                    "records": self.feedback.efficiency_events.len(),
                },
                "costs": {
                    "path": self.feedback.costs_path.display().to_string(),
                    "records": self.feedback.cost_records.len(),
                },
                "provider_model_outcomes": {
                    "path": self.feedback.provider_model_outcomes_path.display().to_string(),
                    "records": self.feedback.provider_model_outcomes.len(),
                },
                "runner_events": {
                    "path": self.feedback.runner_events_path.display().to_string(),
                    "records": self.feedback.runner_events.len(),
                },
                "cascade_router": json_source_state(&self.feedback.cascade_router_path, self.feedback.cascade_router_json.is_some()),
                "gate_thresholds": json_source_state(&self.feedback.gate_thresholds_path, self.feedback.gate_thresholds_json.is_some()),
                "executor_state": runner_json_source_state(
                    &self.feedback.executor_state_path,
                    self.feedback.executor_state_json.is_some(),
                    self.feedback.runner_projection_source,
                    &self.feedback.runner_projection_status,
                    self.feedback.runner_projection_generation.as_deref(),
                    self.feedback.runner_projection_error.as_deref(),
                ),
                "knowledge": {
                    "path": self.feedback.knowledge_path.display().to_string(),
                    "records": self.feedback.knowledge_entries,
                },
            },
        })
    }

    /// Aggregate gate proof rows into per-gate and per-rung summaries.
    pub fn gate_summary(&self, query: &ProjectionQuery) -> Value {
        summarize_gate_evidence(&self.gate_evidence(query))
    }

    /// Return recent gate proof rows with source metadata.
    pub fn gate_history(&self, query: &ProjectionQuery) -> Value {
        let mut history = self.gate_evidence(query);
        let total = history.len();
        truncate_values(&mut history, query.limit.unwrap_or(100));
        json!({
            "sources": self.gate_sources(),
            "gate": query.gate,
            "limit": query.limit.unwrap_or(100),
            "total": total,
            "history": history,
        })
    }

    /// Return normalized episode proof rows.
    pub fn episode_items(&self, query: &ProjectionQuery) -> Vec<Value> {
        self.episode_evidence(query)
    }

    /// Borrow durable efficiency events for metric endpoints.
    pub fn efficiency_events(&self) -> &[AgentEfficiencyEvent] {
        &self.feedback.efficiency_events
    }

    /// Borrow durable episode records for metric endpoints.
    pub fn episodes(&self) -> &[Episode] {
        &self.feedback.episodes
    }

    fn agent_state(&self, query: &ProjectionQuery) -> Value {
        json!({
            "items": self
                .snapshot
                .agents
                .values()
                .filter(|agent| agent_matches_filter(agent, query))
                .take(query.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>(),
            "provider_health": self.provider_health,
            "provider_summary": self.provider_summary(query),
            "stats": {
                "active": self.snapshot.stats.agents_active,
                "total_known": self.snapshot.agents.len(),
                "cost_usd_total": self.snapshot.stats.cost_usd_total,
            },
            "availability": collection_availability(self.snapshot.agents.is_empty(), "no_agent_state"),
        })
    }

    fn plan_state(&self, query: &ProjectionQuery) -> Value {
        json!({
            "plans": self
                .snapshot
                .plans
                .values()
                .filter(|plan| plan_matches_filter(plan.plan_id.as_str(), query))
                .take(query.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>(),
            "tasks": self
                .snapshot
                .tasks
                .values()
                .filter(|task| task_matches_filter(task, query))
                .take(query.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>(),
            "observed_tasks": self.observed_task_evidence(query),
            "stats": {
                "plans_active": self.snapshot.stats.plans_active,
                "plans_completed": self.snapshot.stats.plans_completed,
                "plans_failed": self.snapshot.stats.plans_failed,
                "tasks_active": self.snapshot.stats.tasks_active,
                "tasks_completed": self.snapshot.stats.tasks_completed,
                "tasks_failed": self.snapshot.stats.tasks_failed,
            },
            "availability": collection_availability(
                self.snapshot.plans.is_empty() && self.snapshot.tasks.is_empty(),
                "no_plan_state",
            ),
        })
    }

    fn gate_state(&self, query: &ProjectionQuery) -> Value {
        let evidence = self.gate_evidence(query);
        json!({
            "gates": self
                .snapshot
                .gates
                .iter()
                .filter(|gate| gate_matches_filter(gate, query))
                .take(query.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>(),
            "evidence": evidence,
            "summary": summarize_gate_evidence(&evidence),
            "trends": self.snapshot.gate_trends,
            "recent_failures": self.snapshot.gate_recent_failures,
            "thresholds": self.gate_threshold_state(),
            "stats": {
                "passed": self.snapshot.stats.gates_passed,
                "failed": self.snapshot.stats.gates_failed,
            },
            "availability": collection_availability(self.snapshot.gates.is_empty() && evidence.is_empty(), "no_gate_state"),
        })
    }

    fn learning_policy_state(&self, query: &ProjectionQuery) -> Value {
        json!({
            "experiment_winners": {
                "state": if self.snapshot.experiment_winners.is_empty() { "empty" } else { "available" },
                "items": self.snapshot.experiment_winners,
            },
            "cascade_router": self.cascade_router_state(),
            "gate_thresholds": self.gate_threshold_state(),
            "efficiency_trend": {
                "state": if self.snapshot.efficiency_trend.is_empty() { "empty" } else { "available" },
                "items": self.snapshot.efficiency_trend,
            },
            "cfactor_trend": {
                "state": if self.snapshot.cfactor_trend.is_empty() { "empty" } else { "available" },
                "items": self.snapshot.cfactor_trend,
            },
            "episodes": self.recent_episodes(query),
            "providers": self.provider_state(query),
            "costs": self.cost_state(query),
            "retries": self.retry_state(query),
            "policy_updates": {
                "state": "unavailable_in_statehub",
                "reason": "policy update candidates are persisted in the learning store and exposed through stable projection endpoints instead of private prompt/context internals",
                "endpoint": "/api/projections/runtime_feedback",
            },
            "stats": {
                "episodes_total": self.snapshot.stats.episodes_total.max(self.feedback.episodes.len()),
                "cost_usd_total": self.snapshot.stats.cost_usd_total,
            },
        })
    }

    fn recent_episodes(&self, query: &ProjectionQuery) -> Value {
        let items = self.episode_evidence(query);
        json!({
            "items": items,
            "dashboard_summaries": self
                .snapshot
                .episodes
                .iter()
                .filter(|ep| episode_summary_matches_filter(ep, query))
                .take(query.limit.unwrap_or(usize::MAX))
                .cloned()
                .collect::<Vec<_>>(),
            "stats": {
                "episodes_total": self.snapshot.stats.episodes_total.max(self.feedback.episodes.len()),
                "durable_episode_records": self.feedback.episodes.len(),
            },
        })
    }

    fn cost_state(&self, query: &ProjectionQuery) -> Value {
        let records = self.cost_evidence(query);
        let summary = summarize_cost_evidence(&records, self.snapshot.stats.cost_usd_total);
        json!({
            "total_cost_usd": summary.get("total_cost_usd").cloned().unwrap_or(Value::Null),
            "summary": summary,
            "data_quality": summarize_cost_data_quality(&records),
            "records": limited_values(records, query.limit.unwrap_or(250)),
            "agents": self
                .snapshot
                .agents
                .values()
                .filter(|agent| agent_matches_filter(agent, query))
                .map(|agent| json!({
                    "agent_id": agent.agent_id,
                    "role": agent.role,
                    "model": agent.model,
                    "current_plan": agent.current_plan,
                    "current_task": agent.current_task,
                    "input_tokens": agent.input_tokens,
                    "output_tokens": agent.output_tokens,
                    "cost_usd": agent.cost_usd,
                    "active": agent.active,
                    "source": "state_hub",
                }))
                .take(query.limit.unwrap_or(usize::MAX))
                .collect::<Vec<_>>(),
            "statehub_total_cost_usd": self.snapshot.stats.cost_usd_total,
            "durable_cost_records": self.feedback.cost_records.len(),
            "efficiency_records": self.feedback.efficiency_events.len(),
            "source_evidence": self.evidence(),
            "stats": self.snapshot.stats,
        })
    }

    fn provider_state(&self, query: &ProjectionQuery) -> Value {
        let outcomes = self.provider_outcome_evidence(query);
        json!({
            "providers": self.provider_health,
            "outcomes": limited_values(outcomes.clone(), query.limit.unwrap_or(250)),
            "summary": summarize_provider_outcomes(&outcomes),
            "sources": {
                "provider_model_outcomes": {
                    "path": self.feedback.provider_model_outcomes_path.display().to_string(),
                    "records": self.feedback.provider_model_outcomes.len(),
                },
                "efficiency": {
                    "path": self.feedback.efficiency_path.display().to_string(),
                    "records": self.feedback.efficiency_events.len(),
                },
                "episodes": {
                    "paths": path_list(&self.feedback.episode_paths),
                    "records": self.feedback.episodes.len(),
                },
            },
            "source_evidence": self.evidence(),
        })
    }

    fn retry_state(&self, query: &ProjectionQuery) -> Value {
        let attempts = self.retry_evidence(query);
        json!({
            "attempts": limited_values(attempts.clone(), query.limit.unwrap_or(250)),
            "summary": summarize_retry_evidence(&attempts),
            "sources": {
                "episodes": self.feedback.episodes.len(),
                "efficiency": self.feedback.efficiency_events.len(),
                "provider_model_outcomes": self.feedback.provider_model_outcomes.len(),
                "gate_failures": self.snapshot.gate_recent_failures.len(),
            },
            "source_evidence": self.evidence(),
        })
    }

    fn execution_trace(&self, query: &ProjectionQuery) -> Result<Value, ApiError> {
        Ok(json!({
            "query": query_summary(query),
            "plans": self.project("plan_state", query)?,
            "agents": self.project("agent_state", query)?,
            "gates": self.project("gate_state", query)?,
            "providers": self.provider_state(query),
            "retries": self.retry_state(query),
            "episodes": self.recent_episodes(query),
            "costs": self.cost_state(query),
            "events": self.project("event_log", query)?,
            "task_outputs": self.project("task_outputs", query)?,
            "proof": {
                "has_plan_or_task_state": !self.snapshot.plans.is_empty() || !self.snapshot.tasks.is_empty(),
                "has_provider_state": !self.provider_outcome_evidence(query).is_empty() || !self.provider_health.is_empty(),
                "has_gate_state": !self.gate_evidence(query).is_empty(),
                "has_retry_state": !self.retry_evidence(query).is_empty(),
                "has_episode_state": !self.episode_evidence(query).is_empty(),
                "has_cost_state": !self.cost_evidence(query).is_empty() || self.snapshot.stats.cost_usd_total > 0.0,
            },
        }))
    }

    fn runtime_feedback(&self, query: &ProjectionQuery) -> Value {
        json!({
            "evidence": self.evidence(),
            "episodes": self.recent_episodes(query),
            "efficiency_events": {
                "items": limited_values(
                    self.feedback
                        .efficiency_events
                        .iter()
                        .filter(|event| efficiency_event_matches_filter(event, query))
                        .map(|event| {
                            let mut value = serde_json::to_value(event).unwrap_or(Value::Null);
                            insert_source(&mut value, "efficiency_log");
                            value
                        })
                        .collect(),
                    query.limit.unwrap_or(250),
                ),
                "total": self.feedback.efficiency_events.len(),
                "path": self.feedback.efficiency_path.display().to_string(),
            },
            "costs": self.cost_state(query),
            "providers": self.provider_state(query),
            "retries": self.retry_state(query),
            "cascade_router": self.cascade_router_state(),
            "gate_thresholds": self.gate_threshold_state(),
            "executor_state": self.executor_state(query),
        })
    }

    fn executor_state(&self, query: &ProjectionQuery) -> Value {
        json!({
            "source": runner_json_source_state(
                &self.feedback.executor_state_path,
                self.feedback.executor_state_json.is_some(),
                self.feedback.runner_projection_source,
                &self.feedback.runner_projection_status,
                self.feedback.runner_projection_generation.as_deref(),
                self.feedback.runner_projection_error.as_deref(),
            ),
            "raw": self.feedback.executor_state_json,
            "plans": self
                .snapshot
                .plans
                .values()
                .filter(|plan| plan_matches_filter(plan.plan_id.as_str(), query))
                .cloned()
                .collect::<Vec<_>>(),
            "tasks": self
                .snapshot
                .tasks
                .values()
                .filter(|task| task_matches_filter(task, query))
                .cloned()
                .collect::<Vec<_>>(),
            "agents": self
                .snapshot
                .agents
                .values()
                .filter(|agent| agent_matches_filter(agent, query))
                .cloned()
                .collect::<Vec<_>>(),
            "gates": self.gate_evidence(query),
            "stats": self.snapshot.stats,
        })
    }

    /// Build the telemetry projection: active watchers, circuit breaker states,
    /// observation counts, and provider health for live observability.
    fn telemetry_state(&self, _query: &ProjectionQuery) -> Value {
        // Active watchers derived from recent diagnosis summaries in the snapshot.
        // Each diagnosis subject identifies the watcher (e.g. "Circuit Breaker",
        // "Stuck Pattern", "Time Overrun").
        let active_watchers: Vec<Value> = self
            .snapshot
            .diagnoses
            .iter()
            .map(|diagnosis| {
                json!({
                    "watcher": &diagnosis.subject,
                    "last_fired": diagnosis.ts.to_rfc3339(),
                    "severity": diagnosis.severity,
                    "detail": &diagnosis.detail,
                    "suggested_action": &diagnosis.suggested_action,
                    "intervention_taken": &diagnosis.intervention_taken,
                })
            })
            .collect();

        // Circuit breaker states from provider health. Each provider tracked
        // by the ProviderHealthTracker acts as a circuit breaker that opens
        // on repeated failures and probes for recovery.
        let circuit_breaker_states: Vec<Value> = self.provider_health.clone();

        // Observation counts aggregated from snapshot stats and durable feedback.
        let observation_counts = json!({
            "plans_active": self.snapshot.stats.plans_active,
            "plans_completed": self.snapshot.stats.plans_completed,
            "plans_failed": self.snapshot.stats.plans_failed,
            "tasks_active": self.snapshot.stats.tasks_active,
            "tasks_completed": self.snapshot.stats.tasks_completed,
            "tasks_failed": self.snapshot.stats.tasks_failed,
            "agents_active": self.snapshot.stats.agents_active,
            "gates_passed": self.snapshot.stats.gates_passed,
            "gates_failed": self.snapshot.stats.gates_failed,
            "episodes_total": self.snapshot.stats.episodes_total.max(self.feedback.episodes.len()),
            "errors_total": self.snapshot.stats.errors_total,
            "efficiency_events": self.feedback.efficiency_events.len(),
            "diagnoses": self.snapshot.diagnoses.len(),
            "lens_queue_enqueued": self.lens_runtimes.iter().map(|runtime| runtime.queue.enqueued).sum::<u64>(),
            "lens_queue_processed": self.lens_runtimes.iter().map(|runtime| runtime.queue.processed).sum::<u64>(),
            "lens_queue_dropped_oldest": self.lens_runtimes.iter().map(|runtime| runtime.queue.dropped_oldest).sum::<u64>(),
            "lens_queue_failed_dispatches": self.lens_runtimes.iter().map(|runtime| runtime.queue.failed_dispatches).sum::<u64>(),
        });

        json!({
            "active_watchers": active_watchers,
            "circuit_breaker_states": circuit_breaker_states,
            "observation_counts": observation_counts,
            "provider_health": self.provider_health,
            "lens_runtimes": self.lens_runtimes,
            "stats": self.snapshot.stats,
        })
    }

    fn cascade_router_state(&self) -> Value {
        match self.feedback.cascade_router_json.clone() {
            Some(value) => {
                let data_quality = cascade_router_data_quality(Some(&value));
                json!({
                    "state": "available",
                    "source": self.feedback.cascade_router_path.display().to_string(),
                    "value": value,
                    "data_quality": data_quality,
                })
            }
            None if !self.snapshot.cascade_router_json.trim().is_empty() => {
                let mut node = json_blob_state(
                    self.snapshot.cascade_router_json.as_str(),
                    "cascade_router_not_loaded",
                    "/api/learning/cascade-router",
                );
                if let Some(object) = node.as_object_mut() {
                    let data_quality = cascade_router_data_quality(object.get("value"));
                    object.insert(
                        "data_quality".to_string(),
                        serde_json::to_value(data_quality).unwrap_or(Value::Null),
                    );
                }
                node
            }
            None => json!({
                "state": "missing",
                "reason": "cascade_router_not_loaded",
                "endpoint": "/api/learning/cascade-router",
                "source": self.feedback.cascade_router_path.display().to_string(),
                "data_quality": DataQuality::default(),
            }),
        }
    }

    fn gate_threshold_state(&self) -> Value {
        match self.feedback.gate_thresholds_json.clone() {
            Some(value) => json!({
                "state": "available",
                "source": self.feedback.gate_thresholds_path.display().to_string(),
                "value": value,
            }),
            None if !self.snapshot.gate_thresholds_json.trim().is_empty() => json_blob_state(
                self.snapshot.gate_thresholds_json.as_str(),
                "gate_thresholds_not_loaded",
                "/api/learning/gate-thresholds",
            ),
            None => json!({
                "state": "missing",
                "reason": "gate_thresholds_not_loaded",
                "endpoint": "/api/learning/gate-thresholds",
                "source": self.feedback.gate_thresholds_path.display().to_string(),
            }),
        }
    }

    fn gate_sources(&self) -> Vec<String> {
        let mut sources = vec!["state_hub".to_string()];
        sources.extend(path_list(&self.feedback.episode_paths));
        sources.push(self.feedback.signal_log_path.display().to_string());
        sources.push(self.feedback.runner_events_path.display().to_string());
        sources.push(
            self.feedback
                .provider_model_outcomes_path
                .display()
                .to_string(),
        );
        sources.push(self.feedback.efficiency_path.display().to_string());
        sources
    }

    fn observed_task_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let mut by_task: BTreeMap<(String, String), Value> = BTreeMap::new();

        for event in &self.feedback.efficiency_events {
            if !efficiency_event_matches_filter(event, query) {
                continue;
            }
            let key = (event.plan_id.clone(), event.task_id.clone());
            by_task.entry(key).or_insert_with(|| json!({
                "plan_id": event.plan_id,
                "task_id": event.task_id,
                "agent_id": event.agent_id,
                "role": event.role,
                "provider": event.backend,
                "model": first_non_empty_owned([Some(event.model.clone()), Some(event.model_used.clone())]).unwrap_or_default(),
                "latest_timestamp": event.timestamp,
                "source": "efficiency_log",
            }));
        }

        for episode in &self.feedback.episodes {
            if !episode_matches_query(episode, query) {
                continue;
            }
            let plan_id = episode_plan_id(episode);
            let key = (plan_id.clone(), episode.task_id.clone());
            by_task.entry(key).or_insert_with(|| {
                json!({
                    "plan_id": plan_id,
                    "task_id": episode.task_id,
                    "agent_id": episode.agent_id,
                    "role": episode_role(episode),
                    "provider": episode_provider(episode),
                    "model": episode.model,
                    "latest_timestamp": episode.completed_at.to_rfc3339(),
                    "source": "episode_log",
                })
            });
        }

        limited_values(by_task.into_values().collect(), query.limit.unwrap_or(250))
    }

    fn gate_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let mut values = Vec::new();

        for gate in self
            .snapshot
            .gates
            .iter()
            .filter(|gate| gate_matches_filter(gate, query))
        {
            values.push(json!({
                "source": "state_hub",
                "plan_id": gate.plan_id,
                "task_id": gate.task_id,
                "gate": gate.gate,
                "passed": gate.passed,
                "timestamp_ms": gate.ts_millis,
            }));
        }

        for episode in self
            .feedback
            .episodes
            .iter()
            .filter(|episode| episode_matches_query(episode, query))
        {
            let plan_id = episode_plan_id(episode);
            for verdict in &episode.gate_verdicts {
                if !gate_name_matches(&verdict.gate, query) {
                    continue;
                }
                values.push(json!({
                    "source": "episode_log",
                    "plan_id": plan_id,
                    "task_id": episode.task_id,
                    "agent_id": episode.agent_id,
                    "episode_id": episode_public_id(episode),
                    "gate": verdict.gate,
                    "passed": verdict.passed,
                    "signature": verdict.signature,
                    "timestamp_ms": millis_from_datetime(episode.completed_at),
                }));
            }
        }

        for entry in &self.feedback.signal_events {
            if let Some(row) = signal_gate_evidence(entry, query) {
                values.push(row);
            }
        }

        for outcome in self
            .provider_outcome_records()
            .into_iter()
            .filter(|outcome| provider_outcome_matches_filter(outcome, query))
        {
            for gate in &outcome.gate_outcomes {
                if !gate_name_matches(&gate.gate_name, query) {
                    continue;
                }
                values.push(json!({
                    "source": "provider_model_outcomes",
                    "plan_id": outcome.run_id.clone().unwrap_or_default(),
                    "task_id": outcome.task_id,
                    "provider": outcome.provider,
                    "model": outcome.model,
                    "gate": gate.gate_name,
                    "passed": gate.passed,
                    "score": gate.score,
                    "duration_ms": gate.duration_ms,
                    "retry_count": outcome.retry_count,
                    "status": outcome_status_label(outcome.status),
                    "timestamp": outcome.timestamp,
                    "timestamp_ms": parse_timestamp_millis(&outcome.timestamp).unwrap_or_default(),
                }));
            }
        }

        for event in self
            .feedback
            .efficiency_events
            .iter()
            .filter(|event| efficiency_event_matches_filter(event, query))
        {
            if !gate_name_matches("terminal", query) {
                continue;
            }
            values.push(json!({
                "source": "efficiency_log",
                "plan_id": event.plan_id,
                "task_id": event.task_id,
                "agent_id": event.agent_id,
                "provider": event.backend,
                "model": first_non_empty_owned([Some(event.model.clone()), Some(event.model_used.clone())]).unwrap_or_default(),
                "gate": "terminal",
                "passed": event.gate_passed,
                "duration_ms": event.duration_ms.max(event.wall_time_ms),
                "iteration": event.iteration,
                "strategy_attempted": event.strategy_attempted,
                "timestamp": event.timestamp,
                "timestamp_ms": parse_timestamp_millis(&event.timestamp).unwrap_or_default(),
            }));
        }

        dedupe_and_sort_evidence(values)
    }

    fn episode_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let mut values = self
            .feedback
            .episodes
            .iter()
            .filter(|episode| episode_matches_query(episode, query))
            .map(|episode| {
                json!({
                    "source": "episode_log",
                    "id": episode.id,
                    "episode_id": episode_public_id(episode),
                    "kind": episode.kind,
                    "agent_id": episode.agent_id,
                    "role": episode_role(episode),
                    "agent_template": episode.agent_template,
                    "task_id": episode.task_id,
                    "plan_id": episode_plan_id(episode),
                    "provider": episode_provider(episode),
                    "model": episode.model,
                    "trigger_kind": episode.trigger_kind,
                    "success": episode.success,
                    "status": if episode.success { "passed" } else { "failed" },
                    "turns": episode.turns,
                    "tokens_used": episode.tokens_used,
                    "usage": episode.usage,
                    "retry_count": episode_retry_count(episode),
                    "gate_verdicts": episode.gate_verdicts,
                    "failure_reason": episode.failure_reason,
                    "reflection_present": episode.reflection.as_ref().is_some_and(|value| !value.trim().is_empty()),
                    "started_at": episode.started_at.to_rfc3339(),
                    "completed_at": episode.completed_at.to_rfc3339(),
                    "timestamp_ms": millis_from_datetime(episode.completed_at),
                    "duration_secs": episode.duration_secs,
                    "headline": episode.headline,
                    "prompt_composition_available": episode.prompt_composition.is_some(),
                })
            })
            .collect::<Vec<_>>();

        if values.is_empty() {
            values.extend(
                self.snapshot
                    .episodes
                    .iter()
                    .filter(|episode| episode_summary_matches_filter(episode, query))
                    .map(|episode| {
                        json!({
                            "source": "state_hub",
                            "episode_id": episode.episode_id,
                            "agent_id": episode.agent_id,
                            "role": episode.role,
                            "success": episode.passed,
                            "status": if episode.passed { "passed" } else { "failed" },
                            "timestamp_ms": episode.ts_millis,
                        })
                    }),
            );
        }

        values.sort_by_key(|v| std::cmp::Reverse(value_ts(v)));
        truncate_values(&mut values, query.limit.unwrap_or(250));
        values
    }

    fn cost_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let mut records = Vec::new();

        for record in &self.feedback.cost_records {
            if !cost_record_matches_filter(record, query) {
                continue;
            }
            records.push(json!({
                "source": "costs_log",
                "timestamp": record.timestamp,
                "timestamp_ms": parse_timestamp_millis(&record.timestamp).unwrap_or_default(),
                "model": record.model,
                "provider": record.provider,
                "role": record.role,
                "plan_id": record.plan_id,
                "task_id": record.task_id,
                "complexity_band": record.complexity_band,
                "input_tokens": record.input_tokens,
                "output_tokens": record.output_tokens,
                "cached_tokens": record.cached_tokens,
                "cost_usd": record.cost_usd,
                "duration_ms": record.duration_ms,
                "success": record.success,
                "session_id": record.session_id,
            }));
        }

        if records.is_empty() {
            for event in &self.feedback.efficiency_events {
                if !efficiency_event_matches_filter(event, query) {
                    continue;
                }
                records.push(json!({
                    "source": "efficiency_log",
                    "timestamp": event.timestamp,
                    "timestamp_ms": parse_timestamp_millis(&event.timestamp).unwrap_or_default(),
                    "model": first_non_empty_owned([Some(event.model.clone()), Some(event.model_used.clone())]).unwrap_or_default(),
                    "provider": event.backend,
                    "role": event.role,
                    "plan_id": event.plan_id,
                    "task_id": event.task_id,
                    "input_tokens": event.input_tokens,
                    "output_tokens": event.output_tokens,
                    "cached_tokens": event.cache_read_tokens,
                    "cost_usd": event.cost_usd,
                    "cost_usd_without_cache": event.cost_usd_without_cache,
                    "duration_ms": event.duration_ms.max(event.wall_time_ms),
                    "success": event.gate_passed,
                    "iteration": event.iteration,
                }));
            }
        }

        records.sort_by_key(|v| std::cmp::Reverse(value_ts(v)));
        records
    }

    fn provider_outcome_records(&self) -> Vec<ProviderModelOutcomeRecord> {
        if !self.feedback.provider_model_outcomes.is_empty() {
            return self.feedback.provider_model_outcomes.clone();
        }

        let mut records = Vec::new();
        for event in &self.feedback.efficiency_events {
            if let Some(record) = ProviderModelOutcomeRecord::from_efficiency_event(event) {
                records.push(record);
            }
        }
        for episode in &self.feedback.episodes {
            if let Some(record) = ProviderModelOutcomeRecord::from_episode(episode, None) {
                records.push(record);
            }
        }
        records
    }

    fn provider_outcome_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let source = if self.feedback.provider_model_outcomes.is_empty() {
            "derived_runtime_feedback"
        } else {
            "provider_model_outcomes"
        };
        let mut values = self
            .provider_outcome_records()
            .into_iter()
            .filter(|record| provider_outcome_matches_filter(record, query))
            .map(|record| {
                json!({
                    "source": source,
                    "schema_version": record.schema_version,
                    "timestamp": record.timestamp,
                    "timestamp_ms": parse_timestamp_millis(&record.timestamp).unwrap_or_default(),
                    "action_id": record.action_id,
                    "provider": record.provider,
                    "model": record.model,
                    "task_id": record.task_id,
                    "task_type": record.task_type,
                    "role_id": record.role_id,
                    "status": outcome_status_label(record.status),
                    "success": record.status.is_success(),
                    "gate_outcomes": record.gate_outcomes,
                    "retry_count": record.retry_count,
                    "usage": record.usage,
                    "run_id": record.run_id,
                })
            })
            .collect::<Vec<_>>();

        values.sort_by_key(|v| std::cmp::Reverse(value_ts(v)));
        values
    }

    fn provider_summary(&self, query: &ProjectionQuery) -> Value {
        summarize_provider_outcomes(&self.provider_outcome_evidence(query))
    }

    fn retry_evidence(&self, query: &ProjectionQuery) -> Vec<Value> {
        let mut values = Vec::new();

        for event in self
            .feedback
            .efficiency_events
            .iter()
            .filter(|event| efficiency_event_matches_filter(event, query))
        {
            let retry_like = event.iteration > 1
                || !event.strategy_attempted.trim().is_empty()
                || !event.gate_errors.is_empty()
                || event.gate_passed != Some(true);
            if !retry_like {
                continue;
            }
            values.push(json!({
                "source": "efficiency_log",
                "plan_id": event.plan_id,
                "task_id": event.task_id,
                "agent_id": event.agent_id,
                "provider": event.backend,
                "model": first_non_empty_owned([Some(event.model.clone()), Some(event.model_used.clone())]).unwrap_or_default(),
                "attempt": event.iteration.max(1),
                "retry_count": event.iteration.saturating_sub(1),
                "status": match event.gate_passed { Some(true) => "passed", Some(false) => "failed", None => "unknown" },
                "gate_passed": event.gate_passed,
                "strategy_attempted": event.strategy_attempted,
                "gate_errors": event.gate_errors,
                "cost_usd": event.cost_usd,
                "duration_ms": event.duration_ms.max(event.wall_time_ms),
                "timestamp": event.timestamp,
                "timestamp_ms": parse_timestamp_millis(&event.timestamp).unwrap_or_default(),
            }));
        }

        for outcome in self
            .provider_outcome_records()
            .into_iter()
            .filter(|outcome| provider_outcome_matches_filter(outcome, query))
        {
            if outcome.retry_count == 0
                && !matches!(outcome.status, ProviderModelOutcomeStatus::NeedsRetry)
            {
                continue;
            }
            values.push(json!({
                "source": "provider_model_outcomes",
                "plan_id": outcome.run_id.clone().unwrap_or_default(),
                "task_id": outcome.task_id,
                "provider": outcome.provider,
                "model": outcome.model,
                "attempt": outcome.retry_count.saturating_add(1),
                "retry_count": outcome.retry_count,
                "status": outcome_status_label(outcome.status),
                "gate_outcomes": outcome.gate_outcomes,
                "usage": outcome.usage,
                "timestamp": outcome.timestamp,
                "timestamp_ms": parse_timestamp_millis(&outcome.timestamp).unwrap_or_default(),
            }));
        }

        for episode in self
            .feedback
            .episodes
            .iter()
            .filter(|episode| episode_matches_query(episode, query))
        {
            let retry_count = episode_retry_count(episode);
            let has_gate_failure = episode.gate_verdicts.iter().any(|verdict| !verdict.passed);
            if retry_count == 0 && !has_gate_failure && episode.failure_reason.is_none() {
                continue;
            }
            values.push(json!({
                "source": "episode_log",
                "plan_id": episode_plan_id(episode),
                "task_id": episode.task_id,
                "agent_id": episode.agent_id,
                "provider": episode_provider(episode),
                "model": episode.model,
                "episode_id": episode_public_id(episode),
                "attempt": retry_count.saturating_add(1),
                "retry_count": retry_count,
                "status": if episode.success { "passed" } else { "failed" },
                "success": episode.success,
                "failure_reason": episode.failure_reason,
                "gate_verdicts": episode.gate_verdicts,
                "timestamp": episode.completed_at.to_rfc3339(),
                "timestamp_ms": millis_from_datetime(episode.completed_at),
            }));
        }

        values.sort_by_key(|v| std::cmp::Reverse(value_ts(v)));
        values
    }
}

fn timestamp_from_millis(timestamp_ms: u64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(timestamp_ms.min(i64::MAX as u64) as i64)
        .single()
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

const fn inbox_urgency_rank(urgency: UrgencyLevel) -> u8 {
    match urgency {
        UrgencyLevel::Notify => 1,
        UrgencyLevel::Question => 2,
        UrgencyLevel::Review => 3,
    }
}

fn component(components: &BTreeMap<String, f64>, name: &str) -> f64 {
    components
        .get(name)
        .or_else(|| components.get(&name.replace('_', "-")))
        .copied()
        .unwrap_or_default()
}

impl RuntimeFeedbackProjection {
    async fn load(
        workdir: &Path,
        runner_projection: Option<&DurableRunnerProjection>,
        source_status: Option<&str>,
        source_error: Option<&str>,
    ) -> Result<Self, ApiError> {
        let roko = workdir.join(".roko");
        let learn = roko.join("learn");
        let efficiency_path = learn.join("efficiency.jsonl");
        let costs_path = learn.join("costs.jsonl");
        let provider_model_outcomes_path = learn.join("provider-model-outcomes.jsonl");
        let cascade_router_path = learn.join("cascade-router.json");
        let standalone_gate_thresholds_path = learn.join("gate-thresholds.json");
        let canonical_executor_state_path = roko.join("state").join("state-snapshot.json");
        let executor_state_path = runner_projection.map_or_else(
            || canonical_executor_state_path.clone(),
            |projection| projection.source_path.clone(),
        );
        let gate_thresholds_path = runner_projection
            .filter(|projection| projection.gate_thresholds.is_some())
            .map_or_else(
                || standalone_gate_thresholds_path.clone(),
                |projection| projection.source_path.clone(),
            );
        let knowledge_path = roko.join("neuro").join("knowledge.jsonl");
        let runner_events_path = roko.join("events.jsonl");
        let signal_log_path = roko.join("engrams.jsonl");

        let episodes = read_project_episodes_lossy(workdir)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "runtime projection auxiliary episodes unavailable");
                Vec::new()
            });
        let efficiency_events = read_project_efficiency_events(workdir)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "runtime projection auxiliary efficiency events unavailable");
                Vec::new()
            });
        let cost_records = CostsLog::at(&costs_path)
            .read_all()
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(%error, path = %costs_path.display(), "runtime projection auxiliary costs unavailable");
                Vec::new()
            });
        let provider_model_outcomes = read_provider_model_outcomes(&provider_model_outcomes_path)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(%error, path = %provider_model_outcomes_path.display(), "runtime projection auxiliary provider outcomes unavailable");
                Vec::new()
            });
        let runner_events = read_jsonl_values(&runner_events_path).await.unwrap_or_else(|error| {
            tracing::warn!(%error, path = %runner_events_path.display(), "runtime projection auxiliary runner events unavailable");
            Vec::new()
        });
        let signal_events = read_jsonl_values(&signal_log_path).await.unwrap_or_else(|error| {
            tracing::warn!(%error, path = %signal_log_path.display(), "runtime projection auxiliary signal events unavailable");
            Vec::new()
        });

        Ok(Self {
            runner_events,
            signal_events,
            episodes,
            efficiency_events,
            cost_records,
            provider_model_outcomes,
            cascade_router_json: read_optional_json(&cascade_router_path)
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, path = %cascade_router_path.display(), "runtime projection auxiliary cascade router unavailable");
                    None
                }),
            gate_thresholds_json: match runner_projection
                .and_then(|projection| projection.gate_thresholds.as_ref())
            {
                Some(thresholds) => Some(thresholds.clone()),
                None if source_status == Some("invalid") => None,
                None => read_optional_json(&standalone_gate_thresholds_path)
                    .await
                    .unwrap_or_else(|error| {
                        tracing::warn!(%error, path = %standalone_gate_thresholds_path.display(), "runtime projection auxiliary gate thresholds unavailable");
                        None
                    }),
            },
            executor_state_json: runner_projection.map(|projection| projection.executor.clone()),
            runner_projection_source: runner_projection.map(|projection| projection.source),
            runner_projection_generation: runner_projection
                .map(|projection| projection.generation.clone()),
            runner_projection_status: source_status.unwrap_or("missing").to_string(),
            runner_projection_error: source_error.map(ToOwned::to_owned),
            knowledge_entries: count_jsonl_records(&knowledge_path)
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, path = %knowledge_path.display(), "runtime projection auxiliary knowledge unavailable");
                    0
                }),
            episode_paths: {
                let path = resolve_project_episode_path(workdir);
                path.exists().then_some(path).into_iter().collect()
            },
            efficiency_path,
            costs_path,
            provider_model_outcomes_path,
            cascade_router_path,
            gate_thresholds_path,
            executor_state_path,
            knowledge_path,
            runner_events_path,
            signal_log_path,
        })
    }
}

fn serialize_default_projection<T>() -> Result<Value, ApiError>
where
    T: Default + Serialize,
{
    serde_json::to_value(T::default())
        .map_err(|error| ApiError::internal(format!("serialize default projection: {error}")))
}

/// Canonical projection name for aliases accepted by the HTTP API.
pub fn canonical_projection_name(name: &str) -> &str {
    match name {
        "dashboard_snapshot" => "dashboard",
        "agents" | "agent_trails" => "agent_state",
        "plans" | "plans_list" => "plan_state",
        "gates" => "gate_state",
        "learning" | "learning_policy" => "learning_policy_state",
        "events" => "event_log",
        "providers" | "provider_outcomes" => "provider_state",
        "retries" => "retry_state",
        "costs" => "cost_state",
        "trace" | "proof" => "execution_trace",
        "feedback" => "runtime_feedback",
        "jobs" => "marketplace_jobs",
        "atelier" => "prds",
        "knowledge_entries" => "knowledge",
        "watchers" | "circuit_breakers" | "observations" => "telemetry",
        _ => name,
    }
}

/// Build an SSE delta frame for a projection stream.
pub fn projection_delta_frame(name: &str, cursor: u64, delta: &DashboardEvent) -> Value {
    json!({
        "type": "delta",
        "channel": format!("projection:{name}"),
        "cursor": format!("0x{cursor:x}"),
        "delta": delta,
    })
}

/// Decide whether a StateHub event should be emitted to a projection stream.
pub fn projection_accepts_event(
    name: &str,
    query: &ProjectionQuery,
    event: &DashboardEvent,
) -> bool {
    let canonical_name = canonical_projection_name(name);
    if let DashboardEvent::ProjectionUpdated { projection_id, .. } = event {
        let projection_id = canonical_projection_name(projection_id);
        return projection_id == canonical_name
            || matches!(
                (canonical_name, projection_id),
                (
                    "workbench",
                    "active_tasks" | "agent_vitality" | "gate_pipeline" | "cost_meter"
                ) | ("inbox", "agent_vitality" | "cohort_health")
                    | ("canvas", "active_tasks" | "gate_pipeline")
                    | (
                        "minimap",
                        "agent_vitality" | "c_factor" | "cohort_health" | "knowledge_health"
                    )
                    | ("autonomy", "agent_vitality" | "c_factor")
            );
    }
    match canonical_name {
        "dashboard" | "execution_trace" | "runtime_feedback" => true,
        "workbench" => matches!(
            event,
            DashboardEvent::PlanStarted { .. }
                | DashboardEvent::PlanCompleted { .. }
                | DashboardEvent::PhaseTransition { .. }
                | DashboardEvent::TaskStarted { .. }
                | DashboardEvent::TaskCompleted { .. }
                | DashboardEvent::TaskPhaseChanged { .. }
                | DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::GateResult { .. }
        ),
        "inbox" => matches!(
            event,
            DashboardEvent::InboxItemReceived { .. }
                | DashboardEvent::InboxApprove { .. }
                | DashboardEvent::InboxReject { .. }
                | DashboardEvent::InboxDefer { .. }
                | DashboardEvent::InboxDismiss { .. }
                | DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
        ),
        "canvas" => matches!(
            event,
            DashboardEvent::PlanStarted { .. }
                | DashboardEvent::PlanCompleted { .. }
                | DashboardEvent::TaskStarted { .. }
                | DashboardEvent::TaskCompleted { .. }
                | DashboardEvent::TaskPhaseChanged { .. }
                | DashboardEvent::GateResult { .. }
        ),
        "minimap" => matches!(
            event,
            DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::CFactorTrendUpdated { .. }
                | DashboardEvent::KnowledgeEntriesUpdated { .. }
        ),
        "autonomy" => matches!(
            event,
            DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::CFactorTrendUpdated { .. }
        ),
        "agent_state" => match event {
            DashboardEvent::AgentSpawned { agent_id, .. }
            | DashboardEvent::AgentOutput { agent_id, .. }
            | DashboardEvent::AgentCompleted { agent_id, .. } => {
                agent_id_matches_filter(agent_id, query)
            }
            _ => false,
        },
        "plan_state" => match event {
            DashboardEvent::PlanStarted { plan_id }
            | DashboardEvent::PlanCompleted { plan_id, .. }
            | DashboardEvent::PhaseTransition { plan_id, .. }
            | DashboardEvent::TaskStarted { plan_id, .. }
            | DashboardEvent::TaskCompleted { plan_id, .. }
            | DashboardEvent::TaskPhaseChanged { plan_id, .. } => {
                plan_id_matches_filter(plan_id, query)
            }
            _ => false,
        },
        "gate_state" => match event {
            DashboardEvent::GateResult { plan_id, .. } => plan_id_matches_filter(plan_id, query),
            DashboardEvent::GateThresholdsUpdated { .. } => true,
            _ => false,
        },
        "gate_pipeline" => matches!(event, DashboardEvent::GateResult { .. }),
        "learning_policy_state" => matches!(
            event,
            DashboardEvent::ExperimentWinnersUpdated { .. }
                | DashboardEvent::CFactorTrendUpdated { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::EpisodeRecorded { .. }
                | DashboardEvent::CascadeRouterUpdated { .. }
                | DashboardEvent::GateThresholdsUpdated { .. }
        ),
        "cohort_health" => matches!(
            event,
            DashboardEvent::PlanStarted { .. }
                | DashboardEvent::PlanCompleted { .. }
                | DashboardEvent::TaskStarted { .. }
                | DashboardEvent::TaskCompleted { .. }
                | DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::CFactorTrendUpdated { .. }
                | DashboardEvent::Diagnosis { .. }
                | DashboardEvent::Error { .. }
        ),
        "active_tasks" => match event {
            DashboardEvent::TaskStarted { plan_id, .. }
            | DashboardEvent::TaskCompleted { plan_id, .. }
            | DashboardEvent::TaskPhaseChanged { plan_id, .. } => {
                plan_id_matches_filter(plan_id, query)
            }
            _ => false,
        },
        "alerts" => matches!(
            event,
            DashboardEvent::Diagnosis { .. }
                | DashboardEvent::GateResult { passed: false, .. }
                | DashboardEvent::Error { .. }
        ),
        "recent_episodes" => match event {
            DashboardEvent::EpisodeRecorded { role, .. } => {
                episode_role_matches_filter(role, query.filter.as_deref())
            }
            _ => false,
        },
        "event_log" => matches!(event, DashboardEvent::EventLogEntry { .. }),
        "task_outputs" => matches!(event, DashboardEvent::TaskOutputAppended { .. }),
        "cost_meter" | "cost_state" | "provider_state" => matches!(
            event,
            DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::EpisodeRecorded { .. }
                | DashboardEvent::AgentCompleted { .. }
        ),
        "knowledge_health" => matches!(event, DashboardEvent::KnowledgeEntriesUpdated { .. }),
        "c_factor" => matches!(event, DashboardEvent::CFactorTrendUpdated { .. }),
        "agent_vitality" => matches!(
            event,
            DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
        ),
        "retry_state" => matches!(
            event,
            DashboardEvent::GateResult { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::EpisodeRecorded { .. }
                | DashboardEvent::Diagnosis { .. }
        ),
        "executor_state" => matches!(
            event,
            DashboardEvent::PlanStarted { .. }
                | DashboardEvent::PlanCompleted { .. }
                | DashboardEvent::TaskStarted { .. }
                | DashboardEvent::TaskCompleted { .. }
                | DashboardEvent::TaskPhaseChanged { .. }
                | DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentOutput { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::GateResult { .. }
        ),
        "marketplace_jobs" => matches!(event, DashboardEvent::MarketplaceJobsUpdated { .. }),
        "prds" => matches!(event, DashboardEvent::AtelierPrdsUpdated { .. }),
        "knowledge" => matches!(event, DashboardEvent::KnowledgeEntriesUpdated { .. }),
        "telemetry" => matches!(
            event,
            DashboardEvent::AgentSpawned { .. }
                | DashboardEvent::AgentCompleted { .. }
                | DashboardEvent::EfficiencyEvent { .. }
                | DashboardEvent::Diagnosis { .. }
                | DashboardEvent::Error { .. }
                | DashboardEvent::GateResult { .. }
        ),
        _ => false,
    }
}

fn provider_status_value(status: &ProviderStatus) -> Value {
    let (state, recovery_in_ms) = match status.state {
        HealthState::Healthy => ("healthy", None),
        HealthState::Probing => ("probing", None),
        HealthState::Unhealthy { recovery_at } => (
            "unhealthy",
            Some(
                recovery_at
                    .checked_duration_since(std::time::Instant::now())
                    .unwrap_or_default()
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64,
            ),
        ),
    };
    json!({
        "provider": status.provider,
        "state": state,
        "consecutive_failures": status.consecutive_failures,
        "last_failure_at": status.last_failure_at.map(|ts| ts.to_rfc3339()),
        "last_success_at": status.last_success_at.map(|ts| ts.to_rfc3339()),
        "total_attempts": status.total_attempts,
        "total_successes": status.total_successes,
        "recovery_in_ms": recovery_in_ms,
    })
}

async fn read_optional_json(path: &Path) -> Result<Option<Value>, ApiError> {
    let content = match read_bounded_projection_text(path).await {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ApiError::internal(format!(
                "read {}: {err}",
                path.display()
            )));
        }
    };
    serde_json::from_str::<Value>(&content)
        .map(Some)
        .map_err(|err| ApiError::internal(format!("parse {}: {err}", path.display())))
}

async fn count_jsonl_records(path: &Path) -> Result<usize, ApiError> {
    let content = match read_bounded_projection_text(path).await {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(err) => {
            return Err(ApiError::internal(format!(
                "read {}: {err}",
                path.display()
            )));
        }
    };
    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count())
}

async fn read_jsonl_values(path: &Path) -> Result<Vec<Value>, ApiError> {
    let content = match read_bounded_projection_text(path).await {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(ApiError::internal(format!(
                "read {}: {err}",
                path.display()
            )));
        }
    };

    let mut values = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(line).map_err(|err| {
            ApiError::internal(format!(
                "parse {} line {}: {err}",
                path.display(),
                idx.saturating_add(1)
            ))
        })?;
        values.push(value);
    }
    Ok(values)
}

const MAX_PROJECTION_AUX_BYTES: u64 = 16 * 1024 * 1024;

async fn read_bounded_projection_text(path: &Path) -> std::io::Result<String> {
    let file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    if len > MAX_PROJECTION_AUX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{} is {len} bytes; maximum is {MAX_PROJECTION_AUX_BYTES}",
                path.display()
            ),
        ));
    }
    let mut bytes = Vec::with_capacity(len as usize);
    file.take(MAX_PROJECTION_AUX_BYTES + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() as u64 > MAX_PROJECTION_AUX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{} exceeded maximum {MAX_PROJECTION_AUX_BYTES}",
                path.display()
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} is not UTF-8: {error}", path.display()),
        )
    })
}

fn path_list(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn json_source_state(path: &Path, available: bool) -> Value {
    json!({
        "path": path.display().to_string(),
        "state": if available { "available" } else { "missing" },
    })
}

fn runner_json_source_state(
    path: &Path,
    available: bool,
    source: Option<RunnerProjectionSource>,
    status: &str,
    generation: Option<&str>,
    error: Option<&str>,
) -> Value {
    json!({
        "path": path.display().to_string(),
        "state": if available { "available" } else { status },
        "format": source.map_or_else(
            || status.to_string(),
            |source| source.label().to_string(),
        ),
        "generation": generation,
        "error": error,
    })
}

fn collection_availability(empty: bool, empty_reason: &str) -> Value {
    if empty {
        json!({
            "state": "empty",
            "reason": empty_reason,
        })
    } else {
        json!({ "state": "available" })
    }
}

fn json_blob_state(raw: &str, missing_reason: &str, fallback_endpoint: &str) -> Value {
    let raw = raw.trim();
    if raw.is_empty() {
        return json!({
            "state": "missing",
            "reason": missing_reason,
            "endpoint": fallback_endpoint,
        });
    }

    match serde_json::from_str::<Value>(raw) {
        Ok(value) => json!({
            "state": "available",
            "value": value,
        }),
        Err(error) => json!({
            "state": "invalid",
            "reason": error.to_string(),
            "raw": raw,
            "endpoint": fallback_endpoint,
        }),
    }
}

fn query_summary(query: &ProjectionQuery) -> Value {
    json!({
        "filter": query.filter,
        "limit": query.limit,
        "run_id": query.run_id,
        "plan_id": query.plan_id,
        "task_id": query.task_id,
        "agent_id": query.agent_id,
        "gate": query.gate,
        "role": query.role,
        "event_type": query.event_type,
        "active": query.active,
        "provider": query.provider,
        "model": query.model,
        "status": query.status,
        "episode_id": query.episode_id,
    })
}

fn task_matches_filter(
    task: &roko_core::dashboard_snapshot::TaskState,
    query: &ProjectionQuery,
) -> bool {
    plan_id_matches_filter(&task.plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| task.task_id == task_id)
        && query.active.is_none_or(|active| {
            if active {
                task.outcome.is_none()
            } else {
                task.outcome.is_some()
            }
        })
        && status_matches(
            task.outcome.as_deref().unwrap_or(task.phase.as_str()),
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => task.plan_id == value,
            "task" | "task_id" => task.task_id == value,
            "phase" | "status" => task.phase == value || task.outcome.as_deref() == Some(value),
            "active" => parse_bool(value).is_none_or(|active| {
                if active {
                    task.outcome.is_none()
                } else {
                    task.outcome.is_some()
                }
            }),
            _ => true,
        })
}

fn gate_matches_filter(
    gate: &roko_core::dashboard_snapshot::GateVerdictView,
    query: &ProjectionQuery,
) -> bool {
    plan_id_matches_filter(&gate.plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| gate.task_id == task_id)
        && gate_name_matches(&gate.gate, query)
        && status_matches(
            if gate.passed { "passed" } else { "failed" },
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => gate.plan_id == value,
            "task" | "task_id" => gate.task_id == value,
            "gate" => gate.gate == value,
            "passed" | "status" => parse_bool(value).is_none_or(|passed| gate.passed == passed),
            _ => true,
        })
}

fn plan_matches_filter(plan_id: &str, query: &ProjectionQuery) -> bool {
    plan_id_matches_filter(plan_id, query)
}

fn plan_id_matches_filter(plan_id: &str, query: &ProjectionQuery) -> bool {
    query
        .plan_id
        .as_deref()
        .or(query.run_id.as_deref())
        .is_none_or(|expected| plan_id == expected)
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => plan_id == value,
            _ => true,
        })
}

fn agent_matches_filter(
    agent: &roko_core::dashboard_snapshot::AgentState,
    query: &ProjectionQuery,
) -> bool {
    agent_id_matches_filter(&agent.agent_id, query)
        && query.role.as_deref().is_none_or(|role| agent.role == role)
        && query
            .model
            .as_deref()
            .is_none_or(|model| agent.model == model)
        && query
            .plan_id
            .as_deref()
            .or(query.run_id.as_deref())
            .is_none_or(|plan_id| {
                agent.current_plan == plan_id || agent.agent_id.starts_with(&format!("{plan_id}:"))
            })
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| agent.current_task == task_id)
        && query.active.is_none_or(|active| agent.active == active)
        && filter_terms_match(query, |key, value| match key {
            "agent" | "agent_id" => agent.agent_id == value,
            "role" => agent.role == value,
            "plan" | "plan_id" | "run" | "run_id" => {
                agent.current_plan == value || agent.agent_id.starts_with(&format!("{value}:"))
            }
            "task" | "task_id" => agent.current_task == value,
            "model" => agent.model == value,
            "active" => parse_bool(value).is_none_or(|active| agent.active == active),
            _ => true,
        })
}

fn agent_id_matches_filter(agent_id: &str, query: &ProjectionQuery) -> bool {
    query
        .agent_id
        .as_deref()
        .is_none_or(|expected| agent_id == expected)
        && filter_terms_match(query, |key, value| match key {
            "agent" | "agent_id" => agent_id == value,
            _ => true,
        })
}

fn episode_summary_matches_filter(
    episode: &roko_core::dashboard_snapshot::EpisodeSummary,
    query: &ProjectionQuery,
) -> bool {
    query
        .agent_id
        .as_deref()
        .is_none_or(|agent_id| episode.agent_id == agent_id)
        && query
            .role
            .as_deref()
            .is_none_or(|role| episode.role == role)
        && query
            .episode_id
            .as_deref()
            .is_none_or(|episode_id| episode.episode_id == episode_id)
        && status_matches(
            if episode.passed { "passed" } else { "failed" },
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "role" => episode.role == value,
            "agent" | "agent_id" => episode.agent_id == value,
            "episode" | "episode_id" => episode.episode_id == value,
            "passed" | "status" => parse_bool(value).is_none_or(|passed| episode.passed == passed),
            _ => true,
        })
}

fn episode_role_matches_filter(role: &str, filter: Option<&str>) -> bool {
    filter.is_none_or(|value| value.strip_prefix("role:").is_none_or(|r| r.trim() == role))
}

fn task_id_matches_filter(task_id: &str, query: &ProjectionQuery) -> bool {
    query
        .task_id
        .as_deref()
        .is_none_or(|expected| task_id == expected)
        && filter_terms_match(query, |key, value| match key {
            "task" | "task_id" => task_id == value,
            _ => true,
        })
}

fn event_log_entry_matches_filter(
    entry: &roko_core::dashboard_snapshot::DashboardEventLogEntry,
    query: &ProjectionQuery,
) -> bool {
    plan_id_matches_filter(&entry.plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| entry.task_id == task_id)
        && query
            .event_type
            .as_deref()
            .is_none_or(|event_type| entry.event_type == event_type)
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => entry.plan_id == value,
            "task" | "task_id" => entry.task_id == value,
            "type" | "event_type" => entry.event_type == value,
            _ => true,
        })
}

fn durable_runner_event_matches_filter(entry: &Value, query: &ProjectionQuery) -> bool {
    let plan_id = entry
        .get("plan_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let task_id = entry
        .get("task_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let event_type = entry
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let run_id = entry
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or_default();

    plan_id_matches_filter(plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|expected| task_id == expected)
        && query
            .event_type
            .as_deref()
            .is_none_or(|expected| event_type == expected)
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" => plan_id == value,
            "task" | "task_id" => task_id == value,
            "type" | "event_type" => event_type == value,
            "run" | "run_id" => run_id == value,
            _ => true,
        })
}

fn filter_terms_match<F>(query: &ProjectionQuery, mut pred: F) -> bool
where
    F: FnMut(&str, &str) -> bool,
{
    let Some(filter) = query.filter.as_deref() else {
        return true;
    };
    filter
        .split([',', ';'])
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .all(|term| {
            if let Some((key, value)) = term.split_once(':').or_else(|| term.split_once('=')) {
                pred(key.trim(), value.trim())
            } else {
                pred("id", term)
                    || pred("plan", term)
                    || pred("agent", term)
                    || pred("task", term)
                    || pred("gate", term)
                    || pred("provider", term)
                    || pred("model", term)
                    || pred("episode", term)
            }
        })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "passed" | "pass" | "active" | "success" | "succeeded" => Some(true),
        "false" | "0" | "no" | "failed" | "fail" | "inactive" | "error" => Some(false),
        _ => None,
    }
}

fn status_matches(actual: &str, expected: Option<&str>) -> bool {
    expected.is_none_or(|expected| {
        let actual = actual.trim().to_ascii_lowercase();
        let expected = expected.trim().to_ascii_lowercase();
        actual == expected
            || parse_bool(&expected)
                .is_some_and(|expected_bool| parse_bool(&actual) == Some(expected_bool))
    })
}

fn gate_name_matches(gate_name: &str, query: &ProjectionQuery) -> bool {
    query.gate.as_deref().is_none_or(|gate| gate_name == gate)
        && filter_terms_match(query, |key, value| match key {
            "gate" => gate_name == value,
            _ => true,
        })
}

fn episode_matches_query(episode: &Episode, query: &ProjectionQuery) -> bool {
    let plan_id = episode_plan_id(episode);
    let provider = episode_provider(episode);
    let role = episode_role(episode);
    plan_id_matches_filter(&plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| episode.task_id == task_id)
        && query
            .agent_id
            .as_deref()
            .is_none_or(|agent_id| episode.agent_id == agent_id)
        && query
            .role
            .as_deref()
            .is_none_or(|expected| role == expected)
        && query
            .provider
            .as_deref()
            .is_none_or(|expected| provider == expected)
        && query
            .model
            .as_deref()
            .is_none_or(|expected| episode.model == expected)
        && query
            .episode_id
            .as_deref()
            .is_none_or(|expected| episode_public_id(episode) == expected || episode.id == expected)
        && status_matches(
            if episode.success { "passed" } else { "failed" },
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => plan_id == value,
            "task" | "task_id" => episode.task_id == value,
            "agent" | "agent_id" => episode.agent_id == value,
            "role" => role == value,
            "provider" | "backend" => provider == value,
            "model" => episode.model == value,
            "episode" | "episode_id" => episode_public_id(episode) == value || episode.id == value,
            "status" | "passed" => parse_bool(value).is_none_or(|passed| episode.success == passed),
            _ => true,
        })
}

fn efficiency_event_matches_filter(event: &AgentEfficiencyEvent, query: &ProjectionQuery) -> bool {
    let model = first_non_empty_owned([Some(event.model.clone()), Some(event.model_used.clone())])
        .unwrap_or_default();
    plan_id_matches_filter(&event.plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| event.task_id == task_id)
        && query
            .agent_id
            .as_deref()
            .is_none_or(|agent_id| event.agent_id == agent_id)
        && query.role.as_deref().is_none_or(|role| event.role == role)
        && query
            .provider
            .as_deref()
            .is_none_or(|provider| event.backend == provider)
        && query
            .model
            .as_deref()
            .is_none_or(|expected| model == expected)
        && status_matches(
            match event.gate_passed {
                Some(true) => "passed",
                Some(false) => "failed",
                None => "unknown",
            },
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => event.plan_id == value,
            "task" | "task_id" => event.task_id == value,
            "agent" | "agent_id" => event.agent_id == value,
            "role" => event.role == value,
            "provider" | "backend" => event.backend == value,
            "model" => model == value,
            "status" | "passed" => {
                parse_bool(value).is_none_or(|passed| event.gate_passed == Some(passed))
            }
            _ => true,
        })
}

fn cost_record_matches_filter(record: &CostRecord, query: &ProjectionQuery) -> bool {
    plan_id_matches_filter(&record.plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| record.task_id == task_id)
        && query.role.as_deref().is_none_or(|role| record.role == role)
        && query
            .provider
            .as_deref()
            .is_none_or(|provider| record.provider == provider)
        && query
            .model
            .as_deref()
            .is_none_or(|model| record.model == model)
        && status_matches(
            if record.success { "passed" } else { "failed" },
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => record.plan_id == value,
            "task" | "task_id" => record.task_id == value,
            "role" => record.role == value,
            "provider" | "backend" => record.provider == value,
            "model" => record.model == value,
            "status" | "passed" => parse_bool(value).is_none_or(|passed| record.success == passed),
            _ => true,
        })
}

fn provider_outcome_matches_filter(
    outcome: &ProviderModelOutcomeRecord,
    query: &ProjectionQuery,
) -> bool {
    let plan_id = outcome.run_id.as_deref().unwrap_or_default();
    plan_id_matches_filter(plan_id, query)
        && query
            .task_id
            .as_deref()
            .is_none_or(|task_id| outcome.task_id == task_id)
        && query
            .provider
            .as_deref()
            .is_none_or(|provider| outcome.provider == provider)
        && query
            .model
            .as_deref()
            .is_none_or(|model| outcome.model == model)
        && status_matches(
            &outcome_status_label(outcome.status),
            query.status.as_deref(),
        )
        && filter_terms_match(query, |key, value| match key {
            "plan" | "plan_id" | "run" | "run_id" => plan_id == value,
            "task" | "task_id" => outcome.task_id == value,
            "provider" | "backend" => outcome.provider == value,
            "model" => outcome.model == value,
            "status" => outcome_status_label(outcome.status) == value,
            _ => true,
        })
}

fn episode_extra_string(episode: &Episode, key: &str) -> Option<String> {
    episode
        .extra
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn episode_extra_u64(episode: &Episode, key: &str) -> Option<u64> {
    episode.extra.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|raw| raw.parse::<u64>().ok()))
    })
}

fn episode_plan_id(episode: &Episode) -> String {
    episode_extra_string(episode, "plan_id")
        .or_else(|| episode_extra_string(episode, "run_id"))
        .unwrap_or_default()
}

fn episode_role(episode: &Episode) -> String {
    episode_extra_string(episode, "role")
        .or_else(|| episode_extra_string(episode, "role_id"))
        .or_else(|| {
            (!episode.agent_template.trim().is_empty()).then(|| episode.agent_template.clone())
        })
        .unwrap_or_default()
}

fn episode_provider(episode: &Episode) -> String {
    if !episode.backend.trim().is_empty() {
        return episode.backend.clone();
    }
    episode_extra_string(episode, "provider")
        .or_else(|| episode_extra_string(episode, "backend"))
        .unwrap_or_default()
}

fn episode_public_id(episode: &Episode) -> String {
    if episode.episode_id.trim().is_empty() {
        episode.id.clone()
    } else {
        episode.episode_id.clone()
    }
}

fn episode_retry_count(episode: &Episode) -> u64 {
    episode_extra_u64(episode, "retry_count")
        .or_else(|| episode_extra_u64(episode, "retries"))
        .or_else(|| {
            episode_extra_u64(episode, "iteration").map(|iteration| iteration.saturating_sub(1))
        })
        .or_else(|| episode_extra_u64(episode, "attempt").map(|attempt| attempt.saturating_sub(1)))
        .unwrap_or(0)
}

fn first_non_empty_owned<I>(values: I) -> Option<String>
where
    I: IntoIterator<Item = Option<String>>,
{
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

fn millis_from_datetime(timestamp: DateTime<Utc>) -> u64 {
    u64::try_from(timestamp.timestamp_millis()).unwrap_or_default()
}

fn parse_timestamp_millis(timestamp: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .and_then(|ts| u64::try_from(ts.timestamp_millis()).ok())
}

fn outcome_status_label(status: ProviderModelOutcomeStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{status:?}").to_ascii_lowercase())
}

fn signal_gate_evidence(entry: &Value, query: &ProjectionQuery) -> Option<Value> {
    let kind = entry.get("kind").and_then(Value::as_str)?;
    if !(kind == "gate_verdict" || kind.starts_with("gate:") || kind.starts_with("gate_")) {
        return None;
    }

    let gate = entry
        .pointer("/tags/gate")
        .and_then(Value::as_str)
        .or_else(|| entry.pointer("/body/data/gate").and_then(Value::as_str))
        .or_else(|| entry.pointer("/body/gate").and_then(Value::as_str))
        .or_else(|| kind.strip_prefix("gate:"))
        .or_else(|| kind.strip_prefix("gate_"))?;
    if !gate_name_matches(gate, query) {
        return None;
    }

    let passed = entry
        .pointer("/tags/passed")
        .and_then(Value::as_str)
        .and_then(parse_bool)
        .or_else(|| entry.pointer("/body/data/passed").and_then(Value::as_bool))
        .or_else(|| entry.pointer("/body/passed").and_then(Value::as_bool))?;

    let mut row = json!({
        "source": "signal_log",
        "id": entry.get("id").cloned().unwrap_or(Value::Null),
        "plan_id": entry
            .pointer("/tags/plan_id")
            .and_then(Value::as_str)
            .or_else(|| entry.pointer("/body/data/plan_id").and_then(Value::as_str))
            .or_else(|| entry.pointer("/body/plan_id").and_then(Value::as_str))
            .unwrap_or("unknown"),
        "task_id": entry
            .pointer("/tags/task_id")
            .and_then(Value::as_str)
            .or_else(|| entry.pointer("/body/data/task_id").and_then(Value::as_str))
            .or_else(|| entry.pointer("/body/task_id").and_then(Value::as_str))
            .unwrap_or_default(),
        "gate": gate,
        "passed": passed,
        "timestamp_ms": entry.get("created_at_ms").cloned().unwrap_or(Value::Null),
    });

    if let Some(duration_ms) = entry
        .pointer("/body/data/duration_ms")
        .and_then(Value::as_u64)
        .or_else(|| entry.pointer("/body/duration_ms").and_then(Value::as_u64))
        .or_else(|| entry.pointer("/tags/duration_ms").and_then(Value::as_u64))
    {
        row["duration_ms"] = Value::from(duration_ms);
    }

    if let Some(rung) = entry
        .pointer("/tags/rung")
        .or_else(|| entry.pointer("/body/data/rung"))
        .or_else(|| entry.pointer("/body/rung"))
        .and_then(|raw| {
            raw.as_u64()
                .or_else(|| raw.as_str().and_then(|text| text.parse::<u64>().ok()))
        })
    {
        row["rung"] = Value::from(rung);
    }

    Some(row)
}

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn value_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn value_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn value_f64(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or_default()
}

fn value_ts(value: &Value) -> u64 {
    value_u64(value, "timestamp_ms")
}

fn insert_source(value: &mut Value, source: &str) {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("source".into(), Value::String(source.to_string()));
    }
}

fn truncate_values(values: &mut Vec<Value>, limit: usize) {
    values.truncate(limit);
}

fn limited_values(mut values: Vec<Value>, limit: usize) -> Vec<Value> {
    values.truncate(limit);
    values
}

fn dedupe_and_sort_evidence(values: Vec<Value>) -> Vec<Value> {
    let mut by_key = BTreeMap::new();
    for value in values {
        let key = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            value_str(&value, "plan_id"),
            value_str(&value, "task_id"),
            value_str(&value, "provider"),
            value_str(&value, "model"),
            value_str(&value, "gate"),
            value_bool(&value, "passed"),
            value_ts(&value)
        );
        by_key
            .entry(key)
            .and_modify(|existing| {
                if evidence_detail_score(&value) > evidence_detail_score(existing) {
                    *existing = value.clone();
                }
            })
            .or_insert(value);
    }
    let mut out = by_key.into_values().collect::<Vec<_>>();
    out.sort_by_key(|v| std::cmp::Reverse(value_ts(v)));
    out
}

fn evidence_detail_score(value: &Value) -> u8 {
    u8::from(value.get("id").is_some_and(|id| !id.is_null()))
        + u8::from(value.get("duration_ms").is_some_and(Value::is_number))
        + u8::from(value.get("rung").is_some_and(Value::is_number))
        + u8::from(!value_str(value, "source").is_empty())
        + u8::from(!value_str(value, "provider").is_empty())
        + u8::from(!value_str(value, "model").is_empty())
}

#[derive(Debug, Default)]
struct GateAggregate {
    total: u64,
    passed: u64,
    duration_ms_total: u64,
    latest: Value,
}

fn summarize_gate_evidence(evidence: &[Value]) -> Value {
    let mut by_gate: BTreeMap<String, GateAggregate> = BTreeMap::new();
    let mut by_rung: BTreeMap<u64, (u64, u64)> = BTreeMap::new();

    for row in evidence {
        let gate = value_str(row, "gate");
        if gate.is_empty() {
            continue;
        }
        let passed = value_bool(row, "passed");
        let aggregate = by_gate.entry(gate.to_string()).or_default();
        aggregate.total += 1;
        if passed {
            aggregate.passed += 1;
        }
        aggregate.duration_ms_total = aggregate
            .duration_ms_total
            .saturating_add(value_u64(row, "duration_ms"));
        if aggregate.latest.is_null() || value_ts(row) >= value_ts(&aggregate.latest) {
            aggregate.latest = row.clone();
        }
        if let Some(rung) = row.get("rung").and_then(Value::as_u64) {
            let entry = by_rung.entry(rung).or_default();
            if passed {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }

    let mut out = serde_json::Map::new();
    for (gate, aggregate) in by_gate {
        out.insert(
            gate,
            json!({
                "total_runs": aggregate.total,
                "passed_runs": aggregate.passed,
                "failed_runs": aggregate.total.saturating_sub(aggregate.passed),
                "pass_rate": ratio(aggregate.passed, aggregate.total),
                "avg_duration_ms": if aggregate.total == 0 {
                    0.0
                } else {
                    aggregate.duration_ms_total as f64 / aggregate.total as f64
                },
                "last_run": aggregate.latest,
            }),
        );
    }

    let rungs = by_rung
        .into_iter()
        .map(|(rung, (passed, failed))| {
            let total = passed + failed;
            json!({
                "rung": rung,
                "passed_runs": passed,
                "failed_runs": failed,
                "total_runs": total,
                "pass_rate": ratio(passed, total),
            })
        })
        .collect::<Vec<_>>();
    out.insert("rungs".to_string(), Value::Array(rungs));
    Value::Object(out)
}

fn summarize_cost_evidence(records: &[Value], statehub_total_cost_usd: f64) -> Value {
    let mut total_cost = 0.0;
    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut cached_tokens = 0_u64;
    let mut duration_ms = 0_u64;
    let mut successes = 0_u64;
    let null_cost_count = records
        .iter()
        .filter(|record| value_f64(record, "cost_usd") == 0.0)
        .count();
    let mut by_provider: BTreeMap<String, f64> = BTreeMap::new();
    let mut by_model: BTreeMap<String, f64> = BTreeMap::new();
    let mut by_plan: BTreeMap<String, f64> = BTreeMap::new();
    let mut by_task: BTreeMap<String, f64> = BTreeMap::new();

    for record in records {
        let cost = value_f64(record, "cost_usd");
        total_cost += cost;
        input_tokens = input_tokens.saturating_add(value_u64(record, "input_tokens"));
        output_tokens = output_tokens.saturating_add(value_u64(record, "output_tokens"));
        cached_tokens = cached_tokens.saturating_add(value_u64(record, "cached_tokens"));
        duration_ms = duration_ms.saturating_add(value_u64(record, "duration_ms"));
        if value_bool(record, "success") {
            successes += 1;
        }

        add_cost(&mut by_provider, value_str(record, "provider"), cost);
        add_cost(&mut by_model, value_str(record, "model"), cost);
        add_cost(&mut by_plan, value_str(record, "plan_id"), cost);
        add_cost(&mut by_task, value_str(record, "task_id"), cost);
    }

    let record_count = records.len() as u64;
    let data_quality =
        DataQuality::from_counts(!records.is_empty(), records.len(), null_cost_count);
    let total_cost_value = if record_count == 0 || null_cost_count == records.len() {
        Value::Null
    } else {
        Value::from(total_cost)
    };
    let avg_cost_value = if record_count == 0 || null_cost_count == records.len() {
        Value::Null
    } else {
        Value::from(total_cost / record_count as f64)
    };
    json!({
        "total_cost_usd": total_cost_value,
        "records_total_cost_usd": total_cost,
        "statehub_total_cost_usd": statehub_total_cost_usd,
        "total_input_tokens": input_tokens,
        "total_output_tokens": output_tokens,
        "total_cached_tokens": cached_tokens,
        "record_count": records.len(),
        "avg_cost_usd": avg_cost_value,
        "avg_duration_ms": if record_count == 0 { 0.0 } else { duration_ms as f64 / record_count as f64 },
        "success_rate": ratio(successes, record_count),
        "by_provider": by_provider,
        "by_model": by_model,
        "by_plan": by_plan,
        "by_task": by_task,
        "null_cost_count": null_cost_count,
        "data_quality": data_quality,
    })
}

fn add_cost(map: &mut BTreeMap<String, f64>, key: &str, cost: f64) {
    if key.trim().is_empty() {
        return;
    }
    *map.entry(key.to_string()).or_default() += cost;
}

fn summarize_cost_data_quality(records: &[Value]) -> DataQuality {
    DataQuality::from_counts(
        !records.is_empty(),
        records.len(),
        records
            .iter()
            .filter(|record| value_f64(record, "cost_usd") == 0.0)
            .count(),
    )
}

fn cascade_router_data_quality(value: Option<&Value>) -> DataQuality {
    let entry_count = value
        .map(|value| {
            let mut slugs = BTreeSet::new();
            if let Some(items) = value.get("model_slugs").and_then(Value::as_array) {
                slugs.extend(
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned),
                );
            }
            if let Some(items) = value.get("confidence_stats").and_then(Value::as_object) {
                slugs.extend(items.keys().cloned());
            }
            slugs.len()
        })
        .unwrap_or(0);

    DataQuality::from_counts(entry_count > 0, entry_count, 0)
}

#[derive(Debug, Default)]
struct ProviderAggregate {
    attempts: u64,
    successes: u64,
    retries: u64,
    cost_usd: f64,
    tokens: u64,
    latency_ms: u64,
    latest_timestamp_ms: u64,
}

fn summarize_provider_outcomes(outcomes: &[Value]) -> Value {
    let mut by_provider_model: BTreeMap<String, ProviderAggregate> = BTreeMap::new();
    let mut by_provider: BTreeMap<String, ProviderAggregate> = BTreeMap::new();

    for outcome in outcomes {
        let provider = value_str(outcome, "provider");
        let model = value_str(outcome, "model");
        if provider.is_empty() && model.is_empty() {
            continue;
        }
        let key = format!("{provider}/{model}");
        record_provider_aggregate(by_provider_model.entry(key).or_default(), outcome);
        if !provider.is_empty() {
            record_provider_aggregate(
                by_provider.entry(provider.to_string()).or_default(),
                outcome,
            );
        }
    }

    json!({
        "by_provider_model": provider_aggregate_rows(by_provider_model),
        "by_provider": provider_aggregate_rows(by_provider),
        "total_outcomes": outcomes.len(),
    })
}

fn record_provider_aggregate(aggregate: &mut ProviderAggregate, outcome: &Value) {
    aggregate.attempts += 1;
    if value_bool(outcome, "success") {
        aggregate.successes += 1;
    }
    aggregate.retries = aggregate
        .retries
        .saturating_add(value_u64(outcome, "retry_count"));
    let usage = outcome.get("usage").unwrap_or(&Value::Null);
    aggregate.cost_usd += usage
        .get("cost_usd")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    aggregate.tokens = aggregate.tokens.saturating_add(
        usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    );
    aggregate.latency_ms = aggregate.latency_ms.saturating_add(
        usage
            .get("latency_ms")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    );
    aggregate.latest_timestamp_ms = aggregate.latest_timestamp_ms.max(value_ts(outcome));
}

fn provider_aggregate_rows(map: BTreeMap<String, ProviderAggregate>) -> Vec<Value> {
    map.into_iter()
        .map(|(key, aggregate)| {
            json!({
                "key": key,
                "attempts": aggregate.attempts,
                "successes": aggregate.successes,
                "failures": aggregate.attempts.saturating_sub(aggregate.successes),
                "success_rate": ratio(aggregate.successes, aggregate.attempts),
                "retry_count": aggregate.retries,
                "cost_usd": aggregate.cost_usd,
                "tokens": aggregate.tokens,
                "avg_latency_ms": if aggregate.attempts == 0 {
                    0.0
                } else {
                    aggregate.latency_ms as f64 / aggregate.attempts as f64
                },
                "latest_timestamp_ms": aggregate.latest_timestamp_ms,
            })
        })
        .collect()
}

#[derive(Debug, Default)]
struct RetryAggregate {
    attempts: u64,
    retries: u64,
    failures: u64,
    successes: u64,
    latest_timestamp_ms: u64,
    strategies: BTreeSet<String>,
}

fn summarize_retry_evidence(attempts: &[Value]) -> Value {
    let mut by_task: BTreeMap<String, RetryAggregate> = BTreeMap::new();
    for attempt in attempts {
        let plan_id = value_str(attempt, "plan_id");
        let task_id = value_str(attempt, "task_id");
        let key = if plan_id.is_empty() {
            task_id.to_string()
        } else {
            format!("{plan_id}/{task_id}")
        };
        if key.trim().is_empty() {
            continue;
        }
        let aggregate = by_task.entry(key).or_default();
        aggregate.attempts += 1;
        aggregate.retries = aggregate
            .retries
            .saturating_add(value_u64(attempt, "retry_count"));
        if status_matches("failed", attempt.get("status").and_then(Value::as_str)) {
            aggregate.failures += 1;
        }
        if status_matches("passed", attempt.get("status").and_then(Value::as_str)) {
            aggregate.successes += 1;
        }
        aggregate.latest_timestamp_ms = aggregate.latest_timestamp_ms.max(value_ts(attempt));
        let strategy = value_str(attempt, "strategy_attempted");
        if !strategy.is_empty() {
            aggregate.strategies.insert(strategy.to_string());
        }
    }

    let tasks = by_task
        .into_iter()
        .map(|(task, aggregate)| {
            json!({
                "task": task,
                "attempt_events": aggregate.attempts,
                "retry_count": aggregate.retries,
                "failures": aggregate.failures,
                "successes": aggregate.successes,
                "latest_timestamp_ms": aggregate.latest_timestamp_ms,
                "strategies": aggregate.strategies.into_iter().collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "tasks": tasks,
        "total_attempt_events": attempts.len(),
    })
}

fn ratio(numer: u64, denom: u64) -> f64 {
    if denom == 0 {
        0.0
    } else {
        numer as f64 / denom as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_policies_have_non_zero_max_age() {
        for entry in projection_policies() {
            assert!(
                entry.policy.max_age_secs > 0,
                "projection '{}' has zero max_age_secs",
                entry.name
            );
        }
    }

    #[test]
    fn all_policies_have_at_least_one_trigger() {
        for entry in projection_policies() {
            assert!(
                !entry.policy.invalidation_triggers.is_empty(),
                "projection '{}' has no invalidation triggers",
                entry.name
            );
        }
    }

    #[test]
    fn version_lookup_resolves_aliases() {
        assert_eq!(projection_version("marketplace_jobs"), Some(2));
        assert_eq!(projection_version("jobs"), Some(2));
        assert_eq!(projection_version("dashboard"), Some(1));
        assert_eq!(projection_version("dashboard_snapshot"), Some(1));
        assert_eq!(projection_version("agent_state"), Some(1));
        assert_eq!(projection_version("agent_trails"), Some(1));
        assert_eq!(projection_version("plan_state"), Some(1));
        assert_eq!(projection_version("plans_list"), Some(1));
        assert_eq!(projection_version("gate_state"), Some(1));
        assert_eq!(projection_version("gate_pipeline"), Some(1));
        assert_eq!(projection_version("learning_policy_state"), Some(1));
        assert_eq!(projection_version("learning_policy"), Some(1));
        assert_eq!(projection_version("events"), Some(1));
        assert_eq!(projection_version("event_log"), Some(1));
        assert_eq!(projection_version("task_outputs"), Some(1));
        assert_eq!(projection_version("cost_meter"), Some(1));
        assert_eq!(projection_version("executor_state"), Some(1));
        assert_eq!(projection_version("atelier"), Some(1));
        assert_eq!(projection_version("knowledge_entries"), Some(1));
        assert_eq!(projection_version("nonexistent"), None);
    }

    #[test]
    fn envelope_round_trips_through_serde() {
        let envelope = ProjectionEnvelope {
            name: "test".into(),
            version: 1,
            cursor: 42,
            computed_at: "2026-04-21T00:00:00Z".into(),
            recovered: false,
            data: serde_json::json!({"items": []}),
        };
        let json = serde_json::to_string(&envelope).expect("serialize");
        let decoded: ProjectionEnvelope<serde_json::Value> =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.name, "test");
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.cursor, 42);
        assert!(!decoded.recovered);
    }

    #[test]
    fn projection_invalidation_routes_only_to_its_live_stream() {
        let event = DashboardEvent::ProjectionUpdated {
            projection_id: "cost_meter".into(),
            version: 2,
            source_lens: "cost-monitor".into(),
        };
        let query = ProjectionQuery::default();

        assert!(projection_accepts_event("cost_meter", &query, &event));
        assert!(!projection_accepts_event("c_factor", &query, &event));
        let frame = projection_delta_frame("cost_meter", 7, &event);
        assert_eq!(frame["channel"], "projection:cost_meter");
        assert_eq!(frame["delta"]["type"], "projection_updated");
        assert_eq!(frame["delta"]["version"], 2);
    }

    #[test]
    fn runner_source_state_reports_missing_and_invalid_without_format_drift() {
        let path = Path::new(".roko/state/state-snapshot.json");
        let missing = runner_json_source_state(path, false, None, "missing", None, None);
        assert_eq!(missing["state"], "missing");
        assert_eq!(missing["format"], "missing");

        let invalid = runner_json_source_state(
            path,
            false,
            None,
            "invalid",
            None,
            Some("checksum mismatch"),
        );
        assert_eq!(invalid["state"], "invalid");
        assert_eq!(invalid["format"], "invalid");
        assert_eq!(invalid["error"], "checksum mismatch");

        let canonical = runner_json_source_state(
            path,
            true,
            Some(RunnerProjectionSource::StateSnapshot),
            "state_snapshot",
            Some("generation"),
            None,
        );
        assert_eq!(canonical["state"], "available");
        assert_eq!(canonical["format"], "state_snapshot");
    }

    #[tokio::test]
    async fn invalid_authoritative_runner_never_falls_back_to_standalone_thresholds() {
        let dir = tempfile::tempdir().unwrap();
        let learn = dir.path().join(".roko/learn");
        std::fs::create_dir_all(&learn).unwrap();
        std::fs::write(
            learn.join("gate-thresholds.json"),
            serde_json::json!({"rungs": {"1": {"pass_count": 9}}}).to_string(),
        )
        .unwrap();

        let feedback = RuntimeFeedbackProjection::load(
            dir.path(),
            None,
            Some("invalid"),
            Some("checksum mismatch"),
        )
        .await
        .unwrap();
        assert!(feedback.gate_thresholds_json.is_none());
        assert_eq!(feedback.runner_projection_status, "invalid");
        assert_eq!(
            feedback.runner_projection_error.as_deref(),
            Some("checksum mismatch")
        );
    }

    #[test]
    fn summarize_cost_evidence_preserves_missing_data_as_null() {
        let summary = summarize_cost_evidence(&[], 12.34);

        assert!(summary["total_cost_usd"].is_null());
        assert_eq!(summary["records_total_cost_usd"], 0.0);
        assert_eq!(summary["statehub_total_cost_usd"], 12.34);
        assert_eq!(summary["null_cost_count"], 0);
        assert_eq!(summary["data_quality"]["has_real_data"], false);
        assert_eq!(summary["data_quality"]["entry_count"], 0);
        assert_eq!(summary["data_quality"]["null_cost_count"], 0);
    }
}
