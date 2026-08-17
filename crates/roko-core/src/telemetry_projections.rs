//! Portable StateHub projection schemas produced from telemetry Lens output.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Fleet-level health summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CohortHealthProjection {
    pub agent_count: usize,
    pub active_count: usize,
    pub avg_vitality: f64,
    pub avg_pass_rate: f64,
    pub total_spend_usd: f64,
    pub error_rate: f64,
    pub t0_hit_rate: f64,
    pub regime_distribution: BTreeMap<String, usize>,
}

/// One task in the active-task projection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub title: String,
    pub status: String,
    pub agent: Option<String>,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Active and recently completed task summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveTasksProjection {
    pub tasks: Vec<TaskSnapshot>,
    pub queued: usize,
    pub completed_last_hour: usize,
    pub avg_task_duration_ms: u64,
}

/// One verification rung's aggregate outcome.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RungSnapshot {
    pub name: String,
    pub pass_count: u64,
    pub fail_count: u64,
    pub pass_rate: f64,
}

/// Verification-pipeline health summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GatePipelineProjection {
    pub rungs: Vec<RungSnapshot>,
    pub overall_pass_rate: f64,
    pub avg_reward: f64,
    pub hard_criteria_fail_rate: f64,
}

/// Cost, remaining budget, and trend summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostMeterProjection {
    pub total_usd: f64,
    pub budget_remaining: f64,
    pub burn_rate_usd_per_hour: f64,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cost_trend: String,
}

/// Knowledge-store quality and lifecycle summary.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeHealthProjection {
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<String, u64>,
    pub avg_balance: f64,
    pub cold_entries: u64,
    pub heuristic_count: u64,
    pub heuristic_avg_calibration: f64,
    pub anti_knowledge_count: u64,
}

/// Collective-intelligence projection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CFactorProjection {
    pub c_factor: f64,
    pub components: BTreeMap<String, f64>,
    pub trend: String,
    pub agent_diversity: f64,
}

/// One agent's current vitality state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentVitalitySnapshot {
    pub name: String,
    pub vitality: f64,
    pub phase: String,
    pub regime: String,
    pub slots_active: usize,
    pub slots_total: usize,
    pub tasks_completed: u64,
    pub current_task: Option<String>,
}

/// Agent vitality collection exposed to surfaces.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentVitalityProjection {
    pub agents: Vec<AgentVitalitySnapshot>,
}
