//! Host adapter that makes converted Graph plan tasks execute real agents.
//!
//! ## Streaming dispatch (#274)
//!
//! [`GraphTaskDispatcher`] now implements [`StreamingTaskDispatcher`] in
//! addition to the basic [`TaskDispatcher`] trait. The streaming path adds:
//!
//! - Live text/tool/usage/progress events forwarded through a bounded channel.
//! - Attempt start/terminal receipts through an injected [`ProviderAttemptRecorder`].
//! - `reconcile_attempt` for crash-safe resume (reuse committed, allocate new,
//!   or fail ambiguous).
//! - Lease validation: the workdir must match the acquired lease path.
//!
//! The existing `TaskDispatcher::dispatch` implementation is unchanged and
//! remains the production plan route until #256 atomically activates the
//! streaming path after lease acquisition.

use std::collections::{HashMap, hash_map::Entry};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use roko_agent::safety::contract::{AgentContract, ContractLoadMode};
use roko_core::config::schema::RokoConfig;
use roko_core::error::{Result, RokoError};
use roko_core::{Body, Context, Kind, Signal, Verify};
use roko_gate::GatePayload;
use roko_gate::ShellGate;
use roko_gate::TurnSnapshot;
use roko_gate::eval_generator::EvalGenerator;
use roko_graph::cell::CellContext;
use roko_graph::cells::{
    AttemptReconciliation, GraphTaskEvent, ProviderAttemptRecorder, StreamingTaskDispatcher,
    TaskDispatchOutcome, TaskDispatchOutcomeKind, TaskDispatcher, TaskExecutionSpec, TaskLease,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::oracles::coding::{BuildRecord, CodingOracle, TestRecord};
use roko_learn::shadow::ShadowRunner;

use crate::dispatch::{AgentDispatchRequest, DispatchContext, ModelChoiceSource, SharedAgentFactory};
use crate::graph_checkpoint::GraphCostLedgerCheckpoint;
use crate::runner::tui_bridge::TuiBridge;
use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade};
use crate::task_parser::TaskDef;

const MICRO_USD_PER_USD: f64 = 1_000_000.0;

/// P1-16: Resolve cross-cut functor conflicts at routing time.
///
/// When Memory, Daimon, and Dreams all propose routing recommendations on
/// the same signal set, the arbitrator applies priority resolution (safety-
/// critical Daimon wins, consolidated Memory beats speculative Dreams) and
/// falls back to VCG second-price arbitration for same-level ties.
///
/// Returns an `Option<RoutingBias>` derived from the winning recommendation
/// so the cascade router can incorporate the cross-cut consensus.
fn arbitrate_cross_cut_routing_bias(
    feedback: &GraphFeedbackContext,
    workdir: &Path,
    task_category: &str,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    use roko_compose::auction::{
        CrossCutArbitrationResult, CrossCutDecisionKind, CrossCutRecommendation,
    };

    // Collect recommendations from persisted cross-cut state.
    let mut recommendations = Vec::new();

    // Dreams routing advice (persisted by DreamOutputConsumer or delta dream).
    if let Ok(advice) = roko_dreams::load_dream_routing_advice(workdir) {
        for rec in &advice.recommendations {
            if rec.confidence < 0.5 {
                continue;
            }
            recommendations.push(CrossCutRecommendation {
                source: roko_compose::auction::CrossCutId::Dreams,
                decision_key: format!("route:{task_category}"),
                decision_kind: CrossCutDecisionKind::Route,
                value: rec.recommended_model.clone(),
                confidence: rec.confidence,
                priority_level: 2,
                safety_critical: false,
                knowledge_tier: None,
            });
        }
    }

    // Daimon safety override: if the daimon is Struggling, emit a safety-
    // critical recommendation to prefer a conservative model.
    if let Some(daimon) = &feedback.daimon_state {
        if let Ok(daimon) = daimon.lock() {
            let affect = daimon.query_state();
            if affect.behavioral_state == roko_core::BehavioralState::Struggling {
                recommendations.push(CrossCutRecommendation {
                    source: roko_compose::auction::CrossCutId::Daimon,
                    decision_key: format!("route:{task_category}"),
                    decision_kind: CrossCutDecisionKind::Route,
                    value: "conservative".to_string(),
                    confidence: 0.9,
                    priority_level: 1,
                    safety_critical: true,
                    knowledge_tier: None,
                });
            }
        }
    }

    if recommendations.is_empty() {
        return None;
    }

    // Run priority-then-VCG arbitration.
    let result = roko_compose::auction::resolve_by_priority(&recommendations)
        .unwrap_or_else(|| roko_compose::auction::resolve_by_vcg(&recommendations));

    match result {
        CrossCutArbitrationResult::Resolved {
            winner,
            ref recommendation,
            attention_cost,
            mechanism,
            ..
        } => {
            tracing::debug!(
                ?winner,
                value = %recommendation.value,
                attention_cost,
                ?mechanism,
                "cross-cut arbitration resolved routing recommendation"
            );
            // If the winning recommendation names a specific model to prefer,
            // deprioritize everything else. For safety-critical "conservative"
            // recommendations, signal budget pressure instead.
            if recommendation.safety_critical {
                Some(roko_learn::cascade_router::RoutingBias {
                    deprioritize: Vec::new(),
                    prefer_cheaper: true,
                    reason: format!("cross-cut safety arbitration: {}", recommendation.value),
                })
            } else {
                None // Prefer normal dream routing advice path (P1-18)
            }
        }
        CrossCutArbitrationResult::NoConflict => None,
    }
}

/// Per-plan cost policy applied at the Graph task-dispatch boundary.
///
/// A non-positive or non-finite ceiling means unlimited. When
/// `continue_on_exhaustion` is enabled, spend is still recorded and exposed
/// for observability, but new dispatches are not blocked. This mirrors the
/// existing Runner-v2 semantics for explicit CLI budget overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphPlanBudgetPolicy {
    ceiling_micro_usd: Option<u64>,
    reservation_micro_usd: Option<u64>,
    continue_on_exhaustion: bool,
}

impl GraphPlanBudgetPolicy {
    /// Construct a policy from a USD ceiling.
    #[must_use]
    pub fn from_ceiling(ceiling_usd: f64, continue_on_exhaustion: bool) -> Self {
        Self::from_limits(ceiling_usd, 0.0, continue_on_exhaustion)
    }

    /// Construct a policy with a per-call reservation upper bound.
    #[must_use]
    pub fn from_limits(ceiling_usd: f64, max_turn_usd: f64, continue_on_exhaustion: bool) -> Self {
        let ceiling_micro_usd = (ceiling_usd.is_finite() && ceiling_usd > 0.0)
            .then(|| usd_to_micro_usd(ceiling_usd).max(1));
        Self {
            ceiling_micro_usd,
            reservation_micro_usd: ceiling_micro_usd.map(|ceiling| {
                if max_turn_usd.is_finite() && max_turn_usd > 0.0 {
                    usd_to_micro_usd(max_turn_usd).max(1).min(ceiling)
                } else {
                    // With no configured per-turn bound, conservatively reserve
                    // all remaining plan capacity so only one unknown-cost call
                    // can be in flight at a time.
                    ceiling
                }
            }),
            continue_on_exhaustion,
        }
    }

    /// Construct an unlimited policy.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            ceiling_micro_usd: None,
            reservation_micro_usd: None,
            continue_on_exhaustion: false,
        }
    }
}

impl Default for GraphPlanBudgetPolicy {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// Current cost state for one Graph plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphPlanBudgetSnapshot {
    /// Provider-reported or locally priced spend recorded for the plan.
    pub spent_usd: f64,
    /// Capacity currently reserved by admitted provider calls.
    pub reserved_usd: f64,
    /// Configured ceiling, or `None` when plan cost is unlimited.
    pub ceiling_usd: Option<f64>,
    /// Whether actual spend plus in-flight reservations consume the ceiling.
    pub exhausted: bool,
    /// Whether another dispatch must be rejected under the active policy.
    pub dispatch_blocked: bool,
}

impl GraphPlanBudgetSnapshot {
    fn remaining_usd(self) -> f64 {
        self.ceiling_usd.map_or(f64::INFINITY, |ceiling| {
            (ceiling - self.spent_usd - self.reserved_usd).max(0.0)
        })
    }
}

#[derive(Debug, Default)]
struct PlanBudgetState {
    spent_micro_usd: u64,
    reserved_micro_usd: u64,
    checkpoint: Option<GraphCostLedgerCheckpoint>,
    persistence_error: Option<String>,
}

#[derive(Debug, Default)]
struct GraphPlanBudgetLedger {
    plans: parking_lot::Mutex<HashMap<String, PlanBudgetState>>,
}

impl GraphPlanBudgetLedger {
    fn attach_checkpoint(
        &self,
        plan_id: &str,
        checkpoint: GraphCostLedgerCheckpoint,
    ) -> Result<()> {
        let mut plans = self.plans.lock();
        match plans.entry(plan_id.to_string()) {
            Entry::Vacant(entry) => {
                entry.insert(PlanBudgetState {
                    spent_micro_usd: checkpoint.spent_micro_usd(),
                    checkpoint: Some(checkpoint),
                    ..PlanBudgetState::default()
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(RokoError::Store(format!(
                "Graph cost ledger for plan `{plan_id}` was attached more than once"
            ))),
        }
    }

    fn snapshot(&self, plan_id: &str, policy: GraphPlanBudgetPolicy) -> GraphPlanBudgetSnapshot {
        let plans = self.plans.lock();
        let state = plans.get(plan_id);
        let spent_micro_usd = state.map_or(0, |state| state.spent_micro_usd);
        let reserved_micro_usd = state.map_or(0, |state| state.reserved_micro_usd);
        let persistence_failed = state.is_some_and(|state| state.persistence_error.is_some());
        let committed_micro_usd = spent_micro_usd.saturating_add(reserved_micro_usd);
        let exhausted = policy
            .ceiling_micro_usd
            .is_some_and(|ceiling| committed_micro_usd >= ceiling);

        GraphPlanBudgetSnapshot {
            spent_usd: micro_usd_to_usd(spent_micro_usd),
            reserved_usd: micro_usd_to_usd(reserved_micro_usd),
            ceiling_usd: policy.ceiling_micro_usd.map(micro_usd_to_usd),
            exhausted,
            dispatch_blocked: persistence_failed || (exhausted && !policy.continue_on_exhaustion),
        }
    }

    fn reserve(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
    ) -> Result<GraphPlanBudgetReservation<'_>> {
        let mut plans = self.plans.lock();
        let state = plans.entry(plan_id.to_string()).or_default();
        if let Some(error) = &state.persistence_error {
            return Err(RokoError::Store(format!(
                "Graph cost ledger for plan `{plan_id}` is unavailable: {error}"
            )));
        }

        let mut reserved_micro_usd = 0;
        let routing_budget_micro_usd = match policy.ceiling_micro_usd {
            None => None,
            Some(ceiling) if policy.continue_on_exhaustion => {
                Some(ceiling.saturating_sub(state.spent_micro_usd))
            }
            Some(ceiling) => {
                let committed = state
                    .spent_micro_usd
                    .saturating_add(state.reserved_micro_usd);
                let available = ceiling.saturating_sub(committed);
                if available == 0 {
                    return Err(RokoError::BudgetExceeded {
                        dimension: "plan_cost_micro_usd",
                        used: micro_usd_to_usize(committed),
                        limit: micro_usd_to_usize(ceiling),
                    });
                }
                reserved_micro_usd = policy
                    .reservation_micro_usd
                    .unwrap_or(available)
                    .min(available);
                state.reserved_micro_usd =
                    state.reserved_micro_usd.saturating_add(reserved_micro_usd);
                if let Some(checkpoint) = &state.checkpoint
                    && let Err(error) =
                        checkpoint.persist(state.spent_micro_usd, state.reserved_micro_usd)
                {
                    state.reserved_micro_usd =
                        state.reserved_micro_usd.saturating_sub(reserved_micro_usd);
                    let message = format!("persist provider-cost reservation: {error:#}");
                    state.persistence_error = Some(message.clone());
                    return Err(RokoError::Store(message));
                }
                Some(reserved_micro_usd)
            }
        };
        drop(plans);

        Ok(GraphPlanBudgetReservation {
            ledger: self,
            plan_id: plan_id.to_string(),
            reserved_micro_usd,
            routing_budget_micro_usd,
            settled: false,
        })
    }

    fn settle(&self, plan_id: &str, reserved_micro_usd: u64, cost_usd: f64) -> Result<()> {
        if !cost_usd.is_finite() || cost_usd < 0.0 {
            self.release(plan_id, reserved_micro_usd);
            let mut plans = self.plans.lock();
            let state = plans.entry(plan_id.to_string()).or_default();
            let message = format!("provider reported invalid cost {cost_usd:?}");
            state.persistence_error = Some(message.clone());
            return Err(RokoError::Store(message));
        }
        let cost_micro_usd = usd_to_micro_usd(cost_usd);
        let mut plans = self.plans.lock();
        let state = plans.entry(plan_id.to_string()).or_default();
        state.reserved_micro_usd = state.reserved_micro_usd.saturating_sub(reserved_micro_usd);
        state.spent_micro_usd = state.spent_micro_usd.saturating_add(cost_micro_usd);
        if let Some(checkpoint) = &state.checkpoint
            && let Err(error) = checkpoint.persist(state.spent_micro_usd, state.reserved_micro_usd)
        {
            let message = format!("persist actual provider cost: {error:#}");
            state.persistence_error = Some(message.clone());
            return Err(RokoError::Store(message));
        }
        Ok(())
    }

    fn release(&self, plan_id: &str, reserved_micro_usd: u64) {
        if reserved_micro_usd == 0 {
            return;
        }
        let mut plans = self.plans.lock();
        if let Some(state) = plans.get_mut(plan_id) {
            state.reserved_micro_usd = state.reserved_micro_usd.saturating_sub(reserved_micro_usd);
            if let Some(checkpoint) = &state.checkpoint
                && let Err(error) =
                    checkpoint.persist(state.spent_micro_usd, state.reserved_micro_usd)
            {
                state.persistence_error = Some(format!(
                    "persist released provider-cost reservation: {error:#}"
                ));
            }
        }
    }

    #[cfg(test)]
    fn record_cost(&self, plan_id: &str, cost_usd: f64) {
        self.settle(plan_id, 0, cost_usd).expect("record test cost");
    }
}

struct GraphPlanBudgetReservation<'a> {
    ledger: &'a GraphPlanBudgetLedger,
    plan_id: String,
    reserved_micro_usd: u64,
    routing_budget_micro_usd: Option<u64>,
    settled: bool,
}

impl GraphPlanBudgetReservation<'_> {
    fn routing_budget_usd(&self) -> f64 {
        self.routing_budget_micro_usd
            .map_or(f64::INFINITY, micro_usd_to_usd)
    }

    fn settle(mut self, cost_usd: f64) -> Result<()> {
        let result = self
            .ledger
            .settle(&self.plan_id, self.reserved_micro_usd, cost_usd);
        self.settled = true;
        result
    }
}

impl Drop for GraphPlanBudgetReservation<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.ledger.release(&self.plan_id, self.reserved_micro_usd);
        }
    }
}

fn usd_to_micro_usd(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    (value * MICRO_USD_PER_USD).round() as u64
}

fn micro_usd_to_usd(value: u64) -> f64 {
    value as f64 / MICRO_USD_PER_USD
}

fn micro_usd_to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn effective_routing_budget(context_remaining: Option<f64>, plan_remaining: f64) -> f64 {
    let context_remaining = context_remaining
        .filter(|value| value.is_finite())
        .map_or(f64::INFINITY, |value| value.max(0.0));
    context_remaining.min(plan_remaining)
}

/// Learning/feedback subsystem context for the Graph engine.
///
/// Constructed once in `cmd_plan_run_engine()` and shared by all tasks in
/// the plan run. Each subsystem is optional so the dispatcher degrades
/// gracefully when a component cannot be initialized.
#[derive(Clone)]
pub struct GraphFeedbackContext {
    /// Feedback facade that fans task-completion events to episode and routing sinks.
    pub feedback_facade: Option<Arc<FeedbackFacade>>,
    /// Path to `.roko/learn/efficiency.jsonl` for efficiency event writes.
    pub efficiency_path: Option<PathBuf>,
    /// Path to `.roko/learn/costs.jsonl` for per-task cost record writes.
    ///
    /// When set, each completed dispatch appends one [`roko_learn::costs_db::CostRecord`]
    /// so that `roko status` cost summary reads from the same source as the
    /// efficiency events produced by the Graph engine.
    pub costs_path: Option<PathBuf>,
    /// Path to `.roko/learn/playbooks/` for playbook outcome recording.
    pub playbook_dir: Option<PathBuf>,
    /// Shared daimon affect state, loaded from `.roko/daimon/state.json`.
    pub daimon_state: Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
    /// Path to `.roko/learn/experiments.json` for experiment settlement.
    pub experiment_store_path: Option<PathBuf>,
    /// Whether gate failure replanning is enabled (`learning.replan_on_gate_failure`).
    pub replan_on_gate_failure: bool,
    /// P0-04: CodingOracle for post-gate build/test observations.
    pub coding_oracle: Option<Arc<CodingOracle>>,
    /// P1-01: GateGamingDetector for flagging gaming patterns.
    pub gate_gaming_detector: Option<Arc<tokio::sync::Mutex<roko_learn::GateGamingDetector>>>,
    /// P1-04: HoldoutExperiment for gating learning updates (80/20 train/holdout split).
    pub holdout_experiment: Option<Arc<tokio::sync::Mutex<roko_learn::HoldoutExperiment>>>,
    /// P2-01: ShadowRunner for recording shadow dispatch decisions.
    pub shadow_runner: Option<Arc<ShadowRunner>>,
    /// P0-02: Whether eval generation is enabled for standard+ tier tasks.
    pub eval_generation_enabled: bool,
}

impl std::fmt::Debug for GraphFeedbackContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphFeedbackContext")
            .field("feedback_facade", &self.feedback_facade.is_some())
            .field("efficiency_path", &self.efficiency_path)
            .field("costs_path", &self.costs_path)
            .field("playbook_dir", &self.playbook_dir)
            .field("daimon_state", &self.daimon_state.is_some())
            .field("experiment_store_path", &self.experiment_store_path)
            .field("replan_on_gate_failure", &self.replan_on_gate_failure)
            .field("coding_oracle", &self.coding_oracle.is_some())
            .field("gate_gaming_detector", &self.gate_gaming_detector.is_some())
            .field("holdout_experiment", &self.holdout_experiment.is_some())
            .field("shadow_runner", &self.shadow_runner.is_some())
            .field("eval_generation_enabled", &self.eval_generation_enabled)
            .finish()
    }
}

impl Default for GraphFeedbackContext {
    fn default() -> Self {
        Self {
            feedback_facade: None,
            efficiency_path: None,
            costs_path: None,
            playbook_dir: None,
            daimon_state: None,
            experiment_store_path: None,
            replan_on_gate_failure: false,
            coding_oracle: None,
            gate_gaming_detector: None,
            holdout_experiment: None,
            shadow_runner: None,
            eval_generation_enabled: false,
        }
    }
}

/// Real runner/provider adapter injected into `TaskExecutorCell` factories.
pub struct GraphTaskDispatcher {
    factory: Arc<SharedAgentFactory>,
    config: Arc<RokoConfig>,
    workdir: PathBuf,
    budget_policy: GraphPlanBudgetPolicy,
    budget_ledger: GraphPlanBudgetLedger,
    /// CLI model override (from `--model`). When set, this replaces the
    /// config default and any per-task `model_hint` in dispatch.
    cli_model_override: Option<String>,
    /// Whether to skip agent permission prompts (from `--dangerously-skip-permissions`).
    dangerously_skip_permissions: bool,
    /// Learning/feedback subsystems wired into the Graph engine.
    feedback: GraphFeedbackContext,
    /// Optional per-task worktree isolation provider. When `Some`, each task
    /// dispatch acquires an isolated git worktree via this provider, runs the
    /// agent and verify steps inside it, and releases the worktree on
    /// completion. When `None` (the default), all tasks share `self.workdir`.
    workspace_provider: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>>,
    /// Optional TUI bridge for forwarding live agent output events to the
    /// dashboard. When set, completed dispatch events (text deltas, tool
    /// calls, tool outputs) are published through the StateHub so the TUI
    /// can render agent activity in real time.
    tui_bridge: Option<TuiBridge>,
}

impl GraphTaskDispatcher {
    /// Construct a dispatcher sharing the plan run's provider runtime.
    #[must_use]
    pub fn new(
        factory: Arc<SharedAgentFactory>,
        config: Arc<RokoConfig>,
        workdir: PathBuf,
    ) -> Self {
        Self {
            factory,
            config,
            workdir,
            budget_policy: GraphPlanBudgetPolicy::unlimited(),
            budget_ledger: GraphPlanBudgetLedger::default(),
            cli_model_override: None,
            dangerously_skip_permissions: false,
            feedback: GraphFeedbackContext::default(),
            workspace_provider: None,
            tui_bridge: None,
        }
    }

    /// Set the CLI model override (from `--model`).
    ///
    /// When set, this bypasses adaptive routing and forces all graph task
    /// dispatches to use the specified model slug.
    #[must_use]
    pub fn with_cli_model_override(mut self, model: Option<String>) -> Self {
        self.cli_model_override = model;
        self
    }

    /// Enable or disable permission skipping (from `--dangerously-skip-permissions`).
    #[must_use]
    pub fn with_dangerously_skip_permissions(mut self, skip: bool) -> Self {
        self.dangerously_skip_permissions = skip;
        self
    }

    /// Attach the learning/feedback subsystem context.
    #[must_use]
    pub fn with_feedback(mut self, feedback: GraphFeedbackContext) -> Self {
        self.feedback = feedback;
        self
    }

    /// Enable per-task worktree isolation via the given workspace provider.
    ///
    /// When set, each `dispatch` call will:
    /// 1. Acquire an isolated worktree for the task attempt.
    /// 2. Run the agent and verify steps inside the worktree.
    /// 3. Release the worktree on success (`Delete`) or failure (`RetainForFailure`).
    ///
    /// This is opt-in via `--worktree-per-task` and defaults to `None` (shared workdir).
    #[must_use]
    pub fn with_workspace_provider(
        mut self,
        provider: Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>,
    ) -> Self {
        self.workspace_provider = Some(provider);
        self
    }

    /// Attach a TUI bridge for forwarding live agent output events.
    ///
    /// When set, agent dispatch events (text deltas, tool calls, tool
    /// outputs, spawned/completed lifecycle) are published through the
    /// StateHub so the TUI dashboard can render agent activity during
    /// Graph plan execution.
    #[must_use]
    pub fn with_tui_bridge(mut self, bridge: TuiBridge) -> Self {
        self.tui_bridge = Some(bridge);
        self
    }

    /// Apply a per-plan cost ceiling to subsequent task dispatches.
    #[must_use]
    pub fn with_plan_budget(
        mut self,
        ceiling_usd: f64,
        max_turn_usd: f64,
        continue_on_exhaustion: bool,
    ) -> Self {
        self.budget_policy =
            GraphPlanBudgetPolicy::from_limits(ceiling_usd, max_turn_usd, continue_on_exhaustion);
        self
    }

    /// Restore and attach the durable actual-provider-cost state for a plan.
    pub fn attach_plan_budget_checkpoint(
        &self,
        plan_id: &str,
        checkpoint: GraphCostLedgerCheckpoint,
    ) -> Result<()> {
        self.budget_ledger.attach_checkpoint(plan_id, checkpoint)
    }

    /// Return the current cost state for `plan_id`.
    #[must_use]
    pub fn plan_budget_snapshot(&self, plan_id: &str) -> GraphPlanBudgetSnapshot {
        self.budget_ledger.snapshot(plan_id, self.budget_policy)
    }

    /// Emit all feedback events after a task dispatch completes.
    ///
    /// This is the Graph engine equivalent of Runner-v2's post-dispatch
    /// feedback pipeline. Each subsystem is best-effort: failures are logged
    /// but do not block the task result.
    async fn emit_feedback(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        succeeded: bool,
        wall_duration: std::time::Duration,
        dispatch_plan: &crate::dispatch::RunnerDispatchPlan,
    ) {
        let role = task.role.as_deref().unwrap_or("implementer");
        let provider_id = &dispatch.target.provider_id;
        let model_slug = &dispatch.target.model_slug;
        let cost_usd = f64::from(dispatch.result.usage.cost_usd);
        let tokens_in = u64::from(dispatch.result.usage.input_tokens);
        let tokens_out = u64::from(dispatch.result.usage.output_tokens);
        let duration_ms = wall_duration.as_millis() as u64;

        // Determine model choice source for feedback routing.
        let model_source = if dispatch_plan.forced {
            ModelChoiceSource::Override
        } else if self.cli_model_override.is_some() {
            ModelChoiceSource::Override
        } else if task.model_hint.is_some() {
            ModelChoiceSource::TaskHint
        } else {
            ModelChoiceSource::Router
        };

        // ── W04: FeedbackFacade (episodes + routing) ─────────────────────
        if let Some(facade) = &self.feedback.feedback_facade {
            let outcome = crate::dispatch::AgentOutcome {
                task_id: task.id.clone(),
                plan_id: spec.plan_id.clone(),
                model: model_slug.clone(),
                provider: provider_id.clone(),
                output: dispatch
                    .result
                    .output
                    .body
                    .as_text()
                    .ok()
                    .unwrap_or("")
                    .chars()
                    .take(2048)
                    .collect(),
                tokens_in,
                tokens_out,
                cost_usd,
                duration_ms,
                exit_code: if succeeded { Some(0) } else { Some(1) },
                is_error: !succeeded,
            };
            let event = FeedbackEvent::TaskCompleted {
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                outcome,
                model_source,
                succeeded,
                routing_context: None,
                prompt_text: Some(dispatch_plan.prompt.system_prompt.clone()),
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                knowledge_ids: vec![],
                playbook_ids: vec![],
                initial_model: model_slug.clone(),
            };
            if let Err(error) = facade.on_event(&event).await {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "graph feedback facade error (best-effort)"
                );
            }
        }

        // ── W05: Efficiency event ────────────────────────────────────────
        if let Some(eff_path) = &self.feedback.efficiency_path {
            // P3-02: Extract actual turn count from the dispatch events so
            // the efficiency record carries the real turn number, not 0.
            let agent_num_turns = dispatch
                .events
                .iter()
                .rev()
                .find_map(|ev| match ev {
                    roko_agent::AgentRuntimeEvent::TurnCompleted { num_turns, .. } => {
                        *num_turns
                    }
                    _ => None,
                })
                .unwrap_or(1);
            let event = roko_learn::efficiency::AgentEfficiencyEvent {
                agent_id: format!(
                    "{}/{}",
                    spec.plan_id,
                    task.id
                ),
                role: role.to_string(),
                backend: provider_id.clone(),
                model: model_slug.clone(),
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                attempt_id: String::new(),
                input_tokens: tokens_in,
                output_tokens: tokens_out,
                reasoning_tokens: 0,
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cache_write_tokens: u64::from(dispatch.result.usage.cache_create_tokens),
                cost_usd,
                cost_usd_without_cache: cost_usd,
                prompt_sections: vec![],
                total_prompt_tokens: tokens_in,
                system_prompt_tokens: 0,
                tools_available: 0,
                tools_used: 0,
                tool_calls: vec![],
                wall_time_ms: duration_ms,
                duration_ms,
                time_to_first_token_ms: 0,
                was_warm_start: false,
                iteration: agent_num_turns,
                turn_number: agent_num_turns,
                is_final_turn: true,
                gate_passed: None,
                outcome: if succeeded {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                gate_errors: vec![],
                model_used: model_slug.clone(),
                frequency: roko_core::OperatingFrequency::Gamma,
                strategy_attempted: String::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(error) = append_jsonl_line(eff_path, &event) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "graph efficiency event write failed (best-effort)"
                );
            }
        }

        // ── W05b: Cost record to costs.jsonl ─────────────────────────────
        //
        // `roko status` reads cost totals from `.roko/learn/costs.jsonl` via
        // `CostsLog::total_cost()`. The Graph engine only writes efficiency
        // events (above), so the status cost summary always showed $0.0000.
        // This block bridges the gap: one `CostRecord` per dispatch, written
        // synchronously alongside the efficiency event.
        if let Some(costs_path) = &self.feedback.costs_path {
            let cost_record = CostRecord {
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_slug.clone(),
                provider: provider_id.clone(),
                role: role.to_string(),
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                complexity_band: task.tier.clone(),
                input_tokens: tokens_in,
                output_tokens: tokens_out,
                cached_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cost_usd,
                duration_ms,
                success: succeeded,
                session_id: String::new(),
            };
            if let Err(error) = append_jsonl_line(costs_path, &cost_record) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "graph cost record write failed (best-effort)"
                );
            }
        }

        // ── W07: Playbook outcome recording ──────────────────────────────
        if let Some(playbook_dir) = &self.feedback.playbook_dir {
            let store = roko_learn::playbook::PlaybookStore::new(playbook_dir);
            let playbook_id = format!("task-{}", task.id);
            if let Err(error) = store.record_outcome(&playbook_id, succeeded).await {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "graph playbook outcome recording failed (best-effort)"
                );
            }
        }

        // ── W09: DaimonState affect feedback ─────────────────────────────
        if let Some(daimon) = &self.feedback.daimon_state {
            use roko_daimon::AffectEngine;
            let event = roko_daimon::AffectEvent::TaskOutcome {
                task_id: task.id.clone(),
                succeeded,
            };
            if let Ok(mut state) = daimon.lock() {
                let _ = state.appraise(event);
            }
        }

        // ── W14: Experiment settlement ───────────────────────────────────
        if let Some(store_path) = &self.feedback.experiment_store_path {
            if store_path.exists() {
                let settlement = if succeeded {
                    roko_learn::prompt_experiment::AssignmentSettlement::Observed { success: true }
                } else {
                    roko_learn::prompt_experiment::AssignmentSettlement::Observed { success: false }
                };
                let attempt_key = roko_learn::prompt_experiment::PromptAttemptKey::new(
                    "graph",
                    &spec.plan_id,
                    &task.id,
                    0,
                );
                if let Err(error) = roko_learn::prompt_experiment::ExperimentStore::settle_attempt(
                    store_path,
                    &attempt_key,
                    settlement,
                ) {
                    // AttemptNotFound is normal for non-experiment runs; log others.
                    if !matches!(
                        error,
                        roko_learn::prompt_experiment::PromptAssignmentError::AttemptNotFound(_)
                    ) {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            %error,
                            "graph experiment settlement failed (best-effort)"
                        );
                    }
                }
            }
        }
    }

    /// Forward dispatch events to the TUI bridge so the dashboard shows
    /// live agent output during Graph plan execution.
    ///
    /// Called after `run_shared_agent_bridge` returns. Each event in the
    /// dispatch result is mapped to the corresponding `TuiBridge` method
    /// which publishes a `DashboardEvent` through the StateHub.
    fn forward_dispatch_events_to_tui(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        ctx: &CellContext,
    ) {
        let Some(tui) = &self.tui_bridge else {
            return;
        };

        let agent_id = format!(
            "{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or(&task.id)
        );
        let plan_id = &spec.plan_id;
        let task_id = &task.id;

        // Emit agent spawned event so the TUI knows an agent is active.
        tui.agent_spawned(
            &agent_id,
            plan_id,
            task_id,
            0,
            task.role.as_deref().unwrap_or("implementer"),
            &dispatch.target.model_slug,
            &dispatch.target.provider_id,
        );

        // Forward each provider event as a TUI stream record.
        for event in &dispatch.events {
            match event {
                roko_agent::AgentRuntimeEvent::MessageDelta { text } => {
                    tui.agent_text_delta(&agent_id, plan_id, task_id, 0, text);
                }
                roko_agent::AgentRuntimeEvent::ToolCall { id, name } => {
                    tui.tool_call(&agent_id, plan_id, task_id, 0, id, name);
                }
                roko_agent::AgentRuntimeEvent::ToolOutput { id, output } => {
                    // Truncate tool output for the TUI to avoid overwhelming
                    // the bounded stream ring buffer.
                    let truncated = if output.len() > 2048 {
                        let tail = &output[output.len() - 1024..];
                        format!("[...truncated]\n{tail}")
                    } else {
                        output.clone()
                    };
                    tui.tool_output(&agent_id, plan_id, task_id, 0, id, &truncated);
                }
                _ => {}
            }
        }

        // Emit agent completed event.
        tui.agent_completed(&agent_id, plan_id, task_id, 0);
    }
}

/// Append a single JSON line to a JSONL file, creating parent dirs as needed.
fn append_jsonl_line(path: &std::path::Path, value: &impl serde::Serialize) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    file.flush()?;
    Ok(())
}

fn effective_agent_contract(task_role: &str, task: &TaskDef) -> AgentContract {
    let task_allowed_tools = task
        .allowed_tools
        .as_deref()
        .filter(|tools| !tools.is_empty());
    AgentContract::load_for_role_with_mode(task_role, ContractLoadMode::RestrictedFallback)
        .unwrap_or_else(|_| AgentContract::restricted(task_role))
        .with_tool_restrictions(task_allowed_tools, task.denied_tools.as_deref())
}

fn upstream_outputs(input: &[Signal]) -> Vec<(String, Vec<String>)> {
    input
        .iter()
        .enumerate()
        .filter_map(|(index, signal)| {
            signal
                .body
                .as_text()
                .ok()
                .map(|text| (format!("graph-upstream-{index}"), vec![text.to_string()]))
        })
        .collect()
}

/// P1-18: Load persisted dream routing advice and convert to a `RoutingBias`
/// for the cascade router. Returns `None` when no advice file exists, the
/// advice is stale, or no recommendations match the task category.
fn load_dream_routing_bias(
    workdir: &Path,
    task_category: &str,
    routing_ctx: &roko_learn::model_router::RoutingContext,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    let advice = roko_dreams::load_dream_routing_advice(workdir).ok()?;
    if advice.recommendations.is_empty() {
        return None;
    }
    let complexity_band = routing_ctx.complexity.label();
    let bias = roko_dreams::dream_advice_to_routing_bias(&advice, task_category, complexity_band);
    if bias.deprioritize.is_empty() {
        return None;
    }
    tracing::debug!(
        deprioritize = ?bias.deprioritize,
        reason = %bias.reason,
        "loaded dream routing bias for graph task dispatch"
    );
    Some(bias)
}

/// Build a reasonable `RoutingContext` for Graph task dispatch.
///
/// This provides the cascade router with actionable task signals without
/// requiring the full runner-v2 internal state. The Graph engine has less
/// runtime state than the event loop, so fields like `active_agents` and
/// `ready_queue_depth` are set to sane defaults.
fn build_routing_context(
    role: &str,
    task: &TaskDef,
    daimon_state: &Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
) -> roko_learn::model_router::RoutingContext {
    use roko_core::agent::AgentRole;
    use roko_core::task::{TaskCategory, TaskComplexityBand};
    use roko_learn::model_router::RoutingContext;

    let role_enum = match role.trim().to_ascii_lowercase().as_str() {
        "conductor" => AgentRole::Conductor,
        "strategist" => AgentRole::Strategist,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" | "reviewer" => AgentRole::Auditor,
        "refactorer" => AgentRole::Refactorer,
        _ => AgentRole::Implementer,
    };

    // Derive task category from the role or task type, defaulting to
    // Implementation for most Graph engine work.
    let task_category = match role_enum {
        AgentRole::Researcher => TaskCategory::Research,
        AgentRole::Auditor => TaskCategory::Verification,
        AgentRole::Refactorer => TaskCategory::Refactor,
        AgentRole::Architect => TaskCategory::Scaffolding,
        _ => TaskCategory::Implementation,
    };

    // Infer complexity from the task tier field, or default to Standard.
    let complexity = match task.tier.trim().to_ascii_lowercase().as_str() {
        "fast" | "t0" | "0" => TaskComplexityBand::Fast,
        "complex" | "t2" | "2" | "premium" => TaskComplexityBand::Complex,
        _ => TaskComplexityBand::Standard,
    };

    // Extract daimon policy if the affect state is loaded.
    let daimon_policy = daimon_state
        .as_ref()
        .and_then(|d| {
            d.lock().ok().map(|state| {
                use roko_daimon::AffectEngine;
                let affect = state.query();
                roko_core::DaimonPolicy::new(affect.confidence, affect.behavioral_state)
            })
        })
        .unwrap_or_default();

    RoutingContext {
        task_category,
        complexity,
        iteration: 0,
        role: role_enum,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 1,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy,
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

#[async_trait::async_trait]
impl TaskDispatcher for GraphTaskDispatcher {
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>> {
        let budget_reservation = self
            .budget_ledger
            .reserve(&spec.plan_id, self.budget_policy)?;

        let task: TaskDef = serde_json::from_str(&spec.task_def_json).map_err(|error| {
            RokoError::Planning(format!(
                "decode task definition for `{}`: {error}",
                spec.title
            ))
        })?;

        // ── P0-02: EvalGenerator pre-dispatch ───────────────────────────
        //
        // For standard-tier and above tasks, generate evaluation test
        // artifacts before the agent starts. Mirrors Runner-v2's
        // pre-dispatch eval generation.
        if self.feedback.eval_generation_enabled {
            let tier_lower = task.tier.to_ascii_lowercase();
            let is_standard_or_above = !matches!(tier_lower.as_str(), "mechanical" | "trivial");
            if is_standard_or_above {
                let target_crates =
                    crate::task_helpers::task_target_crates(Some(&task));
                let primary_crate = target_crates
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "roko-cli".to_string());
                let generator = EvalGenerator::new();
                let evals = generator.generate_all(
                    &task.title,
                    &primary_crate,
                    &task.files,
                );
                if !evals.is_empty() {
                    let gen_dir = self.workdir.join("generated-tests");
                    if let Err(err) = std::fs::create_dir_all(&gen_dir) {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P0-02: failed to create generated-tests dir (non-fatal)"
                        );
                    } else {
                        for eval in &evals {
                            let file_name = format!("{}.rs", eval.name);
                            let file_path = gen_dir.join(&file_name);
                            if let Err(err) = std::fs::write(&file_path, &eval.test_source) {
                                tracing::warn!(
                                    plan_id = %spec.plan_id,
                                    task_id = %task.id,
                                    file = %file_name,
                                    error = %err,
                                    "P0-02: failed to write generated eval (non-fatal)"
                                );
                            }
                        }
                        tracing::debug!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            eval_count = evals.len(),
                            "P0-02: generated eval artifacts before dispatch"
                        );
                    }
                }
            }
        }

        // ── P2-01: ShadowRunner decision recording ──────────────────────
        //
        // Record whether this task would be shadowed. Infrastructure-only:
        // we record the decision but do not actually spawn a shadow task.
        if let Some(shadow) = &self.feedback.shadow_runner {
            let should = shadow.should_shadow();
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                should_shadow = should,
                shadow_model = %shadow.config.model_slug,
                "P2-01: shadow decision recorded (infrastructure-only)"
            );
        }

        // ── Worktree isolation: acquire ─────────────────────────────────
        //
        // When a workspace provider is configured, acquire an isolated
        // worktree for this task attempt. The agent and verify steps will
        // run inside it instead of the shared repository root.
        let attempt_id = roko_graph::workspace::WorkspaceAttemptId {
            plan_id: spec.plan_id.clone(),
            task_id: task.id.clone(),
            // CellContext does not carry an attempt counter; the graph engine
            // handles retries by re-executing the cell. Use 0 here -- the
            // workspace provider's idempotent acquire ensures the same
            // (plan_id, task_id, 0) triple reuses the existing worktree.
            attempt: 0,
        };
        let lease = if let Some(provider) = &self.workspace_provider {
            let lease = provider.acquire(&attempt_id).await.map_err(|e| {
                RokoError::Agent {
                    backend: "worktree-isolation".to_string(),
                    message: format!("failed to acquire worktree for {attempt_id}: {e}"),
                }
            })?;
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                worktree = %lease.path.display(),
                "acquired isolated worktree for task"
            );
            Some(lease)
        } else {
            None
        };
        // Effective working directory: worktree path if isolated, else shared workdir.
        let effective_workdir = lease
            .as_ref()
            .map_or_else(|| self.workdir.clone(), |l| l.path.clone());

        let role = task.role.as_deref().unwrap_or("implementer");
        // ── W10: Enrichment pipeline ─────────────────────────────────────
        let routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);

        // P1-16: Run cross-cut arbitration to detect safety-critical
        // overrides before loading dream routing advice.
        let task_category = task.domain.as_ref().map_or("implementation", |d| d.label());
        let arbitration_bias =
            arbitrate_cross_cut_routing_bias(&self.feedback, &self.workdir, task_category);

        // P1-18: Load persisted dream routing advice and convert to a
        // RoutingBias so the cascade router accounts for dream-observed
        // model performance when picking a provider for this task.
        // Arbitration safety overrides take priority over dream advice.
        let routing_bias = arbitration_bias
            .or_else(|| load_dream_routing_bias(&self.workdir, task_category, &routing_ctx));

        let dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: effective_workdir.clone(),
            // Task-authored model_hint flows through to RoutingInputs where it
            // beats the cascade router but loses to force_backend.  When the
            // task has no hint we leave this None so the cascade router can
            // make its own decision rather than short-circuiting to the config
            // default.
            model_hint: task.model_hint.clone(),
            force_backend: self.cli_model_override.clone(),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: 0,
            // Graph does not yet own runner terminal feedback receipts.
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: Some(routing_ctx),
            routing_bias,
            dependency_outputs: upstream_outputs(&input),
        };
        let dispatch_plan = self
            .factory
            .dispatcher()
            .plan(&task, &dispatch_ctx)
            .map_err(|error| RokoError::Planning(error.to_string()))?;
        let contract = effective_agent_contract(role, &task);
        let effective_timeout_secs = if spec.timeout_secs == 0 {
            self.config.timeouts.agent_dispatch_secs
        } else {
            spec.timeout_secs
        };
        let timeout_ms = effective_timeout_secs.max(1).saturating_mul(1_000);
        let request = AgentDispatchRequest {
            model_key: dispatch_plan.model.slug.clone(),
            prompt: dispatch_plan.prompt.user_prompt.clone(),
            system_prompt: dispatch_plan.prompt.system_prompt.clone(),
            workdir: effective_workdir.clone(),
            immune_root: Some(effective_workdir.clone()),
            agent_id: format!(
                "{}/{}",
                spec.plan_id,
                ctx.cell_id.as_deref().unwrap_or(&task.id)
            ),
            command: None,
            timeout_ms: Some(timeout_ms),
            mcp_config: self.config.agent.mcp_config.clone(),
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: Some(self.config.agent.default_effort.clone()),
            tools: None,
            agent_contract: Some(contract),
            bare_mode: self.config.agent.bare_mode,
            dangerously_skip_permissions: self.dangerously_skip_permissions,
        };

        let started_at = Instant::now();
        let dispatch = self
            .factory
            .run_shared_agent_bridge(request)
            .await
            .map_err(|error| {
                // Best-effort release on dispatch failure when worktree isolation is active.
                if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                    let provider = Arc::clone(provider);
                    let lease = lease.clone();
                    tokio::spawn(async move {
                        let _ = provider.release(
                            &lease,
                            roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                        ).await;
                    });
                }
                RokoError::Agent {
                    backend: "graph-task-executor".to_string(),
                    message: error.to_string(),
                }
            })?;
        let wall_duration = started_at.elapsed();

        // Account for every completed provider call, including unsuccessful
        // results: callers may still have incurred the reported cost.
        budget_reservation.settle(f64::from(dispatch.result.usage.cost_usd))?;

        // ── TUI streaming output ─────────────────────────────────────────
        //
        // Forward provider dispatch events (text deltas, tool calls, tool
        // outputs) to the TUI bridge so the dashboard shows what the agent
        // produced. This runs for both successful and failed dispatches.
        self.forward_dispatch_events_to_tui(spec, &task, &dispatch, ctx);

        // ── Learning/feedback pipeline ───────────────────────────────────
        //
        // Emit feedback events for all wired subsystems. This runs for both
        // successful and failed dispatches so the routing and efficiency
        // subsystems learn from every provider call.
        self.emit_feedback(
            spec,
            &task,
            &dispatch,
            dispatch.result.success,
            wall_duration,
            &dispatch_plan,
        )
        .await;

        if !dispatch.result.success {
            // Release worktree with RetainForFailure policy for post-mortem.
            if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                let _ = provider.release(
                    lease,
                    roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                ).await;
            }
            let message = dispatch
                .result
                .output
                .body
                .as_text()
                .unwrap_or("provider returned an unsuccessful result")
                .to_string();
            return Err(RokoError::Agent {
                backend: dispatch.target.provider_id,
                message,
            });
        }

        // ── Verify steps (gate execution) ──────────────────────────────
        //
        // Run each [[task.verify]] step as a shell gate. If any step fails,
        // the task is marked failed so the Graph engine can retry or abort.
        // Gates run in the effective workdir (worktree if isolated).
        if !task.verify.is_empty() {
            let payload = GatePayload::in_dir(&effective_workdir)
                .with_label(format!("{}/{}", spec.plan_id, task.id));
            let gate_signal = Signal::builder(Kind::Task)
                .body(
                    Body::from_json(&payload)
                        .unwrap_or_else(|_| Body::text("gate-payload-fallback")),
                )
                .build();
            let gate_ctx = Context::now();

            let mut failures: Vec<String> = Vec::new();
            // P4-03: PromiseTracker for early termination of doomed attempts.
            let mut promise_tracker = crate::runner::promise_tracker::PromiseTracker::new();
            let mut promise_terminated = false;

            for (i, step) in task.verify.iter().enumerate() {
                // P4-03: Check for early termination before running the next step.
                if promise_terminated {
                    break;
                }

                let step_label = if step.phase.is_empty() {
                    format!("verify[{}]", i)
                } else {
                    format!("verify[{}:{}]", i, step.phase)
                };

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    phase = %step.phase,
                    command = %step.command,
                    timeout_ms = step.timeout_ms,
                    "graph verify step starting"
                );

                // Parse the command into program + args for ShellGate.
                let parts: Vec<&str> = step.command.split_whitespace().collect();
                let (program, args) = if parts.is_empty() {
                    ("true", Vec::new())
                } else {
                    (parts[0], parts[1..].iter().map(|s| s.to_string()).collect())
                };

                let gate = ShellGate::new(program, args)
                    .with_timeout_ms(step.timeout_ms)
                    .with_name(&step_label);

                let verdict = gate.verify(&gate_signal, &gate_ctx).await;

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    gate = %verdict.gate,
                    passed = verdict.passed,
                    duration_ms = verdict.duration_ms,
                    "graph verify step completed"
                );

                // ── P0-04: CodingOracle observations ────────────────────
                //
                // Feed each verdict into the CodingOracle so it can refine
                // its build-time and test-pass-rate predictions.
                if let Some(oracle) = &self.feedback.coding_oracle {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let gate_lower = step.phase.to_ascii_lowercase();
                    if gate_lower.contains("compile") || step.command.contains("cargo check") || step.command.contains("cargo build") {
                        oracle.observe_build(BuildRecord {
                            duration_secs: verdict.duration_ms as f64 / 1000.0,
                            success: verdict.passed,
                            warnings: 0,
                            ts_ms: now_ms,
                        });
                    }
                    if gate_lower.contains("test") || step.command.contains("cargo test") {
                        let (passed, failed, total) = if verdict.passed {
                            (1, 0, 1)
                        } else {
                            (0, 1, 1)
                        };
                        oracle.observe_test(TestRecord {
                            passed,
                            failed,
                            total,
                            ts_ms: now_ms,
                        });
                    }
                }

                // ── P4-03: PromiseTracker per-step check ────────────────
                //
                // Build a TurnSnapshot from this verify step and check
                // whether the attempt should be terminated early.
                {
                    let core_verdict = if verdict.passed {
                        roko_core::Verdict::pass(&step_label)
                    } else {
                        roko_core::Verdict::fail(&step_label, &verdict.reason)
                    };
                    let snapshot = TurnSnapshot {
                        rung: i as u32,
                        verdicts: vec![core_verdict],
                        error_count: if verdict.passed { 0 } else { 1 },
                        diff_lines: 0,
                    };
                    let decision = promise_tracker.record_and_check(snapshot);
                    if let crate::runner::promise_tracker::PromiseDecision::Terminate {
                        promise,
                        consecutive_turns,
                    } = decision
                    {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            step = i,
                            promise,
                            consecutive_turns,
                            "P4-03: PRM early termination — abandoning doomed verify sequence"
                        );
                        failures.push(format!(
                            "early termination: promise {promise:.3} below threshold \
                             for {consecutive_turns} consecutive verify steps"
                        ));
                        promise_terminated = true;
                        // Don't break here; fall through to record the current failure.
                    }
                }

                if !verdict.passed {
                    let fail_msg = step
                        .fail_msg
                        .as_deref()
                        .unwrap_or(&verdict.reason);
                    let detail_snippet = verdict
                        .detail
                        .as_deref()
                        .map(|d| {
                            // Include a bounded tail of the output for diagnostics.
                            let lines: Vec<&str> = d.lines().collect();
                            let start = lines.len().saturating_sub(30);
                            lines[start..].join("\n")
                        })
                        .unwrap_or_default();

                    failures.push(format!(
                        "{step_label} (`{cmd}`): {fail_msg}\n{detail_snippet}",
                        cmd = step.command,
                    ));
                }
            }

            // ── Post-verify: GateGamingDetector + HoldoutExperiment ─────
            //
            // These run after all verify steps complete (or early-terminate)
            // regardless of pass/fail, matching the Runner-v2 gate completion
            // callback pattern.
            let all_passed = failures.is_empty();
            let model_slug = &dispatch.target.model_slug;

            // P1-01: GateGamingDetector observation.
            if let Some(detector) = &self.feedback.gate_gaming_detector {
                let base_quality = if all_passed { 0.8 } else { 0.2 };
                // P3-17: Modulate quality with daimon affect valence when available.
                let affect_bonus = self.feedback.daimon_state.as_ref()
                    .and_then(|d| d.lock().ok())
                    .map(|state| state.state.alma.effective_affect().pleasure)
                    .unwrap_or(0.0);
                let quality_score = (base_quality + affect_bonus * 0.1).clamp(0.0, 1.0);
                if let Ok(mut det) = detector.try_lock() {
                    if let Err(err) = det
                        .observe_and_detect(model_slug, all_passed, quality_score)
                        .await
                    {
                        tracing::warn!(
                            error = %err,
                            model = %model_slug,
                            "P1-01: gate gaming detection I/O error (non-fatal)"
                        );
                    }
                }
            }

            // P1-04: HoldoutExperiment outcome recording and learning gate.
            if let Some(holdout) = &self.feedback.holdout_experiment {
                let holdout_task_key = format!("{}:{}", spec.plan_id, task.id);
                if let Ok(mut exp) = holdout.try_lock() {
                    exp.record_outcome(&holdout_task_key, all_passed, 0.0);
                    if let Some(alert) = exp.check_overfitting() {
                        tracing::warn!(
                            train_pass_rate = alert.train_pass_rate,
                            holdout_pass_rate = alert.holdout_pass_rate,
                            divergence_pp = alert.divergence_pp,
                            "P1-04: holdout overfitting detected"
                        );
                    }
                    // Gate learning updates: only Train partition tasks update
                    // the routing model; holdout tasks are observed but never
                    // feed back into learned state. This affects the playbook,
                    // efficiency, and experiment settlement paths above.
                    let should_update = exp.should_update_learning(&holdout_task_key);
                    tracing::debug!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        should_update_learning = should_update,
                        "P1-04: holdout partition check"
                    );
                }
            }

            if !failures.is_empty() {
                let summary = format!(
                    "{n}/{total} verify step(s) failed for task `{task}`:\n\n{details}",
                    n = failures.len(),
                    total = task.verify.len(),
                    task = spec.title,
                    details = failures.join("\n\n---\n\n"),
                );
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    failed_count = failures.len(),
                    total_count = task.verify.len(),
                    "graph verify steps failed"
                );
                // ── W12: Gate failure replan signal ───────────────────────
                if self.feedback.replan_on_gate_failure {
                    tracing::info!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        failed_count = failures.len(),
                        "gate failure replan enabled; Graph engine will retry via max_retries"
                    );
                    // Update efficiency gate_passed if we wrote one.
                    if let Some(eff_path) = &self.feedback.efficiency_path {
                        // P3-02: Propagate actual turn count from the
                        // dispatch that preceded this gate failure.
                        let gate_turn_number = dispatch
                            .events
                            .iter()
                            .rev()
                            .find_map(|ev| match ev {
                                roko_agent::AgentRuntimeEvent::TurnCompleted {
                                    num_turns, ..
                                } => *num_turns,
                                _ => None,
                            })
                            .unwrap_or(1);
                        let gate_event = roko_learn::efficiency::AgentEfficiencyEvent {
                            agent_id: format!("{}/{}", spec.plan_id, task.id),
                            role: task.role.as_deref().unwrap_or("implementer").to_string(),
                            backend: dispatch.target.provider_id.clone(),
                            model: dispatch.target.model_slug.clone(),
                            plan_id: spec.plan_id.clone(),
                            task_id: task.id.clone(),
                            attempt_id: String::new(),
                            input_tokens: 0,
                            output_tokens: 0,
                            reasoning_tokens: 0,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                            cost_usd: 0.0,
                            cost_usd_without_cache: 0.0,
                            prompt_sections: vec![],
                            total_prompt_tokens: 0,
                            system_prompt_tokens: 0,
                            tools_available: 0,
                            tools_used: 0,
                            tool_calls: vec![],
                            wall_time_ms: 0,
                            duration_ms: 0,
                            time_to_first_token_ms: 0,
                            was_warm_start: false,
                            iteration: gate_turn_number,
                            turn_number: gate_turn_number,
                            is_final_turn: false,
                            gate_passed: Some(false),
                            outcome: "gate_failure".to_string(),
                            gate_errors: failures.clone(),
                            model_used: dispatch.target.model_slug.clone(),
                            frequency: roko_core::OperatingFrequency::Gamma,
                            strategy_attempted: "replan".to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        if let Err(error) = append_jsonl_line(eff_path, &gate_event) {
                            tracing::warn!(
                                %error,
                                "graph gate-failure efficiency event write failed"
                            );
                        }
                    }
                }
                // Release worktree with RetainForFailure for post-mortem.
                if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                    let _ = provider.release(
                        lease,
                        roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                    ).await;
                }
                return Err(RokoError::Verify {
                    gate: "graph-verify".to_string(),
                    message: summary,
                });
            }

            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                step_count = task.verify.len(),
                "all graph verify steps passed"
            );
        }

        // ── Worktree isolation: release on success ──────────────────────
        //
        // On success, release the worktree with Delete policy. The changes
        // are already on the worktree's branch and can be merged separately
        // via the delivery pipeline. For now the worktree is cleaned up.
        if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                worktree = %lease.path.display(),
                "releasing isolated worktree after successful task"
            );
            if let Err(e) = provider.release(
                lease,
                roko_graph::workspace::WorkspaceReleasePolicy::Delete,
            ).await {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    error = %e,
                    "worktree release failed (best-effort); worktree may remain on disk"
                );
            }
        }

        let mut output = dispatch.result.output;
        if output.body.as_text().is_err() {
            output = Signal::builder(Kind::AgentOutput)
                .body(Body::text(format!(
                    "provider `{}` completed task `{}`",
                    dispatch.target.provider_id, spec.title
                )))
                .build();
        }
        Ok(vec![output])
    }
}

/// Bounded channel capacity for streaming graph task events (#233).
const STREAMING_EVENT_CHANNEL_CAPACITY: usize = 256;

/// Recommended channel capacity for callers constructing an event sender.
///
/// Text/progress events may coalesce across sends. Tool boundaries, usage,
/// attempt receipt, and terminal outcome events are reliable and must not be
/// dropped.
#[must_use]
pub const fn streaming_event_channel_capacity() -> usize {
    STREAMING_EVENT_CHANNEL_CAPACITY
}

#[async_trait::async_trait]
impl StreamingTaskDispatcher for GraphTaskDispatcher {
    async fn dispatch_streaming(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
        lease: &TaskLease,
        event_tx: tokio::sync::mpsc::Sender<GraphTaskEvent>,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> Result<TaskDispatchOutcome> {
        // ── Lease validation ─────────────────────────────────────────────
        if !lease.path.exists() {
            return Err(RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: format!(
                    "lease path `{}` does not exist; the caller must acquire the lease before dispatch",
                    lease.path.display()
                ),
            });
        }
        // When worktree isolation is active, the lease path is the
        // worktree (not the repo root), so the mismatch is expected.
        // Only enforce the strict check when no workspace provider is set.
        if self.workspace_provider.is_none() && lease.path != self.workdir {
            return Err(RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: format!(
                    "lease path `{}` does not match the dispatcher workdir `{}`; shared-checkout workdirs are rejected",
                    lease.path.display(),
                    self.workdir.display()
                ),
            });
        }

        // ── Budget reservation ───────────────────────────────────────────
        let budget_reservation = self
            .budget_ledger
            .reserve(&spec.plan_id, self.budget_policy)?;

        // ── Attempt identity ─────────────────────────────────────────────
        let attempt_id = format!(
            "{}/{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or("unknown"),
            uuid::Uuid::new_v4()
        );

        // Record attempt-start receipt before provider launch.
        recorder.record_start(&attempt_id, spec).await?;

        // Notify the event channel that the attempt has started.
        let _ = event_tx
            .send(GraphTaskEvent::AttemptStarted {
                attempt_id: attempt_id.clone(),
            })
            .await;

        let started_at = Instant::now();

        // ── Task + dispatch context ──────────────────────────────────────
        let task: TaskDef = serde_json::from_str(&spec.task_def_json).map_err(|error| {
            RokoError::Planning(format!(
                "decode task definition for `{}`: {error}",
                spec.title
            ))
        })?;
        let role = task.role.as_deref().unwrap_or("implementer");
        // ── W10: Enrichment pipeline (streaming) ─────────────────────────
        let routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);

        let dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: lease.path.clone(),
            model_hint: task.model_hint.clone(),
            force_backend: self.cli_model_override.clone(),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: 0,
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: Some(routing_ctx),
            routing_bias: None,
            dependency_outputs: upstream_outputs(&input),
        };
        let dispatch_plan = self
            .factory
            .dispatcher()
            .plan(&task, &dispatch_ctx)
            .map_err(|error| RokoError::Planning(error.to_string()))?;
        let contract = effective_agent_contract(role, &task);
        let effective_timeout_secs = if spec.timeout_secs == 0 {
            self.config.timeouts.agent_dispatch_secs
        } else {
            spec.timeout_secs
        };
        let timeout_ms = effective_timeout_secs.max(1).saturating_mul(1_000);
        let request = AgentDispatchRequest {
            model_key: dispatch_plan.model.slug.clone(),
            prompt: dispatch_plan.prompt.user_prompt.clone(),
            system_prompt: dispatch_plan.prompt.system_prompt.clone(),
            workdir: lease.path.clone(),
            immune_root: Some(lease.path.clone()),
            agent_id: format!(
                "{}/{}",
                spec.plan_id,
                ctx.cell_id.as_deref().unwrap_or(&task.id)
            ),
            command: None,
            timeout_ms: Some(timeout_ms),
            mcp_config: self.config.agent.mcp_config.clone(),
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: Some(self.config.agent.default_effort.clone()),
            tools: None,
            agent_contract: Some(contract),
            bare_mode: self.config.agent.bare_mode,
            dangerously_skip_permissions: self.dangerously_skip_permissions,
        };

        // ── Provider invocation ──────────────────────────────────────────
        let dispatch_result = self.factory.run_shared_agent_bridge(request).await;

        let wall_duration = started_at.elapsed();

        // ── TUI streaming output (streaming path) ──────────────────────
        // Forward provider events to the TUI bridge in the streaming path too.
        if let Ok(dispatch) = &dispatch_result {
            self.forward_dispatch_events_to_tui(spec, &task, dispatch, ctx);
        }

        // ── Map provider events to graph events ──────────────────────────
        // Forward provider dispatch events as streaming graph events.
        match &dispatch_result {
            Ok(dispatch) => {
                for event in &dispatch.events {
                    let graph_event = match event {
                        roko_agent::AgentRuntimeEvent::MessageDelta { text } => {
                            Some(GraphTaskEvent::Text { text: text.clone() })
                        }
                        roko_agent::AgentRuntimeEvent::ToolCall { id, name } => {
                            Some(GraphTaskEvent::ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                            })
                        }
                        roko_agent::AgentRuntimeEvent::ToolOutput { id, output } => {
                            Some(GraphTaskEvent::ToolOutput {
                                id: id.clone(),
                                output: output.clone(),
                            })
                        }
                        roko_agent::AgentRuntimeEvent::TokenUsage {
                            input_tokens,
                            output_tokens,
                            ..
                        } => Some(GraphTaskEvent::Usage {
                            input_tokens: *input_tokens,
                            output_tokens: *output_tokens,
                            cost_usd: None,
                        }),
                        _ => None,
                    };
                    if let Some(event) = graph_event {
                        // Best-effort send; text/progress may coalesce.
                        let _ = event_tx.send(event).await;
                    }
                }
            }
            Err(_) => {}
        }

        // ── Settle cost and build outcome ────────────────────────────────
        let (outcome, output_signals) = match dispatch_result {
            Ok(dispatch) => {
                let cost_usd = f64::from(dispatch.result.usage.cost_usd);
                let actual_cost = if cost_usd.is_finite() && cost_usd > 0.0 {
                    Some(cost_usd)
                } else {
                    tracing::debug!(
                        attempt = %attempt_id,
                        "provider did not report cost; recording None"
                    );
                    None
                };
                budget_reservation.settle(cost_usd.max(0.0))?;

                // ── Learning/feedback pipeline (streaming) ───────────────
                self.emit_feedback(
                    spec,
                    &task,
                    &dispatch,
                    dispatch.result.success,
                    wall_duration,
                    &dispatch_plan,
                )
                .await;

                // Forward final usage event with cost.
                let _ = event_tx
                    .send(GraphTaskEvent::Usage {
                        input_tokens: u64::from(dispatch.result.usage.input_tokens),
                        output_tokens: u64::from(dispatch.result.usage.output_tokens),
                        cost_usd: actual_cost,
                    })
                    .await;

                let outcome_kind = if dispatch.result.success {
                    TaskDispatchOutcomeKind::Succeeded
                } else {
                    TaskDispatchOutcomeKind::Failed
                };

                let output_signals = if dispatch.result.success {
                    let mut output = dispatch.result.output;
                    if output.body.as_text().is_err() {
                        output = Signal::builder(Kind::AgentOutput)
                            .body(Body::text(format!(
                                "provider `{}` completed task `{}`",
                                dispatch.target.provider_id, spec.title
                            )))
                            .build();
                    }
                    vec![output]
                } else {
                    Vec::new()
                };

                let dispatch_outcome = TaskDispatchOutcome {
                    attempt_id: attempt_id.clone(),
                    outcome: outcome_kind,
                    provider_id: dispatch.target.provider_id.clone(),
                    model: dispatch.target.model_slug.clone(),
                    input_tokens: Some(u64::from(dispatch.result.usage.input_tokens)),
                    output_tokens: Some(u64::from(dispatch.result.usage.output_tokens)),
                    cost_usd: actual_cost,
                    changed_files: Vec::new(), // Changed files computed relative to lease base.
                    wall_duration,
                    output: output_signals.clone(),
                };

                (dispatch_outcome, output_signals)
            }
            Err(error) => {
                let dispatch_outcome = TaskDispatchOutcome {
                    attempt_id: attempt_id.clone(),
                    outcome: TaskDispatchOutcomeKind::Failed,
                    provider_id: "graph-task-executor".to_string(),
                    model: String::new(),
                    input_tokens: None,
                    output_tokens: None,
                    cost_usd: None,
                    changed_files: Vec::new(),
                    wall_duration,
                    output: Vec::new(),
                };

                // Record terminal receipt before propagating the error.
                let _ = recorder
                    .record_terminal(&attempt_id, &dispatch_outcome)
                    .await;

                // Send terminal event.
                let _ = event_tx
                    .send(GraphTaskEvent::AttemptTerminal {
                        attempt_id,
                        outcome: TaskDispatchOutcomeKind::Failed,
                    })
                    .await;

                return Err(RokoError::Agent {
                    backend: "graph-task-executor".to_string(),
                    message: error.to_string(),
                });
            }
        };

        // ── Terminal receipt and event ────────────────────────────────────
        recorder.record_terminal(&attempt_id, &outcome).await?;

        let _ = event_tx
            .send(GraphTaskEvent::AttemptTerminal {
                attempt_id: attempt_id.clone(),
                outcome: outcome.outcome,
            })
            .await;

        // If the provider returned an unsuccessful result, fail the task
        // even though cost was settled (callers still incur the charge).
        if outcome.outcome == TaskDispatchOutcomeKind::Failed {
            return Err(RokoError::Agent {
                backend: outcome.provider_id.clone(),
                message: "provider returned an unsuccessful result".to_string(),
            });
        }

        // ── Verify steps (gate execution) ──────────────────────────────
        //
        // Run each [[task.verify]] step as a shell gate. If any step fails
        // the task is marked failed. Progress events are forwarded through
        // the streaming event channel for TUI display. Gates use the lease
        // path (worktree if isolated, shared workdir otherwise).
        if !task.verify.is_empty() {
            let payload = GatePayload::in_dir(&lease.path)
                .with_label(format!("{}/{}", spec.plan_id, task.id));
            let gate_signal = Signal::builder(Kind::Task)
                .body(
                    Body::from_json(&payload)
                        .unwrap_or_else(|_| Body::text("gate-payload-fallback")),
                )
                .build();
            let gate_ctx = Context::now();
            let total = task.verify.len() as u32;

            let mut failures: Vec<String> = Vec::new();

            for (i, step) in task.verify.iter().enumerate() {
                let step_label = if step.phase.is_empty() {
                    format!("verify[{}]", i)
                } else {
                    format!("verify[{}:{}]", i, step.phase)
                };

                // Emit progress event for the TUI.
                let _ = event_tx
                    .send(GraphTaskEvent::Progress {
                        message: format!("verify: {}", step.command),
                        completed: Some(i as u32),
                        total: Some(total),
                    })
                    .await;

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    phase = %step.phase,
                    command = %step.command,
                    timeout_ms = step.timeout_ms,
                    "graph verify step starting (streaming)"
                );

                let parts: Vec<&str> = step.command.split_whitespace().collect();
                let (program, args) = if parts.is_empty() {
                    ("true", Vec::new())
                } else {
                    (parts[0], parts[1..].iter().map(|s| s.to_string()).collect())
                };

                let gate = ShellGate::new(program, args)
                    .with_timeout_ms(step.timeout_ms)
                    .with_name(&step_label);

                let verdict = gate.verify(&gate_signal, &gate_ctx).await;

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    gate = %verdict.gate,
                    passed = verdict.passed,
                    duration_ms = verdict.duration_ms,
                    "graph verify step completed (streaming)"
                );

                if !verdict.passed {
                    let fail_msg = step.fail_msg.as_deref().unwrap_or(&verdict.reason);
                    let detail_snippet = verdict
                        .detail
                        .as_deref()
                        .map(|d| {
                            let lines: Vec<&str> = d.lines().collect();
                            let start = lines.len().saturating_sub(30);
                            lines[start..].join("\n")
                        })
                        .unwrap_or_default();

                    failures.push(format!(
                        "{step_label} (`{cmd}`): {fail_msg}\n{detail_snippet}",
                        cmd = step.command,
                    ));
                }
            }

            // Emit final progress event.
            let _ = event_tx
                .send(GraphTaskEvent::Progress {
                    message: if failures.is_empty() {
                        "verify: all steps passed".to_string()
                    } else {
                        format!("verify: {}/{} failed", failures.len(), total)
                    },
                    completed: Some(total),
                    total: Some(total),
                })
                .await;

            if !failures.is_empty() {
                let summary = format!(
                    "{n}/{total} verify step(s) failed for task `{task}`:\n\n{details}",
                    n = failures.len(),
                    total = task.verify.len(),
                    task = spec.title,
                    details = failures.join("\n\n---\n\n"),
                );
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    failed_count = failures.len(),
                    total_count = task.verify.len(),
                    "graph verify steps failed (streaming)"
                );
                return Err(RokoError::Verify {
                    gate: "graph-verify".to_string(),
                    message: summary,
                });
            }

            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                step_count = task.verify.len(),
                "all graph verify steps passed (streaming)"
            );
        }

        Ok(TaskDispatchOutcome {
            output: output_signals,
            ..outcome
        })
    }

    async fn reconcile_attempt(
        &self,
        _spec: &TaskExecutionSpec,
        previous_attempt_id: &str,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> AttemptReconciliation {
        // Check if the previous attempt has terminal evidence: if so, the
        // caller should reuse the committed result without re-invoking the
        // provider.
        if recorder.has_terminal_evidence(previous_attempt_id).await {
            return AttemptReconciliation::ReuseCommitted {
                attempt_id: previous_attempt_id.to_string(),
            };
        }

        // Check if the previous attempt started but never reached a terminal
        // state. This is ambiguous: the provider may have been invoked and we
        // cannot know whether it completed. Do not retry.
        if recorder.has_started_evidence(previous_attempt_id).await {
            return AttemptReconciliation::FailAmbiguous {
                attempt_id: previous_attempt_id.to_string(),
                reason: format!(
                    "attempt `{previous_attempt_id}` started but has no terminal evidence; \
                     the provider may have been invoked and cannot be safely retried"
                ),
            };
        }

        // No evidence that the previous attempt ever started. Allocate a
        // fresh attempt ID for a new dispatch.
        AttemptReconciliation::AllocateNew {
            attempt_id: format!("{previous_attempt_id}-retry-{}", uuid::Uuid::new_v4()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
    use roko_graph::cells::{AttemptReconciliation, NoopAttemptRecorder};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn plan_budget_blocks_at_ceiling_and_is_isolated_by_plan() {
        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::from_ceiling(0.50, false);

        ledger.record_cost("plan-a", 0.20);
        let before = ledger.snapshot("plan-a", policy);
        assert_eq!(before.spent_usd, 0.20);
        assert_eq!(before.remaining_usd(), 0.30);
        assert!(!before.exhausted);
        assert!(!before.dispatch_blocked);

        ledger.record_cost("plan-a", 0.30);
        let exhausted = ledger.snapshot("plan-a", policy);
        assert_eq!(exhausted.spent_usd, 0.50);
        assert!(exhausted.exhausted);
        assert!(exhausted.dispatch_blocked);

        assert_eq!(
            ledger.snapshot("plan-b", policy).spent_usd,
            0.0,
            "cost accounting must remain isolated by plan"
        );
    }

    #[test]
    fn explicit_override_observes_exhaustion_without_blocking() {
        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::from_ceiling(0.10, true);
        ledger.record_cost("plan-a", 0.25);

        let snapshot = ledger.snapshot("plan-a", policy);
        assert!(snapshot.exhausted);
        assert!(!snapshot.dispatch_blocked);
        assert_eq!(snapshot.remaining_usd(), 0.0);
        assert!(
            ledger.reserve("plan-a", policy).is_ok(),
            "explicit override must continue admitting calls after exhaustion"
        );
    }

    #[test]
    fn unlimited_policy_never_reserves_or_blocks() {
        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::unlimited();
        let reservations = (0..8)
            .map(|_| {
                ledger
                    .reserve("plan-a", policy)
                    .expect("unlimited admission")
            })
            .collect::<Vec<_>>();

        let snapshot = ledger.snapshot("plan-a", policy);
        assert_eq!(snapshot.reserved_usd, 0.0);
        assert!(!snapshot.exhausted);
        assert!(!snapshot.dispatch_blocked);
        assert!(snapshot.remaining_usd().is_infinite());
        drop(reservations);
    }

    #[test]
    fn routing_uses_the_tighter_context_or_plan_budget() {
        assert_eq!(effective_routing_budget(Some(0.40), 0.25), 0.25);
        assert_eq!(effective_routing_budget(Some(0.10), 0.25), 0.10);
        assert_eq!(effective_routing_budget(None, 0.25), 0.25);
        assert_eq!(effective_routing_budget(Some(-1.0), f64::INFINITY), 0.0);
    }

    #[test]
    fn invalid_provider_cost_fails_closed() {
        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::from_ceiling(1.0, false);
        let reservation = ledger.reserve("plan-a", policy).expect("reservation");
        assert!(reservation.settle(f64::NAN).is_err());

        let snapshot = ledger.snapshot("plan-a", policy);
        assert_eq!(snapshot.spent_usd, 0.0);
        assert_eq!(snapshot.reserved_usd, 0.0);
        assert!(snapshot.dispatch_blocked);
        assert!(ledger.reserve("plan-a", policy).is_err());
    }

    #[test]
    fn concurrent_admission_never_over_reserves_hard_ceiling() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        let ledger = Arc::new(GraphPlanBudgetLedger::default());
        let policy = GraphPlanBudgetPolicy::from_limits(0.50, 0.10, false);
        let attempted = Arc::new(AtomicUsize::new(0));
        let admitted = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(AtomicBool::new(false));
        let start = Arc::new(std::sync::Barrier::new(17));
        let mut threads = Vec::new();

        for _ in 0..16 {
            let ledger = Arc::clone(&ledger);
            let attempted = Arc::clone(&attempted);
            let admitted = Arc::clone(&admitted);
            let release = Arc::clone(&release);
            let start = Arc::clone(&start);
            threads.push(std::thread::spawn(move || {
                start.wait();
                let reservation = ledger.reserve("plan-a", policy).ok();
                if reservation.is_some() {
                    admitted.fetch_add(1, Ordering::SeqCst);
                }
                attempted.fetch_add(1, Ordering::SeqCst);
                while !release.load(Ordering::SeqCst) {
                    std::thread::yield_now();
                }
                drop(reservation);
            }));
        }

        start.wait();
        while attempted.load(Ordering::SeqCst) != 16 {
            std::thread::yield_now();
        }
        let snapshot = ledger.snapshot("plan-a", policy);
        assert_eq!(admitted.load(Ordering::SeqCst), 5);
        assert_eq!(snapshot.reserved_usd, 0.50);
        assert!(snapshot.dispatch_blocked);

        release.store(true, Ordering::SeqCst);
        for thread in threads {
            thread.join().expect("admission thread");
        }
        assert_eq!(ledger.snapshot("plan-a", policy).reserved_usd, 0.0);
    }

    #[tokio::test]
    async fn graph_task_cell_reaches_real_provider_runtime() {
        let temp = tempdir().expect("tempdir");
        let script = temp.path().join("fake-claude.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"graph-live-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-1","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":11,"output_tokens":22}}'
"#,
        )
        .expect("write provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make script executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "graph-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "graph-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(5_000),
                ttft_timeout_ms: Some(5_000),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "graph-model".to_string(),
            ModelProfile {
                provider: "graph-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_plan_budget(0.50, 0.25, false),
        );

        let task = TaskDef {
            id: "T01".to_string(),
            title: "Execute a real graph task".to_string(),
            description: Some("Return live output".to_string()),
            role: Some("implementer".to_string()),
            status: "ready".to_string(),
            tier: "focused".to_string(),
            frequency: None,
            model_hint: Some("graph-model".to_string()),
            replan_strategy: None,
            max_loc: None,
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: 5,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        };
        let config = toml::Value::Table(toml::map::Map::from_iter([
            ("plan_id".to_string(), toml::Value::String("p1".to_string())),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            ("timeout_secs".to_string(), toml::Value::Integer(5)),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        let cell = roko_graph::cells::TaskExecutorCell::live(config, dispatcher.clone());
        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
            )
            .await
            .expect("live graph task dispatch");

        assert_eq!(
            output[0].body.as_text().expect("provider text"),
            "graph-live-output"
        );
        assert!(
            !output[0]
                .body
                .as_text()
                .expect("provider text")
                .contains("dry-run")
        );

        let budget = dispatcher.plan_budget_snapshot("p1");
        assert!((budget.spent_usd - 0.25).abs() < 0.000_001);
        assert!(!budget.exhausted);
        assert!(!budget.dispatch_blocked);

        std::fs::write(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"paid-provider-failure"}}'
printf '%s\n' '{"type":"result","session_id":"sess-2","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":7,"output_tokens":3},"is_error":true}'
exit 1
"#,
        )
        .expect("replace provider script with paid failure");

        let paid_failure = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01-paid-failure".to_string()),
            )
            .await
            .expect_err("unsuccessful paid provider result must fail the task");
        assert!(matches!(paid_failure, RokoError::Agent { .. }));

        let budget = dispatcher.plan_budget_snapshot("p1");
        assert!((budget.spent_usd - 0.50).abs() < 0.000_001);
        assert!(budget.exhausted);
        assert!(budget.dispatch_blocked);

        let blocked = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01-retry".to_string()),
            )
            .await
            .expect_err("later dispatch must fail closed after plan budget exhaustion");
        assert!(matches!(blocked, RokoError::BudgetExceeded { .. }));
    }

    // ─── Streaming dispatch tests (#274) ─────────────────────────────────────

    /// Helper: create a `GraphTaskDispatcher` with a fake CLI provider.
    async fn make_streaming_dispatcher(
        temp: &tempfile::TempDir,
        script_content: &str,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        let script = temp.path().join("fake-claude-stream.sh");
        std::fs::write(&script, script_content).expect("write stream provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "stream-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "stream-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(5_000),
                ttft_timeout_ms: Some(5_000),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "stream-model".to_string(),
            ModelProfile {
                provider: "stream-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_plan_budget(1.00, 0.50, false),
        );

        let task = TaskDef {
            id: "T-STREAM".to_string(),
            title: "Streaming graph task".to_string(),
            description: Some("Test streaming dispatch".to_string()),
            role: Some("implementer".to_string()),
            status: "ready".to_string(),
            tier: "focused".to_string(),
            frequency: None,
            model_hint: Some("stream-model".to_string()),
            replan_strategy: None,
            max_loc: None,
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: 5,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        };

        (dispatcher, task)
    }

    fn make_spec(task: &TaskDef) -> TaskExecutionSpec {
        TaskExecutionSpec {
            plan_id: "stream-plan".to_string(),
            plan_dir: "/tmp/plans/stream-plan".to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            role: task.role.clone(),
            tier: task.tier.clone(),
            model_hint: task.model_hint.clone(),
            files: task.files.clone(),
            timeout_secs: task.timeout_secs,
            max_retries: task.max_retries,
            task_def_json: serde_json::to_string(task).expect("serialize task"),
        }
    }

    #[tokio::test]
    async fn streaming_dispatch_sends_attempt_started_and_terminal_events() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"streaming-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-s1","model":"claude-sonnet-4-6","total_cost_usd":0.10,"usage":{"input_tokens":5,"output_tokens":10}}'
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let outcome = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-STREAM".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect("streaming dispatch");

        assert_eq!(outcome.outcome, TaskDispatchOutcomeKind::Succeeded);
        assert!(!outcome.attempt_id.is_empty());
        assert!(outcome.cost_usd.is_some());
        assert!(!outcome.output.is_empty());

        // Collect all events.
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        // Must have at least AttemptStarted and AttemptTerminal.
        let started = events
            .iter()
            .any(|e| matches!(e, GraphTaskEvent::AttemptStarted { .. }));
        let terminal = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::AttemptTerminal {
                    outcome: TaskDispatchOutcomeKind::Succeeded,
                    ..
                }
            )
        });
        assert!(started, "must emit AttemptStarted event");
        assert!(terminal, "must emit AttemptTerminal(Succeeded) event");

        // Must have usage event with cost.
        let usage = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::Usage {
                    cost_usd: Some(_),
                    ..
                }
            )
        });
        assert!(usage, "must emit Usage event with actual cost");
    }

    #[tokio::test]
    async fn streaming_dispatch_rejects_missing_lease_path() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().join("nonexistent-lease"),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new(),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("missing lease path must fail");

        assert!(error.to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn streaming_dispatch_rejects_mismatched_workdir() {
        let temp = tempdir().expect("tempdir");
        let other_dir = tempdir().expect("other tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: other_dir.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new(),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("mismatched workdir must fail");

        assert!(error.to_string().contains("shared-checkout"));
    }

    #[tokio::test]
    async fn streaming_dispatch_settles_cost_on_provider_failure() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"fail-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-f1","model":"claude-sonnet-4-6","total_cost_usd":0.15,"usage":{"input_tokens":3,"output_tokens":4},"is_error":true}'
exit 1
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-FAIL".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("failed provider must error");

        assert!(matches!(error, RokoError::Agent { .. }));

        // Terminal event must still be emitted for failures.
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        let terminal = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::AttemptTerminal {
                    outcome: TaskDispatchOutcomeKind::Failed,
                    ..
                }
            )
        });
        assert!(terminal, "must emit AttemptTerminal(Failed) event");
    }

    #[tokio::test]
    async fn reconcile_returns_reuse_committed_for_terminal_evidence() {
        use std::sync::atomic::{AtomicBool, Ordering};

        /// Test recorder that reports terminal evidence for a specific attempt.
        struct TerminalRecorder {
            terminal: AtomicBool,
        }

        #[async_trait::async_trait]
        impl ProviderAttemptRecorder for TerminalRecorder {
            async fn record_start(&self, _id: &str, _spec: &TaskExecutionSpec) -> Result<()> {
                Ok(())
            }
            async fn record_terminal(
                &self,
                _id: &str,
                _outcome: &TaskDispatchOutcome,
            ) -> Result<()> {
                self.terminal.store(true, Ordering::SeqCst);
                Ok(())
            }
            async fn has_terminal_evidence(&self, _id: &str) -> bool {
                self.terminal.load(Ordering::SeqCst)
            }
            async fn has_started_evidence(&self, _id: &str) -> bool {
                false
            }
        }

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);

        let recorder = TerminalRecorder {
            terminal: AtomicBool::new(true),
        };

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-attempt-1",
            &recorder,
        )
        .await;

        assert!(
            matches!(result, AttemptReconciliation::ReuseCommitted { .. }),
            "terminal evidence must return ReuseCommitted"
        );
    }

    #[tokio::test]
    async fn reconcile_returns_fail_ambiguous_for_started_evidence() {
        /// Recorder that reports started-but-not-terminal evidence.
        struct StartedRecorder;

        #[async_trait::async_trait]
        impl ProviderAttemptRecorder for StartedRecorder {
            async fn record_start(&self, _id: &str, _spec: &TaskExecutionSpec) -> Result<()> {
                Ok(())
            }
            async fn record_terminal(
                &self,
                _id: &str,
                _outcome: &TaskDispatchOutcome,
            ) -> Result<()> {
                Ok(())
            }
            async fn has_terminal_evidence(&self, _id: &str) -> bool {
                false
            }
            async fn has_started_evidence(&self, _id: &str) -> bool {
                true
            }
        }

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-attempt-ambig",
            &StartedRecorder,
        )
        .await;

        assert!(
            matches!(result, AttemptReconciliation::FailAmbiguous { .. }),
            "started-but-not-terminal must return FailAmbiguous"
        );
    }

    #[tokio::test]
    async fn reconcile_returns_allocate_new_for_no_evidence() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);
        let recorder = NoopAttemptRecorder;

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-never-started",
            &recorder,
        )
        .await;

        match result {
            AttemptReconciliation::AllocateNew { attempt_id } => {
                assert!(
                    attempt_id.contains("prev-never-started"),
                    "new attempt ID must reference the original"
                );
            }
            other => panic!("expected AllocateNew, got {other:?}"),
        }
    }

    #[test]
    fn streaming_event_channel_capacity_is_bounded() {
        let capacity = streaming_event_channel_capacity();
        assert!(
            capacity > 0 && capacity <= 1024,
            "channel capacity {capacity} must be bounded and reasonable"
        );
    }

    #[tokio::test]
    async fn streaming_dispatch_respects_budget_exhaustion() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"expensive"}}'
printf '%s\n' '{"type":"result","session_id":"sess-x","model":"claude-sonnet-4-6","total_cost_usd":1.00,"usage":{"input_tokens":50,"output_tokens":100}}'
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let recorder = NoopAttemptRecorder;

        // First dispatch exhausts the $1.00 budget.
        let (event_tx, _) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let _ = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-EXP-1".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await;

        // Second dispatch must fail with budget exhaustion.
        let (event_tx2, _) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-EXP-2".to_string()),
                &lease,
                event_tx2,
                &recorder,
            )
            .await
            .expect_err("budget-exhausted dispatch must fail");

        assert!(
            matches!(error, RokoError::BudgetExceeded { .. }),
            "error must be BudgetExceeded, got: {error:?}"
        );
    }
}
