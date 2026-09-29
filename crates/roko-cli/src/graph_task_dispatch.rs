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
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use roko_agent::safety::contract::{AgentContract, ContractLoadMode};
use roko_core::config::schema::RokoConfig;
use roko_core::error::{Result, RokoError};
use roko_core::{Body, Context, Kind, Signal, Verify};
use roko_gate::GatePayload;
use roko_gate::ShellGate;
use roko_gate::TurnSnapshot;
use roko_gate::eval_generator::EvalGenerator;
use roko_gate::rung_for_gate_name;
use roko_graph::cell::CellContext;
use roko_graph::cells::task_executor::TaskGateVerdict;
use roko_graph::cells::{
    AttemptReconciliation, GraphTaskEvent, ProviderAttemptRecorder, StreamingTaskDispatcher,
    TaskDispatchOutcome, TaskDispatchOutcomeKind, TaskDispatcher, TaskExecutionSpec, TaskLease,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::oracles::coding::{BuildRecord, CodingOracle, TestRecord};
use roko_learn::reflex_store::{ReflexObservation, ReflexStore};
use roko_learn::shadow::ShadowRunner;

use crate::dispatch::{
    AgentDispatchRequest, DispatchContext, GateFeedback, ModelChoiceSource, SharedAgentFactory,
};
use crate::graph_checkpoint::GraphCostLedgerCheckpoint;
use crate::runner::persist::GateThresholds;
use crate::runner::tui_bridge::TuiBridge;
use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade};
use crate::task_parser::TaskDef;

mod prompt_experiment;
mod sibling_settle;

#[cfg(test)]
mod gate_output_accept;

const MICRO_USD_PER_USD: f64 = 1_000_000.0;

/// How often to publish a [`TuiBridge::agent_heartbeat`] while waiting for a
/// provider dispatch to complete.  5 seconds lets the dashboard show elapsed
/// time at a human-readable granularity without generating excessive events.
const AGENT_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Which live events the dispatcher forwards to the TUI while an agent runs.
///
/// Configured via [`GraphTaskDispatcher::with_live_agent_output`]. The
/// default is `None` (no live output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveAgentOutput {
    /// Forward tool steps only (name + target, no raw text/reasoning).
    ///
    /// Safe to display without additional scrubbing: only the tool ID, tool
    /// name, and the projected/scrubbed target string are forwarded.
    ToolSteps,
    /// Forward tool steps **and** unscreened text/reasoning/tool-call deltas.
    ///
    /// Should only be used for trusted local consumers (e.g. the developer's
    /// TUI), since unscreened content may include sensitive output.
    Trusted,
}

/// Thin `Agent` adapter that forwards a one-shot prompt through the shared
/// factory bridge so `error_enrichment` and `quality_judge` can use the
/// live provider without rebuilding the full dispatch stack.
///
/// The adapter is intentionally lightweight: it constructs a minimal
/// `AgentDispatchRequest` with no tools, no MCP, and no contract, targeting
/// the model chosen by [`select_cheap_model_key`] and bounded by
/// `timeouts.llm_call_secs`. On dispatch failure it returns an
/// unsuccessful `AgentResult` so callers' built-in fallbacks activate.
struct CheapFactoryAgent {
    factory: Arc<SharedAgentFactory>,
    model_key: String,
    workdir: PathBuf,
    timeout_ms: u64,
}

#[async_trait::async_trait]
impl roko_agent::Agent for CheapFactoryAgent {
    async fn run(&self, input: &Signal, _ctx: &Context) -> roko_agent::AgentResult {
        let prompt = match input.body.as_text() {
            Ok(text) => text.to_string(),
            Err(_) => {
                return roko_agent::AgentResult::fail(
                    Signal::builder(Kind::AgentOutput)
                        .body(Body::text("cheap-factory-agent: non-text input"))
                        .build(),
                );
            }
        };
        let request = AgentDispatchRequest {
            model_key: self.model_key.clone(),
            prompt,
            system_prompt: String::new(),
            workdir: self.workdir.clone(),
            immune_root: None,
            agent_id: "cheap-factory-agent".to_string(),
            command: None,
            timeout_ms: Some(self.timeout_ms),
            mcp_config: None,
            env: vec![],
            extra_args: vec![],
            effort: None,
            tools: None,
            agent_contract: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            max_turns: None,
            live_output: None,
        };
        match self.factory.run_shared_agent_bridge(request).await {
            Ok(dispatch) => dispatch.result,
            Err(_) => roko_agent::AgentResult::fail(
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("cheap-factory-agent: dispatch failed"))
                    .build(),
            ),
        }
    }

    fn name(&self) -> &str {
        "cheap-factory-agent"
    }
}

/// Choose the model for best-effort one-shot helper calls (quality judge,
/// error enrichment, gate reflections).
///
/// Prefers `routing.fast_task_model` when it names (by `[models.*]` key or
/// slug) a dispatchable model. Otherwise picks the cheapest dispatchable
/// model by input price, then output price, using `[models.*]` prices with
/// the built-in pricing table as fallback; unpriced models rank last.
/// Embedding models, tool-less (search-only) models, and models of
/// `routing.disabled_providers` are never chosen. Returns the `[models.*]`
/// key so dispatch resolves exactly that profile.
fn select_cheap_model_key(config: &RokoConfig) -> Option<String> {
    select_cheap_model_key_with(config, |key| config.provider_available_for_model_key(key))
}

/// [`select_cheap_model_key`] with an injectable provider-availability check.
fn select_cheap_model_key_with(
    config: &RokoConfig,
    available: impl Fn(&str) -> bool,
) -> Option<String> {
    let models = config.effective_models();
    let candidates: Vec<(&String, &roko_core::config::schema::ModelProfile)> = models
        .iter()
        .filter(|(key, profile)| {
            !profile.is_embedding_model
                && profile.supports_tools
                && !profile.slug.trim().is_empty()
                && !config
                    .routing
                    .disabled_providers
                    .contains(&profile.provider)
                && available(key)
        })
        .collect();

    let fast = config.routing.fast_task_model.trim();
    if let Some((key, _)) = candidates
        .iter()
        .find(|(key, profile)| !fast.is_empty() && (key.as_str() == fast || profile.slug == fast))
    {
        return Some((*key).clone());
    }

    let price = |profile: &roko_core::config::schema::ModelProfile| {
        let builtin = roko_core::config::model_registry::builtin_pricing(&profile.slug);
        (
            profile
                .cost_input_per_m
                .or(builtin.map(|pricing| pricing.input_per_m)),
            profile
                .cost_output_per_m
                .or(builtin.map(|pricing| pricing.output_per_m)),
        )
    };
    candidates
        .into_iter()
        .min_by(|(a_key, a_profile), (b_key, b_profile)| {
            let (a_input, a_output) = price(a_profile);
            let (b_input, b_output) = price(b_profile);
            cmp_price(a_input, b_input)
                .then(cmp_price(a_output, b_output))
                .then_with(|| a_key.cmp(b_key))
        })
        .map(|(key, _)| key.clone())
}

/// Order prices ascending, with unknown or non-finite prices last.
fn cmp_price(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    let rank = |price: Option<f64>| {
        price
            .filter(|value| value.is_finite())
            .unwrap_or(f64::INFINITY)
    };
    rank(a).total_cmp(&rank(b))
}

/// P1-16: Resolve cross-cut functor conflicts at routing time.
///
/// When Memory, Daimon, and Dreams all propose routing recommendations on
/// the same signal set, the arbitrator applies priority resolution (safety-
/// critical Daimon wins, consolidated Memory beats speculative Dreams) and
/// falls back to VCG second-price arbitration for same-level ties.
///
/// Returns an `Option<RoutingBias>` derived from the winning recommendation
/// so the cascade router can incorporate the cross-cut consensus.
///
/// `dream_advice` is the persisted Dreams advice, loaded once per dispatch
/// and shared with [`dream_routing_bias`].
fn arbitrate_cross_cut_routing_bias(
    feedback: &GraphFeedbackContext,
    dream_advice: Option<&roko_dreams::DreamRoutingAdvice>,
    task_category: &str,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    use roko_compose::auction::{
        CrossCutArbitrationResult, CrossCutDecisionKind, CrossCutRecommendation,
    };

    // Collect recommendations from persisted cross-cut state.
    let mut recommendations = Vec::new();

    // Dreams routing advice (persisted by DreamOutputConsumer or delta dream).
    if let Some(advice) = dream_advice {
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
    #[allow(dead_code)] // budget enforcement helper; production caller deferred
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

/// Effective per-task spend ceiling in USD (`0.0` = unlimited): the tighter
/// of `budget.max_task_usd` scaled by the tier multiplier and
/// `budget.max_task_retry_usd`.
fn task_budget_ceiling_usd(budget: &roko_core::config::BudgetConfig, task: &TaskDef) -> f64 {
    [
        budget.task_limit_usd(&task.tier, task.model_hint.as_deref()),
        f64::from(budget.max_task_retry_usd),
    ]
    .into_iter()
    .filter(|limit| limit.is_finite() && *limit > 0.0)
    .reduce(f64::min)
    .unwrap_or(0.0)
}

/// Provider spend per task (`"{plan_id}/{task_id}"`), summed across every
/// attempt of this run, for per-task ceiling admission.
///
/// Unlike the plan ledger it is not checkpointed: a resumed run starts each
/// task's count at zero.
#[derive(Debug, Default)]
struct GraphTaskSpendLedger {
    tasks: parking_lot::Mutex<HashMap<String, u64>>,
}

impl GraphTaskSpendLedger {
    fn record(&self, task_key: &str, cost_usd: f64) {
        let cost_micro_usd = usd_to_micro_usd(cost_usd);
        if cost_micro_usd == 0 {
            return;
        }
        let mut tasks = self.tasks.lock();
        let spent = tasks.entry(task_key.to_string()).or_default();
        *spent = spent.saturating_add(cost_micro_usd);
    }

    /// Reject another attempt once the task's spend reaches `ceiling_usd`.
    /// A non-positive or non-finite ceiling means unlimited.
    fn admit(&self, task_key: &str, ceiling_usd: f64) -> Result<()> {
        if !(ceiling_usd.is_finite() && ceiling_usd > 0.0) {
            return Ok(());
        }
        let ceiling_micro_usd = usd_to_micro_usd(ceiling_usd).max(1);
        let spent_micro_usd = self.tasks.lock().get(task_key).copied().unwrap_or(0);
        if spent_micro_usd >= ceiling_micro_usd {
            return Err(RokoError::BudgetExceeded {
                dimension: "task_cost_micro_usd",
                used: micro_usd_to_usize(spent_micro_usd),
                limit: micro_usd_to_usize(ceiling_micro_usd),
            });
        }
        Ok(())
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

/// P3-AGT-2: Express mode turn limit for mechanical/trivial tasks.
///
/// When express mode is active, these tasks get a 5-turn budget instead of
/// their tier's `[pipeline.<tier>] max_turns`. Mechanical fixes typically
/// need only 1-3 turns (read files, write patch, done).
const EXPRESS_MAX_TURNS: u32 = 5;

/// Return `true` when the task qualifies for express dispatch.
///
/// Express mode is enabled when:
/// - `conductor.express_mode = true` in the workspace config, AND
/// - The task tier is `"mechanical"` or `"trivial"`.
///
/// When active, the dispatcher:
/// - Routes to `routing.fast_task_model` (cheapest available model).
/// - Skips the eval-generation pre-dispatch step.
/// - Caps the agent turn limit at [`EXPRESS_MAX_TURNS`].
fn is_express_task(config: &roko_core::config::schema::RokoConfig, task: &TaskDef) -> bool {
    if !config.conductor.express_mode {
        return false;
    }
    let tier_lower = task.tier.to_ascii_lowercase();
    matches!(tier_lower.as_str(), "mechanical" | "trivial")
}

/// Provider turn cap for one Graph task dispatch.
///
/// Every task gets its tier's `[pipeline.<tier>] max_turns` (unknown tiers
/// use the `focused` band, so the cap is never unbounded); express dispatch
/// lowers it further to [`EXPRESS_MAX_TURNS`].
fn task_turn_limit(config: &RokoConfig, task: &TaskDef, express_active: bool) -> u32 {
    let tier_limit = config.pipeline.max_turns_for_tier(&task.tier);
    if express_active {
        tier_limit.min(EXPRESS_MAX_TURNS)
    } else {
        tier_limit
    }
}

/// An attempt that stopped at its turn cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TurnCapRetry {
    /// Cap the stopped attempt ran with.
    cap: u32,
    /// Turns the provider reported using, when it said.
    num_turns: Option<u32>,
}

/// Cap for the attempt after one that stopped at `cap`: half again, and
/// always at least one more turn.
fn raised_turn_cap(cap: u32) -> u32 {
    cap.saturating_add(cap.div_ceil(2))
        .max(cap.saturating_add(1))
}

/// Section appended to the user prompt of the attempt after a turn-cap stop,
/// so the agent continues the partial work instead of starting over.
fn turn_cap_resume_note(previous: TurnCapRetry, cap: u32) -> String {
    let used = previous
        .num_turns
        .map_or_else(String::new, |turns| format!(" after {turns} turns"));
    format!(
        "\n\n# Resuming a partially completed task\n\n\
         Your previous attempt at this task stopped at its {prev_cap}-turn cap{used}. \
         Its edits are still in the working tree. Inspect the current state of the \
         files in scope (for example with `git diff`), keep what is already correct, \
         and continue from there instead of starting over. This attempt has a \
         {cap}-turn cap.\n",
        prev_cap = previous.cap,
    )
}

/// Longest failure reason recorded on an episode, in bytes.
const MAX_FAILURE_REASON_BYTES: usize = 2_048;

/// Class-prefixed reason for a failed attempt (`"<class>: <detail>"`),
/// recorded on its episode. A reason within [`MAX_FAILURE_REASON_BYTES`]
/// keeps every line, so a verify summary keeps the step that failed; a
/// longer one keeps its first lines and its tail around an omission marker.
fn attempt_failure_reason(class: &str, detail: &str) -> String {
    let detail = detail.trim();
    let detail = if detail.is_empty() {
        "no detail"
    } else {
        detail
    };
    let budget = MAX_FAILURE_REASON_BYTES.saturating_sub(class.len() + 2);
    format!("{class}: {}", head_and_tail(detail, budget))
}

/// `text` when it fits in `max` bytes; otherwise its head and tail, cut at
/// line breaks near the cut points, joined by `… N bytes omitted …`.
fn head_and_tail(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    // Room for the "\n… N bytes omitted …\n" marker.
    let budget = max.saturating_sub(48);
    let mut head_end = budget / 2;
    while !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let head = &text[..head_end];
    let head = head
        .rfind('\n')
        .filter(|&cut| cut >= head_end * 3 / 4)
        .map_or(head, |cut| &head[..cut]);
    let mut tail_start = text.len() - (budget - budget / 2);
    while !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let tail = &text[tail_start..];
    let tail = tail
        .find('\n')
        .filter(|&cut| cut <= tail.len() / 4)
        .map_or(tail, |cut| &tail[cut + 1..]);
    let omitted = text.len() - head.len() - tail.len();
    format!("{head}\n… {omitted} bytes omitted …\n{tail}")
}

/// [`attempt_failure_reason`] for failed verification: the verify summary
/// itself, which leads with any `blocked_by_sibling = <task>` blame, without
/// the error's `gate error (…)` wrapper.
fn verify_failure_reason(error: &RokoError) -> String {
    match error {
        RokoError::Verify { message, .. } => attempt_failure_reason("verify", message),
        other => attempt_failure_reason("verify", &other.to_string()),
    }
}

/// [`attempt_failure_reason`] for an unsuccessful provider result.
fn provider_failure_reason(message: &str) -> String {
    use roko_agent::provider::error_classify::{detect_provider_exhaustion, detect_turn_cap};

    let class = if detect_turn_cap(message).is_some() {
        "turn_cap"
    } else if detect_provider_exhaustion(message).is_some() {
        "provider_exhausted"
    } else {
        "provider"
    };
    attempt_failure_reason(class, message)
}

/// A config key set to a non-default value that `plan run` (Graph engine)
/// never reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InertGraphSetting {
    /// Dotted config key.
    pub key: &'static str,
    /// Why the key has no effect on `plan run`.
    pub reason: &'static str,
}

/// Config keys set to non-default values that have no effect on `plan run`.
///
/// Reported once per process when a Graph dispatcher is built and by
/// `roko config doctor`, so operators stop relying on them.
#[must_use]
pub fn graph_engine_inert_settings(config: &RokoConfig) -> Vec<InertGraphSetting> {
    const LEGACY_GATES: &str = "only the legacy Runner-v2 gate pipeline (--engine legacy) reads it";
    const ADAPTIVE: &str =
        "only AdaptiveThresholds reads it, and no run constructs one (Graph uses a fixed EMA)";
    const NOT_ENFORCED: &str = "not enforced by the Graph engine";
    const NO_READER: &str = "no production code reads it";
    const DISPLAY_ONLY: &str = "shown by config views; no routing decision reads it";
    const NO_WARM_POOL: &str = "no dispatch path pre-spawns or reuses agents";
    const PIPELINE_BAND: &str = "only `max_turns` in [pipeline.<tier>] affects plan run";

    let defaults = RokoConfig::default();
    let (gates, default_gates) = (&config.gates, &defaults.gates);
    let (routing, default_routing) = (&config.routing, &defaults.routing);
    let mut checks = vec![
        (gates.mode != default_gates.mode, "gates.mode", LEGACY_GATES),
        (
            gates.clippy_enabled != default_gates.clippy_enabled,
            "gates.clippy_enabled",
            LEGACY_GATES,
        ),
        (
            gates.skip_tests != default_gates.skip_tests,
            "gates.skip_tests",
            LEGACY_GATES,
        ),
        (
            gates.max_iterations != default_gates.max_iterations,
            "gates.max_iterations",
            LEGACY_GATES,
        ),
        (
            gates.impact_timeout_ms != default_gates.impact_timeout_ms,
            "gates.impact_timeout_ms",
            LEGACY_GATES,
        ),
        (
            gates.impact_max_reverse_dependents != default_gates.impact_max_reverse_dependents,
            "gates.impact_max_reverse_dependents",
            LEGACY_GATES,
        ),
        (
            gates.impact_max_targets != default_gates.impact_max_targets,
            "gates.impact_max_targets",
            LEGACY_GATES,
        ),
        (
            gates.custom_rungs != default_gates.custom_rungs,
            "gates.rungs",
            LEGACY_GATES,
        ),
        (
            gates.max_rung != default_gates.max_rung,
            "gates.max_rung",
            LEGACY_GATES,
        ),
        (
            gates.domain_gates != default_gates.domain_gates,
            "gates.domain_gates",
            NO_READER,
        ),
        (
            gates.ema_alpha.to_bits() != default_gates.ema_alpha.to_bits(),
            "gates.ema_alpha",
            ADAPTIVE,
        ),
        (
            gates.adaptive_min_retries != default_gates.adaptive_min_retries,
            "gates.adaptive_min_retries",
            ADAPTIVE,
        ),
        (
            gates.adaptive_max_retries != default_gates.adaptive_max_retries,
            "gates.adaptive_max_retries",
            ADAPTIVE,
        ),
        (
            gates.skip_streak_threshold != default_gates.skip_streak_threshold,
            "gates.skip_streak_threshold",
            ADAPTIVE,
        ),
        (
            gates.convergence_min_observations != default_gates.convergence_min_observations,
            "gates.convergence_min_observations",
            ADAPTIVE,
        ),
        (
            config.budget.max_daily_usd.to_bits() != defaults.budget.max_daily_usd.to_bits(),
            "budget.max_daily_usd",
            NOT_ENFORCED,
        ),
        (
            config.budget.max_agent_lifetime_usd.to_bits()
                != defaults.budget.max_agent_lifetime_usd.to_bits(),
            "budget.max_agent_lifetime_usd",
            NOT_ENFORCED,
        ),
        (
            config.learning.replan_max_per_plan != defaults.learning.replan_max_per_plan,
            "learning.replan_max_per_plan",
            NO_READER,
        ),
        (
            config.learning.replan_gate_attempts != defaults.learning.replan_gate_attempts,
            "learning.replan_gate_attempts",
            NO_READER,
        ),
        (config.agent.data_llm.is_some(), "agent.data_llm", NO_READER),
        (
            routing.algorithm != default_routing.algorithm,
            "routing.algorithm",
            DISPLAY_ONLY,
        ),
        (
            routing.discount_factor.to_bits() != default_routing.discount_factor.to_bits(),
            "routing.discount_factor",
            DISPLAY_ONLY,
        ),
        (
            routing.standard_task_model != default_routing.standard_task_model,
            "routing.standard_task_model",
            DISPLAY_ONLY,
        ),
        (
            routing.complex_task_model != default_routing.complex_task_model,
            "routing.complex_task_model",
            DISPLAY_ONLY,
        ),
        (
            routing.context_strategy != default_routing.context_strategy,
            "routing.context_strategy",
            DISPLAY_ONLY,
        ),
        (
            routing.weights != default_routing.weights,
            "routing.weights",
            DISPLAY_ONLY,
        ),
        (
            config.runner.warm_pool_size != defaults.runner.warm_pool_size,
            "runner.warm_pool_size",
            NO_WARM_POOL,
        ),
        (
            config.runner.warm_pool_idle_timeout_secs
                != defaults.runner.warm_pool_idle_timeout_secs,
            "runner.warm_pool_idle_timeout_secs",
            NO_WARM_POOL,
        ),
    ];
    for (key, band, default_band) in [
        (
            "pipeline.mechanical",
            config.pipeline.mechanical,
            defaults.pipeline.mechanical,
        ),
        (
            "pipeline.focused",
            config.pipeline.focused,
            defaults.pipeline.focused,
        ),
        (
            "pipeline.integrative",
            config.pipeline.integrative,
            defaults.pipeline.integrative,
        ),
        (
            "pipeline.architectural",
            config.pipeline.architectural,
            defaults.pipeline.architectural,
        ),
    ] {
        let band_without_turns = |band: roko_core::config::PipelineBandConfig| {
            (
                band.strategist,
                band.reviewers,
                band.reviewer_mode,
                band.max_iterations,
            )
        };
        checks.push((
            band_without_turns(band) != band_without_turns(default_band),
            key,
            PIPELINE_BAND,
        ));
    }
    checks
        .into_iter()
        .filter(|(changed, _, _)| *changed)
        .map(|(_, key, reason)| InertGraphSetting { key, reason })
        .collect()
}

/// Warn once per process about [`graph_engine_inert_settings`].
fn warn_inert_graph_settings_once(config: &RokoConfig) {
    static WARNED: std::sync::Once = std::sync::Once::new();
    let inert = graph_engine_inert_settings(config);
    if inert.is_empty() {
        return;
    }
    WARNED.call_once(|| {
        let keys = inert
            .iter()
            .map(|setting| setting.key)
            .collect::<Vec<_>>()
            .join(", ");
        tracing::warn!(
            keys = %keys,
            "config keys set to non-default values have no effect on `plan run` \
             (Graph engine); run `roko config doctor` for details"
        );
    });
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
    /// Path to `.roko/learn/gate-failures.jsonl` for structured gate failure records.
    pub gate_failures_path: Option<PathBuf>,
    /// Path to `.roko/learn/post-gate-reflections.json` for LLM-generated gate reflection store.
    pub post_gate_reflection_path: Option<PathBuf>,
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
    /// P2-LRN-6 Loop 1: Path to `.roko/learn/gate-thresholds.json` for
    /// adaptive EMA threshold updates after each verify run.
    ///
    /// When set, each verify step outcome is fed into `GateThresholds::observe`
    /// so the EMA pass-rate converges toward the workspace's real gate history.
    /// The file is written atomically after every task's verify sequence
    /// completes (both pass and fail), and a `GateThresholdsUpdated` event is
    /// published to the TUI bridge.
    pub gate_thresholds_path: Option<PathBuf>,

    /// RAG-10: Path to `.roko/learn/retrieval-outcomes.jsonl`.
    ///
    /// When set, each task dispatch appends one pre-gate
    /// [`roko_learn::retrieval_outcome::RetrievalOutcomeRecord`] immediately
    /// after prompt assembly (strategy + result count known, gate unknown), and
    /// a second settled record once all verify steps complete so the gate-pass
    /// correlation is durably captured.
    pub retrieval_outcomes_path: Option<PathBuf>,
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
            .field("gate_failures_path", &self.gate_failures_path)
            .field("post_gate_reflection_path", &self.post_gate_reflection_path)
            .field("replan_on_gate_failure", &self.replan_on_gate_failure)
            .field("coding_oracle", &self.coding_oracle.is_some())
            .field("gate_gaming_detector", &self.gate_gaming_detector.is_some())
            .field("holdout_experiment", &self.holdout_experiment.is_some())
            .field("shadow_runner", &self.shadow_runner.is_some())
            .field("eval_generation_enabled", &self.eval_generation_enabled)
            .field("gate_thresholds_path", &self.gate_thresholds_path)
            .field("retrieval_outcomes_path", &self.retrieval_outcomes_path)
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
            gate_failures_path: None,
            post_gate_reflection_path: None,
            replan_on_gate_failure: false,
            coding_oracle: None,
            gate_gaming_detector: None,
            holdout_experiment: None,
            shadow_runner: None,
            eval_generation_enabled: false,
            gate_thresholds_path: None,
            retrieval_outcomes_path: None,
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
    /// Which live events to forward to the TUI while the agent runs.
    ///
    /// Defaults to `None` (no live output). Set via
    /// [`Self::with_live_agent_output`]. Requires `tui_bridge` to be set.
    live_agent_output: Option<LiveAgentOutput>,
    /// Per-task gate failure context carried across retries.
    ///
    /// When a task's verify steps fail, the structured gate output is stored
    /// here keyed by `"{plan_id}/{task_id}"`. On the next retry of the same
    /// task, the dispatcher reads this feedback and injects it into the
    /// `DispatchContext` so the agent prompt includes the previous errors.
    /// The value is `(feedback, attempt_number)`.
    gate_retry_context: parking_lot::Mutex<HashMap<String, (GateFeedback, u32)>>,
    /// Aggregate input tokens accumulated across all dispatches in this run.
    agg_tokens_in: AtomicU64,
    /// Aggregate output tokens accumulated across all dispatches in this run.
    agg_tokens_out: AtomicU64,
    /// Aggregate number of agent dispatch calls in this run.
    agg_dispatch_count: AtomicU64,
    /// Run-scoped cache of static prompt context that does not change per task.
    ///
    /// Contains `(workspace_map, workspace_context, cfactor_context)`.
    /// Computed at most once per plan run on the first dispatch call, then
    /// cloned into every `DispatchContext` to avoid repeated blocking I/O
    /// (filesystem reads + `git` subprocess spawns) on the Tokio reactor.
    static_prompt_cache: std::sync::OnceLock<(String, String, String)>,
    /// T0 reflex store. When set, each dispatch checks for a matching
    /// reflex rule before invoking the LLM. A match bypasses the agent call
    /// entirely and returns the rule's cached output (zero-cost repeated
    /// decisions). Gate feedback records are posted to the store so rules
    /// accumulate confidence or are demoted over time.
    reflex_store: Option<ReflexStore>,
    /// Per-task spend across attempts, enforcing `budget.max_task_usd` and
    /// `budget.max_task_retry_usd`.
    task_spend: GraphTaskSpendLedger,
    /// `[meta] skip_enrichment` per plan id, read once from the plan's
    /// `tasks.toml`.
    skip_enrichment_plans: parking_lot::Mutex<HashMap<String, bool>>,
    /// Tasks (`"{plan_id}/{task_id}"`) whose last attempt stopped at its turn
    /// cap; the next attempt raises the cap and resumes the partial work.
    turn_cap_retries: parking_lot::Mutex<HashMap<String, TurnCapRetry>>,
    /// Dispatch attempts started per task (`"{plan_id}/{task_id}"`) in this
    /// run; numbers each attempt's efficiency records.
    task_attempts: parking_lot::Mutex<HashMap<String, u32>>,

    /// RAG-10/11: Per-task retrieval context retained from prompt assembly until
    /// gate settlement.
    ///
    /// Keyed by `"{plan_id}/{task_id}"`.  Value is
    /// `(strategy, query, results_count, latency_ms)`.
    /// Set immediately after `plan()` returns so that both the pre-gate record and
    /// the gate-settled record carry the same metadata.
    retrieval_ctx: parking_lot::Mutex<HashMap<String, (String, String, usize, u64)>>,
    /// Attempts running now, so a verify step that fails while siblings edit
    /// the same working tree can wait for them to settle.
    in_flight: sibling_settle::InFlightTasks,
}

impl GraphTaskDispatcher {
    /// Construct a dispatcher sharing the plan run's provider runtime.
    #[must_use]
    pub fn new(
        factory: Arc<SharedAgentFactory>,
        config: Arc<RokoConfig>,
        workdir: PathBuf,
    ) -> Self {
        warn_inert_graph_settings_once(&config);
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
            live_agent_output: None,
            gate_retry_context: parking_lot::Mutex::new(HashMap::new()),
            agg_tokens_in: AtomicU64::new(0),
            agg_tokens_out: AtomicU64::new(0),
            agg_dispatch_count: AtomicU64::new(0),
            static_prompt_cache: std::sync::OnceLock::new(),
            reflex_store: None,
            retrieval_ctx: parking_lot::Mutex::new(HashMap::new()),
            task_spend: GraphTaskSpendLedger::default(),
            skip_enrichment_plans: parking_lot::Mutex::new(HashMap::new()),
            turn_cap_retries: parking_lot::Mutex::new(HashMap::new()),
            task_attempts: parking_lot::Mutex::new(HashMap::new()),
            in_flight: sibling_settle::InFlightTasks::default(),
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

    /// Configure which live events are forwarded to the TUI while an agent runs.
    ///
    /// Requires [`Self::with_tui_bridge`] to also be configured. When set,
    /// each task dispatch creates a bounded channel, attaches it to the
    /// [`AgentDispatchRequest`] so the immune boundary can push events, and
    /// spawns a forwarder task that publishes each event to the TUI bridge
    /// before the screened transcript arrives via
    /// [`Self::forward_dispatch_events_to_tui`].
    ///
    /// Default setting: [`LiveAgentOutput::ToolSteps`] when called with `None`
    /// is not applicable — call this method with the desired variant.
    #[must_use]
    pub fn with_live_agent_output(mut self, setting: LiveAgentOutput) -> Self {
        self.live_agent_output = Some(setting);
        self
    }

    /// Attach the T0 reflex store for pre-dispatch reflex checks.
    ///
    /// When set, each `dispatch` call opens the reflex store and checks
    /// whether any rule matches the task's role, file extensions, and
    /// title before invoking the LLM. A match bypasses the agent call
    /// and returns the rule's cached output (`action.args`). Gate feedback
    /// is posted back so rules accumulate confidence or are demoted.
    #[must_use]
    pub fn with_reflex_store(mut self, store: ReflexStore) -> Self {
        self.reflex_store = Some(store);
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

    /// Return aggregate token and dispatch counts accumulated across all
    /// task dispatches in this run. Used by the run-metrics persistence
    /// path (backlog #169) to populate `RunMetricsRecord` with real values
    /// instead of zeros.
    #[must_use]
    pub fn run_aggregate_stats(&self) -> (u64, u64, u64) {
        (
            self.agg_tokens_in.load(Ordering::Relaxed),
            self.agg_tokens_out.load(Ordering::Relaxed),
            self.agg_dispatch_count.load(Ordering::Relaxed),
        )
    }

    /// Return a `CheapFactoryAgent` wired to the model chosen by
    /// [`select_cheap_model_key`], or `None` when no model is dispatchable.
    /// Used for best-effort error enrichment and quality judgment calls.
    fn cheap_agent(&self) -> Option<CheapFactoryAgent> {
        let model_key = select_cheap_model_key(&self.config)?;
        Some(CheapFactoryAgent {
            factory: Arc::clone(&self.factory),
            model_key,
            workdir: self.workdir.clone(),
            timeout_ms: self
                .config
                .timeouts
                .llm_call_secs
                .max(1)
                .saturating_mul(1_000),
        })
    }

    /// Whether the plan's `[meta] skip_enrichment` is set, read once per plan
    /// from `<plan_dir>/tasks.toml`. An unreadable file counts as `false`.
    fn plan_skips_enrichment(&self, spec: &TaskExecutionSpec) -> bool {
        let mut plans = self.skip_enrichment_plans.lock();
        if let Some(skip) = plans.get(&spec.plan_id) {
            return *skip;
        }
        let plan_dir = Path::new(&spec.plan_dir);
        let skip = [plan_dir.to_path_buf(), self.workdir.join(plan_dir)]
            .into_iter()
            .filter(|_| !spec.plan_dir.trim().is_empty())
            .map(|dir| dir.join("tasks.toml"))
            .find(|path| path.is_file())
            .and_then(|path| crate::task_parser::TasksFile::parse(&path).ok())
            .is_some_and(|tasks| tasks.meta.skip_enrichment);
        if skip {
            tracing::info!(
                plan_id = %spec.plan_id,
                "plan sets skip_enrichment: dispatching tasks as authored \
                 (no eval artifacts, no dream/cross-cut routing advice)"
            );
        }
        plans.insert(spec.plan_id.clone(), skip);
        skip
    }

    /// Per-task spend admission against [`task_budget_ceiling_usd`], mirroring
    /// the plan ceiling: an explicit `--budget` override only warns, and
    /// `--no-budget` disables the check.
    fn admit_task_budget(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        task_spend_key: &str,
    ) -> Result<()> {
        let policy = self.budget_policy;
        if policy.continue_on_exhaustion && policy.ceiling_micro_usd.is_none() {
            return Ok(());
        }
        let ceiling_usd = task_budget_ceiling_usd(&self.config.budget, task);
        let Err(error) = self.task_spend.admit(task_spend_key, ceiling_usd) else {
            return Ok(());
        };
        if policy.continue_on_exhaustion {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                ceiling_usd,
                %error,
                "per-task budget exhausted; continuing under the explicit --budget override"
            );
            return Ok(());
        }
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            ceiling_usd,
            %error,
            "per-task budget exhausted (budget.max_task_usd x tier multiplier, \
             budget.max_task_retry_usd); refusing another attempt"
        );
        Err(error)
    }

    /// Allocate the identity of a new dispatch attempt of `task_key`
    /// (`"{plan_id}/{task_id}"`): `"{task_key}/a{n}"`, where `n` counts this
    /// run's attempts of the task from zero. The attempt's gate records append
    /// a suffix, so every efficiency record stays unique yet joins its
    /// dispatch record by prefix.
    fn next_attempt_id(&self, task_key: &str) -> String {
        let mut attempts = self.task_attempts.lock();
        let attempt = attempts.entry(task_key.to_string()).or_default();
        let attempt_id = format!("{task_key}/a{attempt}");
        *attempt = attempt.saturating_add(1);
        attempt_id
    }

    /// Plan a dispatch. When prompt assembly fails with experiment
    /// treatments (say, two running experiments on one section), plan again
    /// without them: a broken experiment must not stop the task.
    fn plan_dispatch(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch_ctx: &mut DispatchContext,
    ) -> Result<crate::dispatch::RunnerDispatchPlan> {
        match self.factory.dispatcher().plan(task, dispatch_ctx) {
            Err(error) if dispatch_ctx.prompt_experiment.is_some() => {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "prompt assembly with experiment treatments failed; dispatching without them"
                );
                dispatch_ctx.prompt_experiment = None;
                self.factory.dispatcher().plan(task, dispatch_ctx)
            }
            planned => planned,
        }
        .map_err(|error| RokoError::Planning(error.to_string()))
    }

    /// Emit all feedback events after a task dispatch completes.
    ///
    /// This is the Graph engine equivalent of Runner-v2's post-dispatch
    /// feedback pipeline. Each subsystem is best-effort: failures are logged
    /// but do not block the task result.
    ///
    /// `attempt_id` comes from [`Self::next_attempt_id`]. `failure_reason` is
    /// the attempt's bounded class-prefixed reason when `succeeded` is false
    /// (see [`attempt_failure_reason`]); it lands on the episode together with
    /// the provider-reported turn count.
    async fn emit_feedback(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_id: &str,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        succeeded: bool,
        wall_duration: std::time::Duration,
        dispatch_plan: &crate::dispatch::RunnerDispatchPlan,
        routing_context: Option<roko_learn::model_router::RoutingContext>,
        failure_reason: Option<String>,
    ) {
        let role = task.role.as_deref().unwrap_or("implementer");
        // P3-02: Agent turns as reported by the provider (the Claude CLI's
        // `num_turns`), so episodes and efficiency records carry real counts.
        let agent_num_turns = dispatch
            .events
            .iter()
            .rev()
            .find_map(|ev| match ev {
                roko_agent::AgentRuntimeEvent::TurnCompleted { num_turns, .. } => *num_turns,
                _ => None,
            })
            .unwrap_or(1);
        let provider_id = &dispatch.target.provider_id;
        let model_slug = &dispatch.target.model_slug;
        let cost_usd = f64::from(dispatch.result.usage.cost_usd);
        let tokens_in = u64::from(dispatch.result.usage.input_tokens);
        let tokens_out = u64::from(dispatch.result.usage.output_tokens);
        let duration_ms = wall_duration.as_millis() as u64;

        // Accumulate run-level aggregates for RunMetricsRecord (#169).
        self.agg_tokens_in.fetch_add(tokens_in, Ordering::Relaxed);
        self.agg_tokens_out.fetch_add(tokens_out, Ordering::Relaxed);
        self.agg_dispatch_count.fetch_add(1, Ordering::Relaxed);

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
        let experiment_settlement =
            prompt_experiment::settlement(succeeded, failure_reason.as_deref());

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
                routing_context,
                prompt_text: Some(dispatch_plan.prompt.system_prompt.clone()),
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                knowledge_ids: vec![],
                playbook_ids: vec![],
                initial_model: model_slug.clone(),
                turns: u64::from(agent_num_turns),
                failure_reason,
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
            // Gather prompt diagnostics for the efficiency event so
            // telemetry reflects what the agent actually received.
            let eff_prompt_sections: Vec<roko_learn::efficiency::PromptSectionMeta> = dispatch_plan
                .prompt
                .diagnostics
                .included_sections
                .iter()
                .map(|name| roko_learn::efficiency::PromptSectionMeta {
                    name: name.clone(),
                    tokens: 0,
                    priority: 0,
                    was_truncated: false,
                    was_dropped: false,
                })
                .collect();
            let eff_system_prompt_tokens = dispatch_plan.prompt.diagnostics.estimated_tokens;
            let eff_tool_calls: Vec<roko_learn::efficiency::ToolCallMeta> = dispatch
                .events
                .iter()
                .filter_map(|ev| match ev {
                    roko_agent::AgentRuntimeEvent::ToolCall { name, .. } => {
                        Some(roko_learn::efficiency::ToolCallMeta {
                            tool_name: name.clone(),
                            duration_ms: 0,
                            result_tokens: 0,
                            succeeded: true,
                            advanced_task: false,
                            was_redundant: false,
                            error_category: None,
                        })
                    }
                    _ => None,
                })
                .collect();
            let eff_tools_used = eff_tool_calls.len() as u32;
            let event = roko_learn::efficiency::AgentEfficiencyEvent {
                agent_id: format!("{}/{}", spec.plan_id, task.id),
                role: role.to_string(),
                backend: provider_id.clone(),
                model: model_slug.clone(),
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                attempt_id: attempt_id.to_string(),
                input_tokens: tokens_in,
                output_tokens: tokens_out,
                reasoning_tokens: 0,
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cache_write_tokens: u64::from(dispatch.result.usage.cache_create_tokens),
                cost_usd,
                cost_usd_without_cache: cost_usd,
                prompt_sections: eff_prompt_sections,
                total_prompt_tokens: tokens_in,
                system_prompt_tokens: u64::from(eff_system_prompt_tokens),
                tools_available: eff_tool_calls.len() as u32,
                tools_used: eff_tools_used,
                tool_calls: eff_tool_calls,
                wall_time_ms: duration_ms,
                duration_ms,
                time_to_first_token_ms: 0,
                // No provider process is pre-spawned or reused, so every
                // dispatch is a cold start.
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
            match serde_json::to_string(&event) {
                Ok(line) => {
                    let path = eff_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    tokio::spawn(async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph efficiency event write failed (best-effort)"
                            );
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "graph efficiency event serialization failed (best-effort)"
                    );
                }
            }
        }

        // ── W05b: Cost record to costs.jsonl ─────────────────────────────
        //
        // `roko status` reads cost totals from `.roko/learn/costs.jsonl` via
        // `CostsLog::total_cost()`. The Graph engine only writes efficiency
        // events (above), so the status cost summary always showed $0.0000.
        // This block bridges the gap: one `CostRecord` per dispatch, written
        // asynchronously alongside the efficiency event.
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
            match serde_json::to_string(&cost_record) {
                Ok(line) => {
                    let path = costs_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    tokio::spawn(async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph cost record write failed (best-effort)"
                            );
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "graph cost record serialization failed (best-effort)"
                    );
                }
            }
        }

        // ── W05c: Publish token usage and cost to the TUI dashboard ──────
        //
        // `token_usage` and `efficiency_event("cost_usd")` feed the snapshot's
        // per-agent counters and the overall `SnapshotStats` totals.  Kept
        // outside the `if let Some(costs_path)` block above so they fire even
        // when `.roko/learn/costs.jsonl` is not configured.  The fold in
        // `DashboardSnapshot::apply` attributes the numbers to the agent that
        // `forward_dispatch_events_to_tui` announced for the task, so publish
        // after that call.
        if let Some(tui) = &self.tui_bridge {
            tui.token_usage(
                &spec.plan_id,
                &task.id,
                tokens_in,
                tokens_out,
                u64::from(dispatch.result.usage.cache_read_tokens),
                u64::from(dispatch.result.usage.cache_create_tokens),
            );
            tui.efficiency_event(&spec.plan_id, &task.id, "cost_usd", cost_usd);
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
        //
        // Settles this attempt's prompt treatments (prepared at prompt
        // assembly, bound to the launched prompt) with its outcome.
        if let Some(store_path) = &self.feedback.experiment_store_path {
            prompt_experiment::settle(
                store_path,
                prompt_experiment::attempt_key(&spec.plan_id, &task.id, attempt_id),
                experiment_settlement,
            )
            .await;
        }
    }

    /// Run a task's authored `[[task.verify]]` steps and settle every
    /// gate-dependent learning record for this attempt.
    ///
    /// Shared by the batch and streaming dispatch paths so both reach the same
    /// verdict. Authored verify steps are deterministic: any failure returns
    /// `RokoError::Verify` (the Graph engine retries up to the task's
    /// `max_retries`, then fails the task) and is never force-accepted. Steps
    /// run fail-fast; the rest are reported as skipped. A step that fails
    /// while sibling tasks edit the same working tree waits for them to
    /// settle and re-runs once; only that result counts (`sibling_settle`).
    /// The caller releases any worktree lease and settles episode feedback
    /// with the result.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    async fn settle_task_verification(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        effective_workdir: &Path,
        retry_key: &str,
        attempt_number: u32,
        attempt_id: &str,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
    ) -> Result<TaskGateVerdict> {
        let effective_workdir = effective_workdir.to_path_buf();
        let retry_key = retry_key.to_string();
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
            // P2-LRN-6 Loop 1: Collect (phase, passed) for each verify step so
            // we can feed outcomes into GateThresholds::observe after all steps
            // complete (including any post-auto-fix re-run).
            let mut step_outcomes: Vec<(String, bool)> = Vec::new();
            // P4-03: PromiseTracker for early termination of doomed attempts.
            let mut promise_tracker = crate::runner::promise_tracker::PromiseTracker::new();
            let mut promise_terminated = false;
            // Steps not run because an earlier step already failed.
            let mut skipped_steps: Vec<String> = Vec::new();
            // Siblings whose files hold every error of a failure that
            // persisted after they settled.
            let mut blocked_by_sibling: Option<String> = None;
            let total_steps = u32::try_from(task.verify.len()).unwrap_or(u32::MAX);

            for (i, step) in task.verify.iter().enumerate() {
                let step_label = verify_step_label(i, &step.phase);

                // Fail fast: once a step has failed (or P4-03 declared the
                // attempt doomed), report later steps as skipped instead of
                // paying for, say, a full compile after a cheap grep failed.
                if promise_terminated || !failures.is_empty() {
                    skipped_steps.push(format!("{step_label} (`{}`)", step.command));
                    continue;
                }

                if let Some(progress_tx) = progress_tx {
                    let _ = progress_tx
                        .send(GraphTaskEvent::Progress {
                            message: format!("verify: {}", step.command),
                            completed: Some(u32::try_from(i).unwrap_or(u32::MAX)),
                            total: Some(total_steps),
                        })
                        .await;
                }

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    phase = %step.phase,
                    command = %step.command,
                    timeout_ms = step.timeout_ms,
                    "graph verify step starting"
                );

                // P2-TUI-4: Notify the TUI that a gate rung is starting so it
                // can show the active rung name and a spinner.
                if let Some(tui) = &self.tui_bridge {
                    tui.gate_rung_started(&spec.plan_id, &task.id, &step_label);
                }

                let gate = ShellGate::new(
                    "bash",
                    vec![
                        "-o".into(),
                        "pipefail".into(),
                        "-c".into(),
                        step.command.clone(),
                    ],
                )
                .with_timeout_ms(step.timeout_ms)
                .with_name(&step_label);

                let compile_permit = verify_compile_permit(
                    &effective_workdir,
                    self.config.gates.compile_concurrency,
                    step,
                    &spec.plan_id,
                    &task.id,
                )
                .await;
                let mut verdict = gate.verify(&gate_signal, &gate_ctx).await;
                if !verdict.passed {
                    // A sibling editing this working tree may have caused the
                    // failure: let it settle, then re-run the step once. The
                    // compile lock is released meanwhile so the sibling's own
                    // cargo steps can finish.
                    drop(compile_permit);
                    let failed_step = sibling_settle::FailedStep {
                        plan_id: &spec.plan_id,
                        task_id: &task.id,
                        files: &task.files,
                        label: &step_label,
                        workdir: &effective_workdir,
                        settle_limit: std::time::Duration::from_secs(
                            self.config.gates.sibling_settle_secs,
                        ),
                    };
                    (verdict, blocked_by_sibling) = self
                        .in_flight
                        .settle_failed_step(&failed_step, verdict, || async {
                            let _compile_permit = verify_compile_permit(
                                &effective_workdir,
                                self.config.gates.compile_concurrency,
                                step,
                                &spec.plan_id,
                                &task.id,
                            )
                            .await;
                            gate.verify(&gate_signal, &gate_ctx).await
                        })
                        .await;
                }

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    gate = %verdict.gate,
                    passed = verdict.passed,
                    duration_ms = verdict.duration_ms,
                    "graph verify step completed"
                );

                // P2-LRN-6 Loop 1: Record this step's (phase, passed) outcome
                // for gate threshold EMA update after the full verify sequence.
                step_outcomes.push((step.phase.clone(), verdict.passed));

                // P2-TUI-4: Forward the gate verdict to the TUI so the
                // dashboard can display pass/fail status and captured output.
                // The command leads the output so failure summaries name it.
                if let Some(tui) = &self.tui_bridge {
                    tui.gate_result_with_output(
                        &spec.plan_id,
                        &task.id,
                        &step_label,
                        verdict.passed,
                        Some(&published_gate_output(&step.command, &verdict)),
                    );
                }

                // ── P0-04: CodingOracle observations ────────────────────
                //
                // Feed each verdict into the CodingOracle so it can refine
                // its build-time and test-pass-rate predictions.
                if let Some(oracle) = &self.feedback.coding_oracle {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let gate_lower = step.phase.to_ascii_lowercase();
                    if gate_lower.contains("compile")
                        || step.command.contains("cargo check")
                        || step.command.contains("cargo build")
                    {
                        oracle.observe_build(BuildRecord {
                            duration_secs: verdict.duration_ms as f64 / 1000.0,
                            success: verdict.passed,
                            warnings: 0,
                            ts_ms: now_ms,
                        });
                    }
                    if gate_lower.contains("test") || step.command.contains("cargo test") {
                        let (passed, failed, total) =
                            if verdict.passed { (1, 0, 1) } else { (0, 1, 1) };
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
                    let fail_msg = step.fail_msg.as_deref().unwrap_or(&verdict.reason);
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

            // ── P1-CLI-2: Compile auto-fix before agent retry ────────────
            //
            // Mirror the Runner-v2 path in gate_dispatch.rs: when verify steps
            // fail and `cargo_fix_enabled` is set (default: true), attempt
            // `cargo fix --allow-dirty` (or the build-system equivalent).
            // If the fix applies cleanly, re-run the verify steps once so the
            // caller sees the corrected result without waiting for a full agent
            // retry loop. Only applies when promise-tracker did NOT terminate
            // early (those failures are structural, not fixable by `cargo fix`).
            if !failures.is_empty() && !promise_terminated && self.config.gates.cargo_fix_enabled {
                // Use the phase of the first failing step as the gate name so
                // `attempt_auto_fix` can pick the right fix command.
                let first_fail_phase = step_outcomes
                    .iter()
                    .find(|(phase, passed)| !passed && !phase.is_empty())
                    .map_or("compile", |(phase, _)| phase.as_str());
                let raw_failures = failures.join("\n---\n");
                match crate::runner::gate_dispatch::attempt_auto_fix(
                    &effective_workdir,
                    first_fail_phase,
                    &raw_failures,
                    crate::runner::gate_dispatch::AutoFixBounds::from_config(
                        &self.config,
                        &task.files,
                    ),
                )
                .await
                {
                    Ok(outcome) if outcome.fix_applied => {
                        tracing::info!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            gate = first_fail_phase,
                            command = ?outcome.command,
                            "P1-CLI-2: auto-fix applied — re-running verify steps"
                        );
                        // Re-run verify steps with a fresh failures list.
                        // P2-LRN-6 Loop 1: Also collect retry outcomes to replace
                        // the original step_outcomes with post-fix results.
                        let mut retry_failures: Vec<String> = Vec::new();
                        let mut retry_step_outcomes: Vec<(String, bool)> = Vec::new();
                        let mut retry_skipped: Vec<String> = Vec::new();
                        for (i, step) in task.verify.iter().enumerate() {
                            let step_label = verify_step_label(i, &step.phase);
                            if !retry_failures.is_empty() {
                                retry_skipped.push(format!("{step_label} (`{}`)", step.command));
                                continue;
                            }
                            // P2-TUI-4: Notify the TUI of the post-fix re-run.
                            if let Some(tui) = &self.tui_bridge {
                                tui.gate_rung_started(&spec.plan_id, &task.id, &step_label);
                            }
                            let retry_gate = ShellGate::new(
                                "bash",
                                vec![
                                    "-o".into(),
                                    "pipefail".into(),
                                    "-c".into(),
                                    step.command.clone(),
                                ],
                            )
                            .with_timeout_ms(step.timeout_ms)
                            .with_name(&step_label);
                            let retry_verdict = retry_gate.verify(&gate_signal, &gate_ctx).await;
                            tracing::info!(
                                plan_id = %spec.plan_id,
                                task_id = %task.id,
                                step = i,
                                gate = %retry_verdict.gate,
                                passed = retry_verdict.passed,
                                duration_ms = retry_verdict.duration_ms,
                                "P1-CLI-2: post-fix verify step completed"
                            );
                            // P2-LRN-6 Loop 1: Record retry step outcome.
                            retry_step_outcomes.push((step.phase.clone(), retry_verdict.passed));

                            // P2-TUI-4: Forward post-fix verdict to the TUI.
                            if let Some(tui) = &self.tui_bridge {
                                tui.gate_result_with_output(
                                    &spec.plan_id,
                                    &task.id,
                                    &step_label,
                                    retry_verdict.passed,
                                    Some(&published_gate_output(&step.command, &retry_verdict)),
                                );
                            }
                            if !retry_verdict.passed {
                                let fail_msg =
                                    step.fail_msg.as_deref().unwrap_or(&retry_verdict.reason);
                                let detail_snippet = retry_verdict
                                    .detail
                                    .as_deref()
                                    .map(|d| {
                                        let lines: Vec<&str> = d.lines().collect();
                                        let start = lines.len().saturating_sub(30);
                                        lines[start..].join("\n")
                                    })
                                    .unwrap_or_default();
                                retry_failures.push(format!(
                                    "{step_label} (`{cmd}`): {fail_msg}\n{detail_snippet}",
                                    cmd = step.command,
                                ));
                            }
                        }
                        // Replace the original failure list and step outcomes with
                        // the post-fix results. The retry outcomes are the ground
                        // truth for gate threshold EMA updates (P2-LRN-6 Loop 1).
                        failures = retry_failures;
                        step_outcomes = retry_step_outcomes;
                        skipped_steps = retry_skipped;
                        blocked_by_sibling = None;
                    }
                    Ok(outcome) => {
                        tracing::debug!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            was_candidate = outcome.was_candidate,
                            fix_applied = outcome.fix_applied,
                            "P1-CLI-2: auto-fix not applied — proceeding to agent retry"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P1-CLI-2: auto-fix error (non-fatal) — proceeding to agent retry"
                        );
                    }
                }
            }

            if let Some(progress_tx) = progress_tx {
                let message = if failures.is_empty() {
                    "verify: all steps passed".to_string()
                } else {
                    format!(
                        "verify: {} failed, {} skipped of {total_steps}",
                        failures.len(),
                        skipped_steps.len()
                    )
                };
                let _ = progress_tx
                    .send(GraphTaskEvent::Progress {
                        message,
                        completed: Some(total_steps),
                        total: Some(total_steps),
                    })
                    .await;
            }

            // ── P2-LRN-6 Loop 1: Gate threshold EMA updates ─────────────
            //
            // Feed each completed verify step's pass/fail outcome into the
            // persisted `GateThresholds` store so the EMA converges toward the
            // workspace's actual gate history. Both pass and fail outcomes are
            // recorded; the EMA is a smoothed pass rate per rung.
            //
            // Steps: load → observe each (phase→rung) pair → atomic save →
            // notify the TUI bridge so the dashboard reflects updated EMAs.
            //
            // All I/O is synchronous and lightweight (one JSON file read+write).
            // On any error we log at warn and proceed — a missed flush is
            // non-fatal; the next task will attempt its own update.
            if let Some(gt_path) = &self.feedback.gate_thresholds_path {
                // Load existing thresholds or start from defaults if missing.
                let mut thresholds = match GateThresholds::load_or_default(gt_path) {
                    Ok(t) => t,
                    Err(err) => {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P2-LRN-6 Loop 1: gate threshold load failed (non-fatal)"
                        );
                        GateThresholds::default()
                    }
                };
                for (phase, passed) in &step_outcomes {
                    // Map the verify step's phase label to a canonical rung
                    // index using the same registry used by the Runner-v2
                    // gate pipeline (rung_for_gate_name strips attribution
                    // prefixes like "baseline:" automatically).
                    if let Some(rung) = rung_for_gate_name(phase.as_str()).map(|r| r.as_index()) {
                        thresholds.observe(rung, *passed);
                    }
                }
                match thresholds.save(gt_path) {
                    Ok(()) => {
                        // Notify the TUI bridge so the learning tab reflects
                        // the updated per-rung EMA thresholds immediately.
                        if let Some(tui) = &self.tui_bridge {
                            if let Ok(json) = serde_json::to_string(&thresholds) {
                                tui.gate_thresholds_updated(&json);
                            }
                        }
                        tracing::debug!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            steps = step_outcomes.len(),
                            "P2-LRN-6 Loop 1: gate thresholds updated"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P2-LRN-6 Loop 1: gate threshold save failed (non-fatal)"
                        );
                    }
                }
            }

            // ── Post-verify: GateGamingDetector + HoldoutExperiment ─────
            //
            // These run after all verify steps complete (or early-terminate)
            // regardless of pass/fail, matching the Runner-v2 gate completion
            // callback pattern.
            let all_passed = failures.is_empty();
            let model_slug = &dispatch.target.model_slug;

            // ── quality_judge + P1-01 GateGamingDetector (best-effort) ────
            //
            // The judge score only feeds the gaming detector: it never gates
            // the retry decision or the retry prompt, so it runs in the
            // background instead of blocking the retry. The LLM judge is only
            // consulted when verify failed and a detector is configured; all
            // passed maps to the deterministic high-quality score.
            if let Some(detector) = self.feedback.gate_gaming_detector.clone() {
                // P3-17: Modulate the judge score with daimon affect valence.
                let affect_bonus = self
                    .feedback
                    .daimon_state
                    .as_ref()
                    .and_then(|d| d.lock().ok())
                    .map(|state| state.state.alma.effective_affect().pleasure)
                    .unwrap_or(0.0);
                let judge = if all_passed { None } else { self.cheap_agent() };
                let judge_timeout = self.config.timeouts.llm_call();
                let agent_text = dispatch
                    .result
                    .output
                    .body
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                let title = spec.title.clone();
                let plan_id = spec.plan_id.clone();
                let task_id = task.id.clone();
                let model_slug = model_slug.clone();
                tokio::spawn(async move {
                    let judge_quality_score: f64 = if all_passed {
                        0.9
                    } else if let Some(cheap_agent) = judge {
                        let rubric = "Did the agent make meaningful progress toward the task even though verify steps failed?";
                        match tokio::time::timeout(
                            judge_timeout,
                            roko_learn::quality_judge::judge_quality(
                                &cheap_agent,
                                &title,
                                &agent_text,
                                rubric,
                            ),
                        )
                        .await
                        {
                            Ok(score) => {
                                tracing::debug!(
                                    plan_id = %plan_id,
                                    task_id = %task_id,
                                    model = %model_slug,
                                    quality_score = score,
                                    "quality_judge: gate output scored"
                                );
                                score
                            }
                            Err(_) => {
                                tracing::warn!(
                                    plan_id = %plan_id,
                                    task_id = %task_id,
                                    "quality_judge timed out; using heuristic score"
                                );
                                0.2
                            }
                        }
                    } else {
                        // No model configured — fall through to heuristic score.
                        0.2
                    };
                    let quality_score = (judge_quality_score + affect_bonus * 0.1).clamp(0.0, 1.0);
                    let mut det = detector.lock().await;
                    if let Err(err) = det
                        .observe_and_detect(&model_slug, all_passed, quality_score)
                        .await
                    {
                        tracing::warn!(
                            error = %err,
                            model = %model_slug,
                            "P1-01: gate gaming detection I/O error (non-fatal)"
                        );
                    }
                });
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
                // Authored verify steps are deterministic, so a failure is never
                // force-accepted: the Graph engine retries up to the task's
                // `max_retries` and then fails the task. (`gates.max_review_cycles`
                // may only bound non-deterministic review/judge verdicts, and
                // the Graph dispatcher gates on none.)
                let mut summary = verify_failure_summary(
                    &spec.title,
                    task.verify.len(),
                    &failures,
                    &skipped_steps,
                );
                // Lead with the blamed sibling so one-line failure reasons,
                // such as the episode's, keep it.
                if let Some(sibling) = &blocked_by_sibling {
                    summary = format!("blocked_by_sibling = {sibling}: {summary}");
                }
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    failed_count = failures.len(),
                    skipped_count = skipped_steps.len(),
                    total_count = task.verify.len(),
                    attempt = attempt_number,
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
                            attempt_id: format!("{attempt_id}/gate-fail"),
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
                        if let Ok(line) = serde_json::to_string(&gate_event) {
                            let path = eff_path.clone();
                            tokio::spawn(async move {
                                if let Err(error) = append_jsonl_line_async(path, line).await {
                                    tracing::warn!(
                                        %error,
                                        "graph gate-failure efficiency event write failed"
                                    );
                                }
                            });
                        }
                    }
                }
                // ── error_enrichment: enrich gate failure before retry ───
                //
                // Ask a cheap judge model for a two-sentence diagnosis of the
                // raw failure so the retry prompt carries a focused summary
                // rather than raw compiler noise. Falls back deterministically
                // when no model is configured or the call fails.
                let raw_for_feedback = failures.join("\n---\n");
                // The diagnosis feeds the retry prompt, so it stays inline but
                // is bounded by `timeouts.llm_call_secs`; on timeout the retry
                // proceeds with the raw gate output alone.
                let enriched_diagnosis = if let Some(cheap_agent) = self.cheap_agent() {
                    tokio::time::timeout(
                        self.config.timeouts.llm_call(),
                        roko_learn::error_enrichment::enrich_error_digest(
                            &raw_for_feedback,
                            &cheap_agent,
                            &spec.title,
                        ),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            "error enrichment timed out; retrying with raw gate output"
                        );
                        String::new()
                    })
                } else {
                    String::new()
                };

                // ── Store gate feedback for retry injection ─────────────
                //
                // Parse the raw failure text into structured GateFeedback
                // and store it keyed by task so the next dispatch attempt
                // can inject the errors into the agent's prompt.
                // Prepend the enriched diagnosis to raw_output so the prompt
                // builder surfaces the focused summary ahead of the raw output.
                let feedback_raw = if enriched_diagnosis.is_empty() {
                    raw_for_feedback.clone()
                } else {
                    format!("Diagnosis: {enriched_diagnosis}\n\n{raw_for_feedback}")
                };
                if let Some(feedback) = GateFeedback::from_raw(&feedback_raw) {
                    let next_attempt = attempt_number.saturating_add(1);
                    tracing::info!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        compile_errors = feedback.compile_errors.len(),
                        test_failures = feedback.test_failures.len(),
                        clippy_warnings = feedback.clippy_warnings.len(),
                        has_enriched_diagnosis = !enriched_diagnosis.is_empty(),
                        next_attempt,
                        "storing gate feedback for retry injection"
                    );
                    self.gate_retry_context
                        .lock()
                        .insert(retry_key.clone(), (feedback, next_attempt));
                }
                // ── W13: Persist structured gate failure record ──────────
                //
                // Classify the raw failure text and append a GateFailureRecord
                // to `.roko/learn/gate-failures.jsonl` for fast triage and
                // adaptive threshold learning (#218).
                if let Some(gf_path) = &self.feedback.gate_failures_path {
                    let raw_for_classification = failures.join("\n---\n");
                    let classification =
                        roko_gate::classify_gate_failure("graph-verify", &raw_for_classification);
                    let record = roko_gate::GateFailureRecord::from_classification(
                        &spec.plan_id,
                        &task.id,
                        "graph-verify",
                        0,
                        &classification,
                    );
                    if let Ok(line) = serde_json::to_string(&record) {
                        let path = gf_path.clone();
                        tokio::spawn(async move {
                            if let Err(error) = append_jsonl_line_async(path, line).await {
                                tracing::warn!(
                                    %error,
                                    "gate failure record write failed (non-fatal)"
                                );
                            }
                        });
                    }
                }
                // ── P2-PLN-2: Post-gate LLM reflection ───────────────────
                //
                // When `replan_on_gate_failure` is enabled and a cheap
                // agent is available, ask the LLM for a one-sentence
                // reflection explaining the root cause. The lesson is
                // stored in the PostGateReflectionStore (at
                // `.roko/learn/post-gate-reflections.json`) so subsequent
                // retry prompts and playbook extraction see real LLM
                // analysis instead of the deterministic pattern template.
                if self.feedback.replan_on_gate_failure {
                    if let Some((reflection_path, cheap_agent)) = self
                        .feedback
                        .post_gate_reflection_path
                        .as_ref()
                        .cloned()
                        .zip(self.cheap_agent())
                    {
                        let raw_for_reflection = failures.join("\n---\n");
                        let task_desc = spec.title.clone();
                        let plan_id = spec.plan_id.clone();
                        let task_id = task.id.clone();
                        tokio::spawn(async move {
                            let lesson =
                                roko_learn::post_gate_reflection::generate_post_gate_reflection(
                                    &cheap_agent,
                                    &task_desc,
                                    "graph-verify",
                                    &raw_for_reflection,
                                )
                                .await;
                            tracing::info!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                lesson_chars = lesson.len(),
                                "post-gate LLM reflection generated"
                            );
                            let input = roko_learn::post_gate_reflection::ReflectionInput {
                                plan_id: Some(plan_id),
                                task_id: Some(task_id),
                                episode_id: None,
                                trigger_gate: "graph-verify".to_string(),
                                outcome:
                                    roko_learn::post_gate_reflection::ReflectionGateOutcome::Failed,
                                failure_pattern_ids: vec![],
                                pass_evidence: vec![],
                                proposed_lesson: lesson,
                            };
                            let mut store =
                                roko_learn::post_gate_reflection::PostGateReflectionStore::load(
                                    &reflection_path,
                                );
                            store.observe(
                                input,
                                roko_learn::post_gate_reflection::ReflectionPromotionConfig::default(),
                            );
                            if let Err(error) = store.save(&reflection_path) {
                                tracing::warn!(
                                    %error,
                                    "post-gate reflection store write failed (non-fatal)"
                                );
                            }
                        });
                    }
                }
                // ── RAG-10/11: Retrieval outcome settlement (gate fail) ──
                {
                    let ctx_snapshot = self.retrieval_ctx.lock().get(&retry_key).cloned();
                    if let Some((strategy, query, results_count, latency_ms)) = ctx_snapshot {
                        // RAG-11: update experiment store with gate-fail outcome.
                        if let Some(exp_path) = &self.feedback.experiment_store_path {
                            // Locked: prompt treatments share the file.
                            let _ = roko_learn::prompt_experiment::ExperimentStore::transaction(
                                exp_path,
                                |store| {
                                    store.record_retrieval_outcome(&strategy, false);
                                    Ok(())
                                },
                            );
                        }
                        // RAG-10: write settled record.
                        if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                            let record =
                                roko_learn::retrieval_outcome::RetrievalOutcomeRecord::settled(
                                    &spec.plan_id,
                                    &task.id,
                                    &query,
                                    &strategy,
                                    results_count,
                                    false,
                                )
                                .with_latency_ms(latency_ms);
                            tokio::spawn(async move {
                                if let Err(error) =
                                    roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                                        .without_fsync()
                                        .append(&record)
                                        .await
                                {
                                    tracing::warn!(
                                        %error,
                                        "RAG-10: gate-fail retrieval outcome write failed (best-effort)"
                                    );
                                }
                            });
                        }
                    }
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
            // ── P0-GA-1: Emit gate-pass efficiency event ──────────────────
            //
            // The initial efficiency event (W05 in emit_feedback) is written
            // before gate execution with gate_passed: None, so the metric was
            // always 0%. Write a follow-up record now that we know all verify
            // steps passed so readers that filter by gate_passed == Some(true)
            // see the correct pass count.
            if let Some(eff_path) = &self.feedback.efficiency_path {
                let gate_turn_number = dispatch
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
                let gate_pass_event = roko_learn::efficiency::AgentEfficiencyEvent {
                    agent_id: format!("{}/{}", spec.plan_id, task.id),
                    role: task.role.as_deref().unwrap_or("implementer").to_string(),
                    backend: dispatch.target.provider_id.clone(),
                    model: dispatch.target.model_slug.clone(),
                    plan_id: spec.plan_id.clone(),
                    task_id: task.id.clone(),
                    // Suffixed so it stays distinct from, yet joins, the
                    // attempt's dispatch event.
                    attempt_id: format!("{attempt_id}/gate-pass"),
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
                    is_final_turn: true,
                    gate_passed: Some(true),
                    outcome: "gate_pass".to_string(),
                    gate_errors: vec![],
                    model_used: dispatch.target.model_slug.clone(),
                    frequency: roko_core::OperatingFrequency::Gamma,
                    strategy_attempted: String::new(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                if let Ok(line) = serde_json::to_string(&gate_pass_event) {
                    let path = eff_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    tokio::spawn(async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph gate-pass efficiency event write failed (best-effort)"
                            );
                        }
                    });
                }
            }
            // ── RAG-10/11: Retrieval outcome settlement (gate pass) ───────
            {
                let ctx_snapshot = self.retrieval_ctx.lock().get(&retry_key).cloned();
                if let Some((strategy, query, results_count, latency_ms)) = ctx_snapshot {
                    // RAG-11: update experiment store with gate-pass outcome.
                    if let Some(exp_path) = &self.feedback.experiment_store_path {
                        // Locked: prompt treatments share the file.
                        let _ = roko_learn::prompt_experiment::ExperimentStore::transaction(
                            exp_path,
                            |store| {
                                store.record_retrieval_outcome(&strategy, true);
                                Ok(())
                            },
                        );
                    }
                    // RAG-10: write settled record.
                    if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                        let record =
                            roko_learn::retrieval_outcome::RetrievalOutcomeRecord::settled(
                                &spec.plan_id,
                                &task.id,
                                &query,
                                &strategy,
                                results_count,
                                true,
                            )
                            .with_latency_ms(latency_ms);
                        tokio::spawn(async move {
                            if let Err(error) =
                                roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                                    .without_fsync()
                                    .append(&record)
                                    .await
                            {
                                tracing::warn!(
                                    %error,
                                    "RAG-10: gate-pass retrieval outcome write failed (best-effort)"
                                );
                            }
                        });
                    }
                }
            }
            // Clear any stale gate retry context on success.
            self.gate_retry_context.lock().remove(&retry_key);
            self.retrieval_ctx.lock().remove(&retry_key);
        }

        Ok(if task.verify.is_empty() {
            TaskGateVerdict::Unverified
        } else {
            TaskGateVerdict::Passed
        })
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

        // `agent_spawned` is now published immediately before dispatch starts
        // (see the `dispatch` fn). Do not publish it here to avoid a duplicate.

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
                    //
                    // Safety: floor the slice start to a char boundary so we
                    // never split a multi-byte character, which would panic.
                    let truncated = if output.len() > 2048 {
                        let raw_start = output.len().saturating_sub(1024);
                        // Walk backwards until we land on a char boundary.
                        let char_start = (0..=raw_start)
                            .rev()
                            .find(|&i| output.is_char_boundary(i))
                            .unwrap_or(0);
                        let tail = &output[char_start..];
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

/// Forward a single [`roko_agent::live_output::LiveAgentEvent`] to the TUI
/// bridge.
///
/// Called from the forwarder task spawned alongside the heartbeat task.
/// Mapping:
/// - `ToolStep` → `TuiBridge::tool_step` (always; safe, scrubbed target)
/// - `Unscreened(TextDelta)` → unscreened `text` record
/// - `Unscreened(ReasoningDelta)` → unscreened `reasoning` record
/// - `Unscreened(ToolCallEnd)` → unscreened `tool_start` record with args
///   truncated to 2 048 bytes on a char boundary
/// - `Unscreened(ToolResult)` → unscreened `tool_result` record with output
///   truncated the same way `forward_dispatch_events_to_tui` truncates it
/// - All other `Unscreened` variants are silently ignored.
fn forward_live_event_to_tui(
    tui: &TuiBridge,
    agent_id: &str,
    plan_id: &str,
    task_id: &str,
    event: roko_agent::live_output::LiveAgentEvent,
) {
    use roko_agent::StreamEventKind;
    use roko_agent::live_output::LiveAgentEvent;

    const ARGS_MAX_BYTES: usize = 2048;

    match event {
        LiveAgentEvent::ToolStep { id, name, target } => {
            tui.tool_step(agent_id, plan_id, task_id, 0, &id, &name, &target);
        }
        LiveAgentEvent::Unscreened(kind) => match kind {
            StreamEventKind::TextDelta(text) => {
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "text",
                    serde_json::json!({"text": text}),
                );
            }
            StreamEventKind::ReasoningDelta(text) => {
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "reasoning",
                    serde_json::json!({"text": text}),
                );
            }
            StreamEventKind::ToolCallEnd { id, name, args } => {
                // Serialize args and truncate to ARGS_MAX_BYTES on a char
                // boundary to avoid overwhelming the TUI ring buffer.
                let args_str = serde_json::to_string(&args).unwrap_or_default();
                let args_truncated = if args_str.len() > ARGS_MAX_BYTES {
                    let cut = (0..=ARGS_MAX_BYTES)
                        .rev()
                        .find(|&i| args_str.is_char_boundary(i))
                        .unwrap_or(0);
                    format!("{}…", &args_str[..cut])
                } else {
                    args_str
                };
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "tool_start",
                    serde_json::json!({
                        "tool_id": id,
                        "tool": name,
                        "args": args_truncated,
                    }),
                );
            }
            StreamEventKind::ToolResult { id, output } => {
                // Truncate tool output the same way forward_dispatch_events_to_tui
                // does: keep the last 1 024 bytes (aligned to a char boundary).
                let truncated = if output.len() > 2048 {
                    let raw_start = output.len().saturating_sub(1024);
                    let char_start = (0..=raw_start)
                        .rev()
                        .find(|&i| output.is_char_boundary(i))
                        .unwrap_or(0);
                    let tail = &output[char_start..];
                    format!("[...truncated]\n{tail}")
                } else {
                    output
                };
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "tool_result",
                    serde_json::json!({"tool_id": id, "output": truncated}),
                );
            }
            // All other stream event kinds (ToolCallStart, ToolCallDelta,
            // Usage, Done) are not forwarded as unscreened records.
            _ => {}
        },
    }
}

/// Append a single JSON line to a JSONL file, creating parent dirs as needed.
#[allow(dead_code)] // sync fallback; production paths use append_jsonl_line_async
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

/// Async wrapper for [`append_jsonl_line`].
///
/// Serializes `value` on the calling async task (cheap), then offloads the
/// blocking file I/O to a `spawn_blocking` thread so the Tokio reactor is
/// not stalled on disk writes inside `async fn emit_feedback`.
async fn append_jsonl_line_async(path: std::path::PathBuf, line: String) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{line}")?;
        file.flush()?;
        Ok(())
    })
    .await
    .unwrap_or_else(|join_err| Err(std::io::Error::new(std::io::ErrorKind::Other, join_err)))
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

/// P1-18: Convert persisted dream routing advice to a `RoutingBias` for the
/// cascade router. Returns `None` when no advice was loaded (missing or
/// stale file) or no recommendations match the task category.
fn dream_routing_bias(
    advice: Option<&roko_dreams::DreamRoutingAdvice>,
    task_category: &str,
    routing_ctx: &roko_learn::model_router::RoutingContext,
) -> Option<roko_learn::cascade_router::RoutingBias> {
    let advice = advice?;
    if advice.recommendations.is_empty() {
        return None;
    }
    let complexity_band = routing_ctx.complexity.label();
    let bias = roko_dreams::dream_advice_to_routing_bias(advice, task_category, complexity_band);
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

/// RAG-11: assign the retrieval-strategy arm from the experiment store.
///
/// Blocking file I/O: call it from `spawn_blocking`. Assignment is a pure
/// read of the persisted arm statistics. The store is written only to
/// register the experiment, and then under its lock, so the prompt
/// treatments parallel attempts record in the same file are never lost.
fn assign_retrieval_strategy_arm(exp_path: &Path) -> String {
    use roko_learn::prompt_experiment::ExperimentStore;

    let mut store = ExperimentStore::load_or_new(exp_path);
    if store
        .get(ExperimentStore::RETRIEVAL_STRATEGY_EXPERIMENT_ID)
        .is_none()
    {
        store.ensure_retrieval_strategy_experiment();
        if let Err(error) = ExperimentStore::transaction(exp_path, |locked| {
            locked.ensure_retrieval_strategy_experiment();
            Ok(())
        }) {
            tracing::debug!(
                %error,
                "RAG-11: persisting the retrieval-strategy experiment failed (best-effort)"
            );
        }
    }
    store
        .assign_retrieval_strategy()
        .unwrap_or_else(|| roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string())
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
        let task_spend_key = format!("{}/{}", spec.plan_id, task.id);
        self.admit_task_budget(spec, &task, &task_spend_key)?;

        // ── Role-enabled check ──────────────────────────────────────────
        //
        // When a role is disabled via `[agent.roles.<role>] enabled = false`,
        // skip the task with a warning rather than failing it.
        if let Some(role_label) = task.role.as_deref() {
            if !crate::config_helpers::is_role_enabled(&self.config, role_label) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    role = role_label,
                    "role is disabled in config; skipping task"
                );
                // Return an empty signal vec — the graph engine treats this
                // as a completed (no-output) cell, not a failure.
                return Ok(Vec::new());
            }
        }

        // ── T0 reflex check ─────────────────────────────────────────────
        //
        // Before invoking the LLM, check the reflex store for a matching
        // deterministic rule. A rule fires when every populated field of its
        // `ReflexCondition` matches the task's observable attributes.  The
        // observation is built from the task's role (→ `message_type`), title
        // (→ `context`), and the unique file extensions present in `task.files`
        // (→ `file_exts`).  When a rule matches:
        //
        //   1. The agent call is skipped entirely.
        //   2. The rule's `action.args` field is used as the cached output text.
        //   3. Gate feedback (`pass` / `fail`) is posted to the same rule_id
        //      so the store accumulates confidence or demotes the rule.
        //
        // This implements the "zero-cost repeated decisions" pattern: tasks
        // that succeed repeatedly with the same structural signature can be
        // served from the T0 store without an LLM round-trip.
        //
        // Reflexes skip the provider *and* the verify steps, so they only
        // serve tasks that author no verification: a verify-bearing task must
        // earn its pass from its own gates.
        if let Some(reflex_store) = self
            .reflex_store
            .as_ref()
            .filter(|_| task.verify.is_empty())
        {
            let file_exts: Vec<String> = task
                .files
                .iter()
                .filter_map(|f| {
                    std::path::Path::new(f)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| format!(".{ext}"))
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let observation = ReflexObservation {
                tool: None,
                args: None,
                context: Some(task.title.clone()),
                message_type: task.role.clone(),
                file_exts,
            };

            if let Some(reflex_match) = reflex_store.match_observation_with_id(&observation) {
                let rule_id = reflex_match.rule_id;
                let cached_output = reflex_match.action.args.clone();
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %rule_id,
                    "T0 reflex matched: skipping LLM dispatch, using cached output"
                );
                // Settle the budget reservation at zero cost (no LLM call).
                budget_reservation.settle(0.0)?;
                let output_signal = Signal::builder(Kind::AgentOutput)
                    .body(Body::text(cached_output))
                    .build();
                // Post gate feedback after we run the verify steps. For now
                // we immediately record a gate pass since the reflex is only
                // wired here on the success path. Gate failure from verify
                // steps will trigger the graph engine retry, which will avoid
                // the reflex (attempt_number > 0) or rely on demotion logic.
                reflex_store.record_gate_pass_for(rule_id);
                let mut outputs = vec![output_signal];
                TaskGateVerdict::Unverified.stamp(&mut outputs);
                return Ok(outputs);
            }
        }

        // A plan with `[meta] skip_enrichment = true` is dispatched as
        // authored: no eval artifacts and no dream/cross-cut routing advice.
        let skip_enrichment = self.plan_skips_enrichment(spec);

        // ── P0-02: EvalGenerator pre-dispatch ───────────────────────────
        //
        // For standard-tier and above tasks, generate evaluation test
        // artifacts before the agent starts. Opt-in via
        // `gates.write_eval_artifacts`, because nothing in `plan run`
        // executes them. Enabled artifacts go to `.roko/generated-tests/`
        // (read by the Runner-v2 generated-test rung), not the repo root.
        if self.feedback.eval_generation_enabled
            && self.config.gates.write_eval_artifacts
            && !skip_enrichment
        {
            let tier_lower = task.tier.to_ascii_lowercase();
            let is_standard_or_above = !matches!(tier_lower.as_str(), "mechanical" | "trivial");
            if is_standard_or_above {
                let target_crates = crate::task_helpers::task_target_crates(Some(&task));
                let primary_crate = target_crates
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "roko-cli".to_string());
                let generator = EvalGenerator::new();
                let evals = generator.generate_all(&task.title, &primary_crate, &task.files);
                if !evals.is_empty() {
                    let gen_dir = self.workdir.join(".roko").join("generated-tests");
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
            let lease = provider
                .acquire(&attempt_id)
                .await
                .map_err(|e| RokoError::Agent {
                    backend: "worktree-isolation".to_string(),
                    message: format!("failed to acquire worktree for {attempt_id}: {e}"),
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
        // Until this attempt ends, a sibling's failed verify step in the same
        // working tree may wait for it to settle.
        let _in_flight = self
            .in_flight
            .register(&task_spend_key, &effective_workdir, &task.files);

        let role = task.role.as_deref().unwrap_or("implementer");

        // ── P3-AGT-2: Express mode check ────────────────────────────────
        //
        // When `conductor.express_mode = true` and the task tier is
        // "mechanical" or "trivial", bypass the full routing pipeline:
        //   - Route to `routing.fast_task_model` (cheapest available model).
        //   - Cap the agent turn limit at EXPRESS_MAX_TURNS (5).
        //   - Eval-generation is already skipped by the tier check above.
        // A CLI `--model` override (`cli_model_override`) takes precedence over
        // express routing so manual experiments are not silently replaced.
        let express_active = is_express_task(&self.config, &task);
        let mut max_turns = task_turn_limit(&self.config, &task, express_active);
        // The last attempt stopped at its turn cap with partial work on disk:
        // raise the cap and tell the agent to resume, never rerun the same cap.
        let turn_cap_resume = self.turn_cap_retries.lock().remove(&task_spend_key);
        if let Some(previous) = turn_cap_resume {
            max_turns = max_turns.max(raised_turn_cap(previous.cap));
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                previous_cap = previous.cap,
                max_turns,
                "previous attempt hit its turn cap; resuming with a raised cap"
            );
        }
        if express_active {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                tier = %task.tier,
                fast_model = %self.config.routing.fast_task_model,
                max_turns,
                "P3-AGT-2: express mode active — routing to fast model with reduced turn limit"
            );
        }

        // ── W10: Enrichment pipeline ─────────────────────────────────────
        let routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);
        // Clone before the move into DispatchContext so emit_feedback can pass
        // the real dispatch-time context to the routing observation sink.
        // This ensures force_backend override outcomes are recorded with the
        // correct task category, complexity, and role rather than fallback defaults.
        let routing_ctx_for_feedback = routing_ctx.clone();

        // Load persisted dream routing advice once; both the cross-cut
        // arbitration and the P1-18 dream bias read it. Plans that skip
        // enrichment get neither.
        let routing_bias = if skip_enrichment {
            None
        } else {
            let dream_advice = roko_dreams::load_dream_routing_advice(&self.workdir).ok();
            // P1-16: Run cross-cut arbitration to detect safety-critical
            // overrides before applying dream routing advice.
            let task_category = task.domain.as_ref().map_or("implementation", |d| d.label());
            let arbitration_bias = arbitrate_cross_cut_routing_bias(
                &self.feedback,
                dream_advice.as_ref(),
                task_category,
            );

            // P1-18: Convert the dream advice to a RoutingBias so the cascade
            // router accounts for dream-observed model performance when
            // picking a provider for this task. Arbitration safety overrides
            // take priority over dream advice.
            arbitration_bias
                .or_else(|| dream_routing_bias(dream_advice.as_ref(), task_category, &routing_ctx))
        };

        // ── Gate retry context lookup ──────────────────────────────────
        //
        // If this task was previously dispatched and failed verification,
        // the gate_retry_context map holds the structured errors and attempt
        // count. Injecting this into the DispatchContext causes the prompt
        // assembler to include a "Previous attempt feedback" section with
        // the actual compile/test/clippy errors so the agent can fix them.
        let retry_key = format!("{}/{}", spec.plan_id, task.id);
        let (prior_gate_feedback, attempt_number) = self
            .gate_retry_context
            .lock()
            .get(&retry_key)
            .cloned()
            .map(|(fb, attempt)| (Some(fb), attempt))
            .unwrap_or((None, 0));
        let efficiency_attempt_id = self.next_attempt_id(&retry_key);

        if attempt_number > 0 {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                attempt = attempt_number,
                has_gate_feedback = prior_gate_feedback.is_some(),
                "graph dispatch: injecting gate feedback from previous attempt"
            );
        }

        let (cached_workspace_map, cached_workspace_context, cached_cfactor_context) =
            self.static_prompt_cache.get_or_init(|| {
                let ws_map =
                    crate::dispatch::prompt_builder::generate_workspace_map_pub(&self.workdir);
                let ws_ctx =
                    crate::dispatch::prompt_builder::generate_workspace_context_pub(&self.workdir);
                let cf_ctx =
                    crate::dispatch::prompt_builder::generate_cfactor_context_pub(&self.workdir);
                tracing::debug!(
                    ws_map_bytes = ws_map.len(),
                    ws_ctx_bytes = ws_ctx.len(),
                    cf_ctx_bytes = cf_ctx.len(),
                    "static_prompt_cache: computed once for this run"
                );
                (ws_map, ws_ctx, cf_ctx)
            });
        // Express mode sets force_backend to the fast model unless the operator
        // has already supplied a --model override (cli_model_override takes
        // priority so manual experiments are not silently replaced).
        let express_force_backend = if express_active && self.cli_model_override.is_none() {
            Some(self.config.routing.fast_task_model.clone())
        } else {
            None
        };
        // Durable, attempt-scoped prompt treatments from the root workspace's
        // experiment store; settled with the attempt's outcome in
        // `emit_feedback`.
        let prompt_experiment = self
            .feedback
            .experiment_store_path
            .as_deref()
            .and_then(|store| {
                prompt_experiment::context(store, &spec.plan_id, &task.id, &efficiency_attempt_id)
            });
        let mut dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: effective_workdir.clone(),
            // Task-authored model_hint flows through to RoutingInputs where it
            // beats the cascade router but loses to force_backend.  When the
            // task has no hint we leave this None so the cascade router can
            // make its own decision rather than short-circuiting to the config
            // default.
            model_hint: task.model_hint.clone(),
            force_backend: self.cli_model_override.clone().or(express_force_backend),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: attempt_number,
            prompt_experiment: prompt_experiment.clone(),
            gate_feedback: prior_gate_feedback,
            routing_context: Some(routing_ctx),
            routing_bias,
            dependency_outputs: upstream_outputs(&input),
            error_patterns_context: self.factory.format_error_patterns_for_prompt(5),
            cached_workspace_map: cached_workspace_map.clone(),
            cached_workspace_context: cached_workspace_context.clone(),
            cached_cfactor_context: cached_cfactor_context.clone(),
        };
        let prompt_assembly_started = std::time::Instant::now();
        let dispatch_plan = self.plan_dispatch(spec, &task, &mut dispatch_ctx)?;
        let prompt_assembly_latency_ms = prompt_assembly_started.elapsed().as_millis() as u64;

        // ── RAG-10/11: Retrieval outcome telemetry (pre-gate) ────────────
        //
        // Immediately after prompt assembly we know:
        //   - which strategy was used (RAG-11 experiment assignment or default)
        //   - how many knowledge entries were retrieved (diagnostics.knowledge_ids)
        //   - the query text (task title + description)
        //   - prompt assembly latency (covers neuro knowledge retrieval)
        //
        // We record a pre-gate record now and a settled record after verify.
        {
            let results_count = dispatch_plan.prompt.diagnostics.knowledge_ids.len();
            let query = format!(
                "{} {}",
                task.title,
                task.description.as_deref().unwrap_or("")
            )
            .trim()
            .to_string();

            // RAG-11: assign retrieval strategy via experiment store, or fall
            // back to the default "keyword" arm (which is what the current
            // `collect_neuro_knowledge_cached` always runs). The store read is
            // blocking file I/O, so it runs off the reactor.
            let strategy = if let Some(exp_path) = self.feedback.experiment_store_path.clone() {
                tokio::task::spawn_blocking(move || assign_retrieval_strategy_arm(&exp_path))
                    .await
                    .unwrap_or_else(|_| roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string())
            } else {
                roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string()
            };

            // Stash for gate-settlement below.
            self.retrieval_ctx.lock().insert(
                retry_key.clone(),
                (
                    strategy.clone(),
                    query.clone(),
                    results_count,
                    prompt_assembly_latency_ms,
                ),
            );

            // Write the pre-gate record (best-effort, non-blocking).
            if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                let record = roko_learn::retrieval_outcome::RetrievalOutcomeRecord::pre_gate(
                    &spec.plan_id,
                    &task.id,
                    &query,
                    &strategy,
                    results_count,
                )
                .with_latency_ms(prompt_assembly_latency_ms);
                tokio::spawn(async move {
                    if let Err(error) =
                        roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                            .without_fsync()
                            .append(&record)
                            .await
                    {
                        tracing::warn!(
                            %error,
                            "RAG-10: pre-gate retrieval outcome write failed (best-effort)"
                        );
                    }
                });
            }
        }

        let contract = effective_agent_contract(role, &task);
        let effective_timeout_secs = if spec.timeout_secs == 0 {
            self.config.timeouts.agent_dispatch_secs
        } else {
            spec.timeout_secs
        };
        let timeout_ms = effective_timeout_secs.max(1).saturating_mul(1_000);
        let mut request = AgentDispatchRequest {
            model_key: dispatch_plan.model.slug.clone(),
            prompt: match turn_cap_resume {
                Some(previous) => format!(
                    "{}{}",
                    dispatch_plan.prompt.user_prompt,
                    turn_cap_resume_note(previous, max_turns)
                ),
                None => dispatch_plan.prompt.user_prompt.clone(),
            },
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
            // Tier turn cap from `[pipeline.<tier>] max_turns`, lowered to
            // EXPRESS_MAX_TURNS for express tasks and raised after a turn-cap
            // stop. Never unbounded.
            max_turns: Some(max_turns),
            live_output: None,
        };

        // Bind the prompt treatments to the exact final prompt before launch;
        // an attempt that ends before `emit_feedback` abandons them on drop.
        let _launched_treatments = prompt_experiment::LaunchedTreatments::bind(
            prompt_experiment,
            &dispatch_plan.prompt.diagnostics.experiment_assignments,
            &request.system_prompt,
            &request.prompt,
        )
        .await;

        // ── T04: Pre-dispatch agent_spawned ─────────────────────────────
        //
        // Publish `agent_spawned` immediately so the TUI shows the agent as
        // active before the (potentially multi-minute) provider call starts.
        // `forward_dispatch_events_to_tui` no longer emits it.
        let pre_dispatch_agent_id = format!(
            "{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or(&task.id)
        );
        if let Some(tui) = &self.tui_bridge {
            // Derive a provider label from the planned backend so the dashboard
            // can display it before the actual dispatch resolves a provider.
            let planned_provider: String =
                roko_core::ProviderKind::from(dispatch_plan.model.backend)
                    .label()
                    .to_string();
            tui.agent_spawned(
                &pre_dispatch_agent_id,
                &spec.plan_id,
                &task.id,
                0,
                task.role.as_deref().unwrap_or("implementer"),
                &dispatch_plan.model.slug,
                &planned_provider,
            );
        }

        // ── Live output forwarder ──────────────────────────────────────
        //
        // When both a TUI bridge and a live-output setting are configured,
        // create a bounded channel, attach it to the request so the immune
        // boundary can push events while the agent runs, and spawn a task
        // that forwards each event to the TUI before screening completes.
        // `forward_dispatch_events_to_tui` still publishes the screened
        // transcript after `run_bridge_with_failover` returns (§4).
        if let (Some(tui), Some(live_setting)) = (&self.tui_bridge, &self.live_agent_output) {
            let (live_tx, mut live_rx) =
                tokio::sync::mpsc::channel::<roko_agent::live_output::LiveAgentEvent>(64);
            let trusted = matches!(live_setting, LiveAgentOutput::Trusted);
            request.live_output = Some(roko_agent::live_output::LiveOutput {
                sink: live_tx,
                trusted,
            });
            let tui_clone = tui.clone();
            let agent_id_clone = pre_dispatch_agent_id.clone();
            let plan_id_clone = spec.plan_id.clone();
            let task_id_clone = task.id.clone();
            tokio::spawn(async move {
                while let Some(event) = live_rx.recv().await {
                    forward_live_event_to_tui(
                        &tui_clone,
                        &agent_id_clone,
                        &plan_id_clone,
                        &task_id_clone,
                        event,
                    );
                }
            });
        }

        let started_at = Instant::now();
        // The planned model is a preference: an unusable or out-of-usage
        // provider fails over; `dispatch.target` names the model that ran.
        //
        // T04: Drive the dispatch future through a select loop so we can emit
        // periodic `agent_heartbeat` events while waiting.  This keeps the
        // elapsed-time counter live on the TUI even though the transcript is
        // only available after the immune boundary screens the final result.
        let dispatch_result = {
            let mut heartbeat = tokio::time::interval(AGENT_HEARTBEAT_INTERVAL);
            heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // Consume the immediate first tick so we don't fire at t=0.
            heartbeat.tick().await;
            let dispatch_future = self.run_bridge_with_failover(spec, &task.id, request);
            tokio::pin!(dispatch_future);
            loop {
                tokio::select! {
                    result = &mut dispatch_future => { break result; }
                    _ = heartbeat.tick() => {
                        let elapsed_ms = started_at.elapsed().as_millis() as u64;
                        if let Some(tui) = &self.tui_bridge {
                            tui.agent_heartbeat(
                                &pre_dispatch_agent_id,
                                &spec.plan_id,
                                &task.id,
                                elapsed_ms,
                            );
                        }
                    }
                }
            }
        };
        let dispatch = dispatch_result.map_err(|error| {
            // Best-effort release on dispatch failure when worktree isolation is active.
            if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                let provider = Arc::clone(provider);
                let lease = lease.clone();
                tokio::spawn(async move {
                    let _ = provider
                        .release(
                            &lease,
                            roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                        )
                        .await;
                });
            }
            // T04: Publish agent_completed on the error path so the dashboard
            // never leaves an agent stuck in the "running" state.
            if let Some(tui) = &self.tui_bridge {
                tui.agent_completed(&pre_dispatch_agent_id, &spec.plan_id, &task.id, 0);
            }
            error
        })?;
        let wall_duration = started_at.elapsed();

        // Account for every completed provider call, including unsuccessful
        // results: callers may still have incurred the reported cost.
        self.task_spend
            .record(&task_spend_key, f64::from(dispatch.result.usage.cost_usd));
        budget_reservation.settle(f64::from(dispatch.result.usage.cost_usd))?;

        // ── TUI streaming output ─────────────────────────────────────────
        //
        // Forward provider dispatch events (text deltas, tool calls, tool
        // outputs) to the TUI bridge so the dashboard shows what the agent
        // produced. This runs for both successful and failed dispatches.
        self.forward_dispatch_events_to_tui(spec, &task, &dispatch, ctx);

        if !dispatch.result.success {
            let message = dispatch
                .result
                .output
                .body
                .as_text()
                .unwrap_or("provider returned an unsuccessful result")
                .to_string();
            // A failed provider call is settled now; a successful one is
            // settled after its verify steps so learning sees the verified
            // outcome.
            self.emit_feedback(
                spec,
                &task,
                &efficiency_attempt_id,
                &dispatch,
                false,
                wall_duration,
                &dispatch_plan,
                Some(routing_ctx_for_feedback),
                Some(provider_failure_reason(&message)),
            )
            .await;
            // Release worktree with RetainForFailure policy for post-mortem.
            if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                let _ = provider
                    .release(
                        lease,
                        roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                    )
                    .await;
            }

            // Detect billing/credit errors and log a clear warning so
            // operators see the root cause. The in-memory health registry
            // has already been updated by the dispatch layer (dispatch_v2),
            // which marks the provider as Open-circuit with a 24-hour
            // cooldown, so the next retry will route to a different provider
            // through the cascade router.
            let message_lower = message.to_ascii_lowercase();
            if roko_agent::provider::error_classify::is_billing_message(&message_lower) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    provider = %dispatch.target.provider_id,
                    "provider {} returned billing error: {}. \
                     Marking as unavailable for this run.",
                    dispatch.target.provider_id,
                    message
                );
            }

            if let Some(hit) = roko_agent::provider::error_classify::detect_turn_cap(&message) {
                self.turn_cap_retries.lock().insert(
                    task_spend_key.clone(),
                    TurnCapRetry {
                        cap: max_turns,
                        num_turns: hit.num_turns,
                    },
                );
                return Err(RokoError::TurnLimitReached {
                    backend: dispatch.target.provider_id,
                    limit: max_turns,
                    num_turns: hit.num_turns.unwrap_or(max_turns),
                });
            }
            return Err(RokoError::Agent {
                backend: dispatch.target.provider_id,
                message,
            });
        }

        // ── Verify steps (gate execution) ──────────────────────────────
        //
        // Authored [[task.verify]] steps gate the task in the effective
        // workdir (worktree if isolated). A failure fails this attempt so the
        // Graph engine can retry or abort; it is never force-accepted.
        let verification = self
            .settle_task_verification(
                spec,
                &task,
                &dispatch,
                &effective_workdir,
                &retry_key,
                attempt_number,
                &efficiency_attempt_id,
                None,
            )
            .await;

        // ── Learning/feedback pipeline ───────────────────────────────────
        //
        // Settled after the gate so episodes, routing, playbooks, affect, and
        // experiments learn from the verified outcome rather than from the
        // provider dispatch result.
        self.emit_feedback(
            spec,
            &task,
            &efficiency_attempt_id,
            &dispatch,
            verification.is_ok(),
            wall_duration,
            &dispatch_plan,
            Some(routing_ctx_for_feedback),
            verification.as_ref().err().map(verify_failure_reason),
        )
        .await;

        let verdict = match verification {
            Ok(verdict) => verdict,
            Err(error) => {
                // Release worktree with RetainForFailure for post-mortem.
                if let Some((provider, lease)) =
                    self.workspace_provider.as_ref().zip(lease.as_ref())
                {
                    let _ = provider
                        .release(
                            lease,
                            roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                        )
                        .await;
                }
                return Err(error);
            }
        };

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
            if let Err(e) = provider
                .release(lease, roko_graph::workspace::WorkspaceReleasePolicy::Delete)
                .await
            {
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
        let mut outputs = vec![output];
        verdict.stamp(&mut outputs);
        Ok(outputs)
    }
}

/// Queue a cargo verify step on the per-repository compile lock before its
/// timeout starts, so a build by a plan running beside this one cannot time
/// the step out. Other steps take no permit.
async fn verify_compile_permit(
    workdir: &Path,
    compile_concurrency: usize,
    step: &crate::task_parser::VerifyStep,
    plan_id: &str,
    task_id: &str,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    let runs_cargo = step
        .command
        .split(|c: char| c.is_whitespace() || "&|;({".contains(c))
        .any(|word| word == "cargo");
    if !runs_cargo {
        return None;
    }
    crate::runner::gate_dispatch::acquire_compile_ownership(
        workdir,
        compile_concurrency,
        std::time::Duration::from_millis(step.timeout_ms),
        plan_id,
        task_id,
        &step.command,
    )
    .await
    .inspect_err(|error| {
        tracing::warn!(%error, "running the cargo verify step without the compile lock");
    })
    .ok()
}

/// Stable label for the `index`-th verify step (`verify[i]` or `verify[i:phase]`).
fn verify_step_label(index: usize, phase: &str) -> String {
    if phase.is_empty() {
        format!("verify[{index}]")
    } else {
        format!("verify[{index}:{phase}]")
    }
}

/// Maximum number of output lines kept from a gate's detail for dashboard
/// display.  Lines beyond this are replaced by a "… N earlier lines not shown"
/// leader.
const GATE_OUTPUT_TAIL_LINES: usize = 60;

/// Maximum byte size of the kept tail before a hard byte truncation kicks in.
/// When the 60-line tail still exceeds this, the last 16 KiB is kept (aligned
/// to a UTF-8 char boundary) and prefixed with "… earlier output not shown".
const GATE_OUTPUT_TAIL_BYTES: usize = 16 * 1024;

/// Gate output published to the dashboard: `$ {command}`, then (when
/// non-empty after trimming) the last 60 lines / 16 KiB of the gate's detail,
/// and finally — when the verdict failed — `✗ ` and how it ended.
fn published_gate_output(command: &str, verdict: &roko_core::Verdict) -> String {
    let mut result = format!("$ {command}");

    let trimmed = verdict.detail.as_deref().unwrap_or("").trim_end();
    if !trimmed.is_empty() {
        result.push('\n');
        result.push_str(&gate_output_tail(trimmed));
    }

    if !verdict.passed {
        result.push_str("\n✗ ");
        result.push_str(&gate_how_ended(&verdict.reason));
    }

    result
}

/// Keep the last [`GATE_OUTPUT_TAIL_LINES`] lines of `detail`; prepend a
/// "… N earlier lines not shown" message when lines are dropped.  If the
/// resulting tail still exceeds [`GATE_OUTPUT_TAIL_BYTES`], truncate to the
/// last 16 KiB (aligned to a UTF-8 char boundary) and prepend
/// "… earlier output not shown".
fn gate_output_tail(detail: &str) -> String {
    let all_lines: Vec<&str> = detail.lines().collect();
    let total = all_lines.len();

    let (tail_lines, dropped) = if total > GATE_OUTPUT_TAIL_LINES {
        let dropped = total - GATE_OUTPUT_TAIL_LINES;
        (&all_lines[dropped..], dropped)
    } else {
        (&all_lines[..], 0)
    };

    let mut tail = tail_lines.join("\n");

    if tail.len() > GATE_OUTPUT_TAIL_BYTES {
        // Move the cut forward to a UTF-8 char boundary so we keep at most
        // GATE_OUTPUT_TAIL_BYTES bytes.
        let ideal = tail.len() - GATE_OUTPUT_TAIL_BYTES;
        let mut byte_pos = ideal;
        while !tail.is_char_boundary(byte_pos) {
            byte_pos += 1;
        }
        let kept = tail[byte_pos..].to_string();
        tail = format!("… earlier output not shown\n{kept}");
    } else if dropped > 0 {
        tail = format!("… {dropped} earlier lines not shown\n{tail}");
    }

    tail
}

/// Translate the verdict's `reason` field into a human-readable closing line.
///
/// - `"exit code: <n>"` → `"exit status <n>"`
/// - `"exit code: terminated by signal"` → `"terminated by a signal"`
/// - `""` (empty) → `"failed"`
/// - anything else → unchanged
fn gate_how_ended(reason: &str) -> String {
    if reason.is_empty() {
        "failed".to_string()
    } else if let Some(code) = reason.strip_prefix("exit code: ") {
        if code == "terminated by signal" {
            "terminated by a signal".to_string()
        } else {
            format!("exit status {code}")
        }
    } else {
        reason.to_string()
    }
}

/// Retry-facing summary of a failed verify run, including skipped steps.
fn verify_failure_summary(
    title: &str,
    total: usize,
    failures: &[String],
    skipped: &[String],
) -> String {
    let summary = format!(
        "{n}/{total} verify step(s) failed for task `{title}`:\n\n{details}",
        n = failures.len(),
        details = failures.join("\n\n---\n\n"),
    );
    if skipped.is_empty() {
        summary
    } else {
        format!(
            "{summary}\n\nSkipped after the first failure: {}",
            skipped.join(", ")
        )
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
        let _in_flight = self.in_flight.register(
            &format!("{}/{}", spec.plan_id, task.id),
            &lease.path,
            &task.files,
        );

        // ── P3-AGT-2: Express mode check (streaming) ─────────────────────
        let express_active = is_express_task(&self.config, &task);
        let max_turns = task_turn_limit(&self.config, &task, express_active);
        if express_active {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                tier = %task.tier,
                fast_model = %self.config.routing.fast_task_model,
                max_turns,
                "P3-AGT-2: express mode active (streaming) — routing to fast model with reduced turn limit"
            );
        }

        // ── W10: Enrichment pipeline (streaming) ─────────────────────────
        let routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);
        // Clone before the move into DispatchContext so emit_feedback can pass
        // the real dispatch-time context to the routing observation sink.
        let routing_ctx_for_feedback = routing_ctx.clone();

        let (cached_workspace_map, cached_workspace_context, cached_cfactor_context) =
            self.static_prompt_cache.get_or_init(|| {
                let ws_map =
                    crate::dispatch::prompt_builder::generate_workspace_map_pub(&self.workdir);
                let ws_ctx =
                    crate::dispatch::prompt_builder::generate_workspace_context_pub(&self.workdir);
                let cf_ctx =
                    crate::dispatch::prompt_builder::generate_cfactor_context_pub(&self.workdir);
                tracing::debug!(
                    ws_map_bytes = ws_map.len(),
                    ws_ctx_bytes = ws_ctx.len(),
                    cf_ctx_bytes = cf_ctx.len(),
                    "static_prompt_cache: computed once for this run (streaming path)"
                );
                (ws_map, ws_ctx, cf_ctx)
            });
        let express_force_backend_streaming = if express_active && self.cli_model_override.is_none()
        {
            Some(self.config.routing.fast_task_model.clone())
        } else {
            None
        };
        let retry_key = format!("{}/{}", spec.plan_id, task.id);
        let efficiency_attempt_id = self.next_attempt_id(&retry_key);
        let prompt_experiment = self
            .feedback
            .experiment_store_path
            .as_deref()
            .and_then(|store| {
                prompt_experiment::context(store, &spec.plan_id, &task.id, &efficiency_attempt_id)
            });
        let mut dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: lease.path.clone(),
            model_hint: task.model_hint.clone(),
            force_backend: self
                .cli_model_override
                .clone()
                .or(express_force_backend_streaming),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: 0,
            prompt_experiment: prompt_experiment.clone(),
            gate_feedback: None,
            routing_context: Some(routing_ctx),
            routing_bias: None,
            dependency_outputs: upstream_outputs(&input),
            error_patterns_context: self.factory.format_error_patterns_for_prompt(5),
            cached_workspace_map: cached_workspace_map.clone(),
            cached_workspace_context: cached_workspace_context.clone(),
            cached_cfactor_context: cached_cfactor_context.clone(),
        };
        let dispatch_plan = self.plan_dispatch(spec, &task, &mut dispatch_ctx)?;
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
            max_turns: Some(max_turns),
            live_output: None,
        };
        let _launched_treatments = prompt_experiment::LaunchedTreatments::bind(
            prompt_experiment,
            &dispatch_plan.prompt.diagnostics.experiment_assignments,
            &request.system_prompt,
            &request.prompt,
        )
        .await;

        // ── Live output forwarder (streaming path) ────────────────────────
        let request = {
            let mut req = request;
            if let (Some(tui), Some(live_setting)) = (&self.tui_bridge, &self.live_agent_output) {
                let (live_tx, mut live_rx) =
                    tokio::sync::mpsc::channel::<roko_agent::live_output::LiveAgentEvent>(64);
                let trusted = matches!(live_setting, LiveAgentOutput::Trusted);
                req.live_output = Some(roko_agent::live_output::LiveOutput {
                    sink: live_tx,
                    trusted,
                });
                let tui_clone = tui.clone();
                let agent_id_s = format!(
                    "{}/{}",
                    spec.plan_id,
                    ctx.cell_id.as_deref().unwrap_or(&task.id)
                );
                let plan_id_s = spec.plan_id.clone();
                let task_id_s = task.id.clone();
                tokio::spawn(async move {
                    while let Some(event) = live_rx.recv().await {
                        forward_live_event_to_tui(
                            &tui_clone,
                            &agent_id_s,
                            &plan_id_s,
                            &task_id_s,
                            event,
                        );
                    }
                });
            }
            req
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
        let (outcome, output_signals, verification) = match dispatch_result {
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

                // Forward final usage event with cost.
                let _ = event_tx
                    .send(GraphTaskEvent::Usage {
                        input_tokens: u64::from(dispatch.result.usage.input_tokens),
                        output_tokens: u64::from(dispatch.result.usage.output_tokens),
                        cost_usd: actual_cost,
                    })
                    .await;

                // ── Verify steps (streaming) ─────────────────────────────
                //
                // Same verdict logic as the batch path; gates run in the lease
                // path and progress streams through the event channel.
                let verification = if dispatch.result.success {
                    let attempt_number = self
                        .gate_retry_context
                        .lock()
                        .get(&retry_key)
                        .map_or(0, |(_, attempt)| *attempt);
                    Some(
                        self.settle_task_verification(
                            spec,
                            &task,
                            &dispatch,
                            &lease.path,
                            &retry_key,
                            attempt_number,
                            &efficiency_attempt_id,
                            Some(&event_tx),
                        )
                        .await,
                    )
                } else {
                    None
                };
                let verified = matches!(verification, Some(Ok(_)));

                // ── Learning/feedback pipeline (streaming) ───────────────
                //
                // Settled after the gate so learning sees the verified outcome.
                let failure_reason = match &verification {
                    Some(Ok(_)) => None,
                    Some(Err(error)) => Some(verify_failure_reason(error)),
                    None => Some(provider_failure_reason(
                        dispatch.result.output.body.as_text().unwrap_or_default(),
                    )),
                };
                self.emit_feedback(
                    spec,
                    &task,
                    &efficiency_attempt_id,
                    &dispatch,
                    verified,
                    wall_duration,
                    &dispatch_plan,
                    Some(routing_ctx_for_feedback),
                    failure_reason,
                )
                .await;

                let outcome_kind = if verified {
                    TaskDispatchOutcomeKind::Succeeded
                } else {
                    TaskDispatchOutcomeKind::Failed
                };

                let output_signals = match &verification {
                    Some(Ok(verdict)) => {
                        let mut output = dispatch.result.output;
                        if output.body.as_text().is_err() {
                            output = Signal::builder(Kind::AgentOutput)
                                .body(Body::text(format!(
                                    "provider `{}` completed task `{}`",
                                    dispatch.target.provider_id, spec.title
                                )))
                                .build();
                        }
                        let mut outputs = vec![output];
                        verdict.stamp(&mut outputs);
                        outputs
                    }
                    _ => Vec::new(),
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

                (dispatch_outcome, output_signals, verification)
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

        // A verify failure is reported as such; otherwise an unsuccessful
        // provider result fails the task even though cost was settled
        // (callers still incur the charge).
        if let Some(Err(error)) = verification {
            return Err(error);
        }
        if outcome.outcome == TaskDispatchOutcomeKind::Failed {
            return Err(RokoError::Agent {
                backend: outcome.provider_id.clone(),
                message: "provider returned an unsuccessful result".to_string(),
            });
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

// ─── Provider failover ──────────────────────────────────────────────────────

/// `RokoError::Gateway` category for a task left without a usable provider.
///
/// The error is non-retryable, so `TaskExecutorCell` fails the attempt at once
/// instead of re-running a dispatch that would be refused again.
const PROVIDER_EXHAUSTED_CATEGORY: &str = "provider_exhausted";

/// A model to dispatch: the key sent to the bridge and the config resolving it.
#[derive(Clone)]
struct DispatchCandidate {
    model_key: String,
    /// A clone of the run config with a `[models.*]` entry that serves a
    /// hinted slug on another configured provider; `None` uses the run config.
    config: Option<Arc<RokoConfig>>,
}

/// Why the provider behind a model cannot take this dispatch.
#[derive(Debug, Clone)]
struct ProviderRefusal {
    model_key: String,
    /// Wire slug of the refused model, used to find the same model elsewhere.
    model_slug: String,
    provider_id: String,
    provider_kind: roko_core::agent::ProviderKind,
    /// The provider's own words, or why it cannot be called.
    reason: String,
    /// When the provider is expected to accept work again (unix ms).
    until_ms: Option<i64>,
    /// Calling the provider again cannot help (missing, not dispatchable, no
    /// credentials, out of usage, billing), unlike an open circuit that may
    /// already have recovered.
    definitive: bool,
}

/// Provider kinds that serve the same model family over another transport.
fn same_family_kinds(
    kind: roko_core::agent::ProviderKind,
) -> &'static [roko_core::agent::ProviderKind] {
    use roko_core::agent::ProviderKind;
    match kind {
        ProviderKind::ClaudeCli | ProviderKind::AnthropicApi => {
            &[ProviderKind::ClaudeCli, ProviderKind::AnthropicApi]
        }
        ProviderKind::GeminiCli | ProviderKind::GeminiApi => {
            &[ProviderKind::GeminiCli, ProviderKind::GeminiApi]
        }
        _ => &[],
    }
}

/// CLI and ACP harnesses bring their own tools, so a profile's
/// `supports_tools` only matters for providers driven by roko's tool loop.
fn provider_brings_own_tools(provider: &roko_core::config::schema::ProviderConfig) -> bool {
    matches!(
        provider.transport(),
        roko_core::config::ProviderTransport::Cli { .. }
            | roko_core::config::ProviderTransport::Acp { .. }
    )
}

fn missing_credentials_reason(
    provider: &roko_core::config::schema::ProviderConfig,
    provider_id: &str,
) -> String {
    provider.api_key_env.as_deref().map_or_else(
        || format!("provider `{provider_id}` is not installed or has no credentials"),
        |env| format!("{env} is not set"),
    )
}

fn format_local_ms(ms: i64) -> String {
    use chrono::TimeZone as _;
    chrono::Local.timestamp_millis_opt(ms).single().map_or_else(
        || ms.to_string(),
        |at| at.format("%Y-%m-%d %H:%M %:z").to_string(),
    )
}

impl GraphTaskDispatcher {
    /// Run the provider bridge, treating the planned model as a preference.
    ///
    /// A model whose provider is missing, not dispatchable, without
    /// credentials, disabled, or has an open circuit is skipped before any
    /// call. One that refuses with a usage-window error ("You've hit your
    /// session limit · resets 4pm") is quarantined in the persisted health
    /// registry until its reported reset (else
    /// `routing.exhaustion_cooldown_secs`). Either way the next candidate from
    /// [`Self::failover_candidates`] runs in the same attempt, so no task
    /// retry is burned. An explicit `--model` override is a pin and never
    /// fails over. When nothing usable remains the attempt fails with a
    /// non-retryable error that says how to recover.
    async fn run_bridge_with_failover(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        mut request: AgentDispatchRequest,
    ) -> Result<crate::dispatch_v2::AgentResultDispatch> {
        let pinned = self.cli_model_override.is_some();
        let mut candidate = DispatchCandidate {
            model_key: request.model_key.clone(),
            config: None,
        };
        let mut refusals: Vec<ProviderRefusal> = Vec::new();
        loop {
            if !pinned && let Some(refusal) = self.blocked_provider(&candidate) {
                let definitive = refusal.definitive;
                refusals.push(refusal);
                match self.failover_model(spec, task_id, &refusals) {
                    Ok(next) => {
                        candidate = next;
                        continue;
                    }
                    Err(error) if definitive => return Err(error),
                    // An open circuit may already have recovered; with no
                    // usable alternative, dispatch the planned model.
                    Err(_) => {
                        refusals.pop();
                    }
                }
            }

            request.model_key = candidate.model_key.clone();
            let dispatch = match &candidate.config {
                Some(config) => {
                    self.factory
                        .run_shared_agent_bridge_with_config(request.clone(), Arc::clone(config))
                        .await
                }
                None => self.factory.run_shared_agent_bridge(request.clone()).await,
            }
            .map_err(|error| RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: error.to_string(),
            })?;
            if dispatch.result.success {
                return Ok(dispatch);
            }
            let Some(exhaustion) = dispatch
                .result
                .output
                .body
                .as_text()
                .ok()
                .and_then(roko_agent::provider::error_classify::detect_provider_exhaustion)
            else {
                return Ok(dispatch);
            };

            let cooldown_ms = i64::try_from(
                self.config
                    .routing
                    .exhaustion_cooldown_secs
                    .saturating_mul(1_000),
            )
            .unwrap_or(i64::MAX);
            let until_ms = exhaustion.resets_at_ms.unwrap_or_else(|| {
                chrono::Utc::now()
                    .timestamp_millis()
                    .saturating_add(cooldown_ms)
            });
            let provider_id = dispatch.target.provider_id.clone();
            self.factory
                .health_registry
                .record_exhaustion(&provider_id, until_ms);
            // The caller settles only the result it receives; account the
            // refused call here.
            self.budget_ledger.settle(
                &spec.plan_id,
                0,
                f64::from(dispatch.result.usage.cost_usd),
            )?;
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id,
                provider = %provider_id,
                model = %candidate.model_key,
                until = %format_local_ms(until_ms),
                reason = %exhaustion.message,
                "provider is out of usage; skipping it until its reset"
            );
            refusals.push(ProviderRefusal {
                model_key: candidate.model_key.clone(),
                model_slug: dispatch.target.model_slug.clone(),
                provider_id,
                provider_kind: dispatch.target.provider_kind,
                reason: exhaustion.message,
                until_ms: Some(until_ms),
                definitive: true,
            });
            if pinned {
                return Err(self.no_usable_provider(&refusals, &[], true));
            }
            candidate = self.failover_model(spec, task_id, &refusals)?;
        }
    }

    fn resolve_candidate(
        &self,
        candidate: &DispatchCandidate,
    ) -> crate::dispatch_v2::ProviderDispatchSpec {
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        crate::dispatch_v2::ProviderDispatchResolver::new(Arc::clone(config))
            .resolve(&candidate.model_key)
    }

    /// The refusal for `candidate` when its provider must not be called now:
    /// missing, not dispatchable, without credentials, statically disabled,
    /// or its circuit is open in the health registry.
    fn blocked_provider(&self, candidate: &DispatchCandidate) -> Option<ProviderRefusal> {
        use crate::dispatch_v2::ProviderRuntime;
        use roko_learn::provider_health::ErrorClass;

        let target = self.resolve_candidate(candidate);
        let provider_id = target.provider_id.clone();
        let refusal = |reason: String, until_ms: Option<i64>, definitive: bool| ProviderRefusal {
            model_key: candidate.model_key.clone(),
            model_slug: target.model_slug.clone(),
            provider_id: provider_id.clone(),
            provider_kind: target.provider_kind,
            reason,
            until_ms,
            definitive,
        };
        let Some(provider) = target.provider_config.as_ref() else {
            return Some(refusal(
                format!("provider `{provider_id}` is not configured"),
                None,
                true,
            ));
        };
        if let ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Some(refusal(
                format!(
                    "provider `{provider_id}` is not dispatchable: {}",
                    unsupported.detail
                ),
                None,
                true,
            ));
        }
        if self
            .config
            .routing
            .disabled_providers
            .contains(&provider_id)
        {
            return Some(refusal(
                "listed in routing.disabled_providers".to_string(),
                None,
                false,
            ));
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Some(refusal(
                missing_credentials_reason(provider, &provider_id),
                None,
                true,
            ));
        }
        let registry = &self.factory.health_registry;
        if registry.is_available(&provider_id) {
            return None;
        }
        let health = registry.get(&provider_id);
        let (reason, definitive) = match health.failure_window.back().map(|r| r.error_class) {
            Some(ErrorClass::Exhausted) => ("out of usage", true),
            Some(ErrorClass::Billing) => ("billing failure", true),
            _ => ("circuit open after repeated failures", false),
        };
        Some(refusal(
            reason.to_string(),
            health.cooldown_until,
            definitive,
        ))
    }

    /// Candidates after `refusals`, in order: the first refused model's slug
    /// on another configured provider of its family (claude_cli can run any
    /// Claude slug), `[routing] fallback_models`, `agent.fallback_model`, and
    /// `agent.default_model`.
    fn failover_candidates(&self, refusals: &[ProviderRefusal]) -> Vec<DispatchCandidate> {
        let mut candidates: Vec<DispatchCandidate> = Vec::new();
        let push_run_model = |candidates: &mut Vec<DispatchCandidate>, model_key: &str| {
            if !model_key.trim().is_empty()
                && !candidates
                    .iter()
                    .any(|candidate| candidate.model_key == model_key)
            {
                candidates.push(DispatchCandidate {
                    model_key: model_key.to_string(),
                    config: None,
                });
            }
        };
        if let Some(first) = refusals.first() {
            for (key, profile) in self.config.effective_models() {
                if profile.slug == first.model_slug && key != first.model_key {
                    push_run_model(&mut candidates, &key);
                }
            }
            let family = same_family_kinds(first.provider_kind);
            let base_profile = self
                .resolve_candidate(&DispatchCandidate {
                    model_key: first.model_key.clone(),
                    config: None,
                })
                .model_profile
                .unwrap_or_else(|| roko_core::config::schema::ModelProfile {
                    slug: first.model_slug.clone(),
                    supports_tools: true,
                    ..Default::default()
                });
            for (provider_id, provider) in self.config.effective_providers() {
                if !family.contains(&provider.kind) || provider_id == first.provider_id {
                    continue;
                }
                let model_key = format!("{}@{provider_id}", first.model_slug);
                let mut config = (*self.config).clone();
                config.models.insert(
                    model_key.clone(),
                    roko_core::config::schema::ModelProfile {
                        provider: provider_id,
                        ..base_profile.clone()
                    },
                );
                candidates.push(DispatchCandidate {
                    model_key,
                    config: Some(Arc::new(config)),
                });
            }
        }
        for model_key in self
            .config
            .routing
            .fallback_models
            .iter()
            .chain(self.config.agent.fallback_model.iter())
            .chain(std::iter::once(&self.config.agent.default_model))
        {
            push_run_model(&mut candidates, model_key);
        }
        candidates
    }

    /// The first usable model in [`Self::failover_candidates`], with the
    /// substitution and its reason logged at WARN.
    fn failover_model(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        refusals: &[ProviderRefusal],
    ) -> Result<DispatchCandidate> {
        let mut skipped = Vec::new();
        for candidate in self.failover_candidates(refusals) {
            match self.failover_candidate(&candidate, refusals) {
                Ok(provider_id) => {
                    if let Some(refused) = refusals.last() {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id,
                            from_model = %refused.model_key,
                            from_provider = %refused.provider_id,
                            to_model = %candidate.model_key,
                            to_provider = %provider_id,
                            reason = %refused.reason,
                            "model substitution: `{}` on `{}` is unusable ({}); running `{}` on `{provider_id}` instead",
                            refused.model_key,
                            refused.provider_id,
                            refused.reason,
                            candidate.model_key,
                        );
                    }
                    return Ok(candidate);
                }
                Err(why) => skipped.push(format!("{}: {why}", candidate.model_key)),
            }
        }
        Err(self.no_usable_provider(refusals, &skipped, false))
    }

    /// The provider id of `candidate` when it can take the task now, else why not.
    fn failover_candidate(
        &self,
        candidate: &DispatchCandidate,
        refusals: &[ProviderRefusal],
    ) -> std::result::Result<String, String> {
        let target = self.resolve_candidate(candidate);
        let provider_id = &target.provider_id;
        let (Some(profile), Some(provider)) = (&target.model_profile, &target.provider_config)
        else {
            return Err(format!("provider `{provider_id}` is not configured"));
        };
        if refusals
            .iter()
            .any(|refusal| &refusal.provider_id == provider_id)
        {
            return Err(format!("provider `{provider_id}` is unavailable too"));
        }
        if let crate::dispatch_v2::ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Err(format!(
                "provider `{provider_id}` is not dispatchable: {}",
                unsupported.detail
            ));
        }
        if self.config.routing.disabled_providers.contains(provider_id) {
            return Err(format!(
                "provider `{provider_id}` is in routing.disabled_providers"
            ));
        }
        if !profile.supports_tools && !provider_brings_own_tools(provider) {
            return Err("model does not support tools".to_string());
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Err(missing_credentials_reason(provider, provider_id));
        }
        let registry = &self.factory.health_registry;
        if !registry.is_available(provider_id) {
            let until = registry
                .get(provider_id)
                .cooldown_until
                .map(|ms| format!(" until {}", format_local_ms(ms)))
                .unwrap_or_default();
            return Err(format!("provider `{provider_id}` is quarantined{until}"));
        }
        Ok(provider_id.clone())
    }

    /// Non-retryable error for a task left without a usable provider, naming
    /// each refusal, each skipped fallback, and how to recover.
    fn no_usable_provider(
        &self,
        refusals: &[ProviderRefusal],
        skipped: &[String],
        pinned: bool,
    ) -> RokoError {
        let refused = refusals
            .iter()
            .map(|refusal| {
                let until = refusal
                    .until_ms
                    .map(|ms| format!(" (skipped until {})", format_local_ms(ms)))
                    .unwrap_or_default();
                format!(
                    "`{}` on `{}`: {}{until}",
                    refusal.model_key, refusal.provider_id, refusal.reason
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let fallback = if pinned {
            "The --model override pins this model, so no failover was attempted; drop --model to \
             allow it."
                .to_string()
        } else if skipped.is_empty() {
            format!(
                "No fallback model is configured: set [routing] fallback_models in roko.toml{} \
                 and put that provider's API key in ~/.roko/.env (loaded automatically at startup).",
                self.api_key_model_examples()
            )
        } else {
            format!(
                "No fallback model is usable ({}): put the missing API key in ~/.roko/.env \
                 (loaded automatically at startup) or extend [routing] fallback_models.",
                skipped.join("; ")
            )
        };
        let wait = refusals
            .iter()
            .filter_map(|refusal| refusal.until_ms)
            .min()
            .map_or_else(
                || "wait for the provider to recover".to_string(),
                |ms| format!("wait until {}", format_local_ms(ms)),
            );
        RokoError::Gateway {
            category: PROVIDER_EXHAUSTED_CATEGORY,
            retryable: false,
            message: format!(
                "no usable provider for this task: {refused}. {fallback} Otherwise {wait} and re-run."
            ),
        }
    }

    /// " (e.g. kimi-k2-5 needs MOONSHOT_API_KEY, …)" for configured API-key models.
    fn api_key_model_examples(&self) -> String {
        let providers = self.config.effective_providers();
        let mut examples: Vec<String> = self
            .config
            .effective_models()
            .into_iter()
            .filter(|(_, profile)| profile.supports_tools)
            .filter_map(|(key, profile)| {
                let env = providers.get(&profile.provider)?.api_key_env.clone()?;
                Some(format!("{key} needs {env}"))
            })
            .collect();
        examples.sort();
        examples.truncate(3);
        if examples.is_empty() {
            String::new()
        } else {
            format!(" (e.g. {})", examples.join(", "))
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

    // ─── Verify verdict tests ───────────────────────────────────────────────

    const VERIFY_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"verify-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-v1","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    fn verify_step(phase: &str, command: &str) -> crate::task_parser::VerifyStep {
        crate::task_parser::VerifyStep {
            phase: phase.to_string(),
            command: command.to_string(),
            fail_msg: None,
            timeout_ms: 10_000,
        }
    }

    fn no_auto_fix(config: &mut RokoConfig) {
        config.gates.cargo_fix_enabled = false;
    }

    /// Sorted `(attempt_id, outcome)` of every efficiency record, once
    /// `expected` records have landed from the background writers.
    async fn efficiency_records(path: &Path, expected: usize) -> Vec<(String, String)> {
        // Generous deadline: the writers are background tasks, and a loaded
        // test run can starve them for seconds. Returns as soon as they land.
        for _ in 0..600 {
            let mut records: Vec<(String, String)> = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .map(|record| {
                    let field = |name: &str| record[name].as_str().unwrap_or_default().to_string();
                    (field("attempt_id"), field("outcome"))
                })
                .collect();
            if records.len() >= expected {
                records.sort();
                return records;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("efficiency records were not written to {}", path.display());
    }

    #[tokio::test]
    async fn failing_authored_verify_is_never_force_accepted() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                // The retired review-cycle cap force-accepted at this count.
                config.gates.max_review_cycles = 1;
                no_auto_fix(config);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "exit 1")];
        let spec = make_spec(&task);

        for attempt in 0..3 {
            let error = dispatcher
                .dispatch(&spec, Vec::new(), &CellContext::new())
                .await
                .expect_err("a failing authored verify step must fail the attempt");
            assert!(
                matches!(error, RokoError::Verify { .. }),
                "attempt {attempt}: {error}"
            );
        }
    }

    #[tokio::test]
    async fn verify_fails_fast_and_reports_skipped_steps() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("compile-ran");
        task.verify = vec![
            verify_step("structural", "grep -q missing-symbol /dev/null"),
            verify_step("compile", &format!("touch {}", marker.display())),
        ];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("structural failure");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("1/2 verify step(s) failed"), "{message}");
        assert!(message.contains("grep -q missing-symbol"), "{message}");
        assert!(
            message.contains("Skipped after the first failure: verify[1:compile]"),
            "{message}"
        );
        assert!(!marker.exists(), "fail-fast must not run later steps");
    }

    /// Dispatches `task` beside a fake sibling `T12` of the same plan that
    /// edits `web/src/PlanView.tsx` in the same working tree and finishes its
    /// attempt once `failed_once` exists, first creating `sibling_done`.
    async fn dispatch_beside_editing_sibling(
        dispatcher: &GraphTaskDispatcher,
        task: &TaskDef,
        failed_once: &Path,
        sibling_done: &Path,
    ) -> Result<Vec<Signal>> {
        let spec = make_spec(task);
        let sibling = dispatcher.in_flight.register(
            &format!("{}/T12", spec.plan_id),
            &dispatcher.workdir,
            &["web/src/PlanView.tsx".to_string()],
        );
        let finish_sibling = async {
            while !failed_once.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            std::fs::write(sibling_done, "").expect("sibling edit");
            drop(sibling);
        };
        let ctx = CellContext::new();
        let (outcome, ()) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            tokio::join!(dispatcher.dispatch(&spec, Vec::new(), &ctx), finish_sibling)
        })
        .await
        .expect("the failed step settles once the sibling finishes");
        outcome
    }

    fn settle_quickly(config: &mut RokoConfig) {
        config.gates.cargo_fix_enabled = false;
        config.gates.sibling_settle_secs = 30;
    }

    #[tokio::test]
    async fn a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            settle_quickly,
            GraphFeedbackContext::default(),
        )
        .await;
        let failed_once = temp.path().join("failed-once");
        let sibling_done = temp.path().join("sibling-done");
        task.verify = vec![verify_step(
            "typecheck",
            &format!(
                "test -f {} || {{ touch {}; exit 2; }}",
                sibling_done.display(),
                failed_once.display()
            ),
        )];

        let outputs =
            dispatch_beside_editing_sibling(&dispatcher, &task, &failed_once, &sibling_done)
                .await
                .expect("the re-run after the sibling settled passes");

        assert!(
            failed_once.exists(),
            "the first run failed beside the sibling"
        );
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
    }

    #[tokio::test]
    async fn a_verify_failure_left_in_a_sibling_file_blames_the_sibling() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            settle_quickly,
            GraphFeedbackContext::default(),
        )
        .await;
        let failed_once = temp.path().join("failed-once");
        let sibling_done = temp.path().join("sibling-done");
        task.verify = vec![verify_step(
            "typecheck",
            &format!(
                "touch {}; echo 'src/PlanView.tsx(448,24): error TS2304: \
                 Cannot find name formatDuration.' >&2; exit 2",
                failed_once.display()
            ),
        )];

        let error =
            dispatch_beside_editing_sibling(&dispatcher, &task, &failed_once, &sibling_done)
                .await
                .expect_err("the failure persists after the sibling settled");

        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(
            message.starts_with("blocked_by_sibling = T12: 1/1 verify step(s) failed"),
            "{message}"
        );
        assert!(
            message.contains("first at src/PlanView.tsx:448:24"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn verified_outcome_drives_output_verdict_and_feedback() {
        let temp = tempdir().expect("tempdir");
        let efficiency = temp.path().join("efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;

        let unverified = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("dispatch without verify steps");
        assert_eq!(
            TaskGateVerdict::from_signals(&unverified),
            Some(TaskGateVerdict::Unverified)
        );

        task.verify = vec![verify_step("structural", "true")];
        let passed = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("verified dispatch");
        assert_eq!(
            TaskGateVerdict::from_signals(&passed),
            Some(TaskGateVerdict::Passed)
        );

        task.verify = vec![verify_step("structural", "false")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("verify failure");

        // The provider succeeded all three times; learning must record the
        // verified outcome, so the failed-verify attempt is a failure. Each
        // attempt gets its own identity, which its gate-pass record extends.
        let task_key = format!("{}/{}", make_spec(&task).plan_id, task.id);
        let attempt =
            |suffix: &str, outcome: &str| (format!("{task_key}/{suffix}"), outcome.to_string());
        assert_eq!(
            efficiency_records(&efficiency, 4).await,
            vec![
                attempt("a0", "success"),
                attempt("a1", "success"),
                attempt("a1/gate-pass", "gate_pass"),
                attempt("a2", "failure"),
            ]
        );
    }

    #[tokio::test]
    async fn streaming_verify_failure_is_a_failed_terminal_attempt() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![
            verify_step("structural", "false"),
            verify_step("compile", "true"),
        ];
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());

        let error = dispatcher
            .dispatch_streaming(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_cell_id("T-STREAM".to_string()),
                &lease,
                event_tx,
                &NoopAttemptRecorder,
            )
            .await
            .expect_err("streaming verify failure");
        assert!(matches!(error, RokoError::Verify { .. }), "{error}");

        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        let terminal: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                GraphTaskEvent::AttemptTerminal { outcome, .. } => Some(*outcome),
                _ => None,
            })
            .collect();
        assert_eq!(terminal, vec![TaskDispatchOutcomeKind::Failed]);
    }

    #[test]
    fn published_gate_output_leads_with_the_command() {
        assert_eq!(
            published_gate_output(
                "cargo check -p roko-cli",
                &roko_core::Verdict::fail("verify[0]", "exit code: 101")
                    .with_detail("error[E0425]\n"),
            ),
            "$ cargo check -p roko-cli\nerror[E0425]\n✗ exit status 101"
        );
        assert_eq!(
            published_gate_output("true", &roko_core::Verdict::pass("verify[0]")),
            "$ true"
        );
        assert_eq!(verify_step_label(2, ""), "verify[2]");
        assert_eq!(verify_step_label(0, "compile"), "verify[0:compile]");
    }

    // ─── Provider failover on usage exhaustion ──────────────────────────────

    fn write_executable(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write script");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("make script executable");
    }

    /// A fake `claude` that refuses exactly as the CLI did on 2026-09-26 and
    /// appends one line to `calls` per invocation.
    fn session_limit_claude(path: &Path, calls: &Path) {
        write_executable(
            path,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":true,"total_cost_usd":0,"result":"You’ve hit your session limit · resets 4pm (Europe/Berlin)"}}'
exit 1
"#,
                calls.display()
            ),
        );
    }

    fn invocations(calls: &Path) -> usize {
        std::fs::read_to_string(calls).map_or(0, |log| log.lines().count())
    }

    /// Serve canned OpenAI-compatible chat responses, one per connection,
    /// capturing each request body.
    fn spawn_openai_mock(
        responses: Vec<serde_json::Value>,
    ) -> (String, Arc<parking_lot::Mutex<Vec<serde_json::Value>>>) {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}/v1", listener.local_addr().expect("mock addr"));
        let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let requests = Arc::clone(&captured);
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
                let mut buf = Vec::new();
                let mut chunk = [0_u8; 8192];
                let body_start = loop {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&buf[..body_start]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                while buf.len() < body_start + length {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let end = buf.len().min(body_start + length);
                requests.lock().push(
                    serde_json::from_slice(&buf[body_start..end])
                        .unwrap_or(serde_json::Value::Null),
                );
                let body = response.to_string();
                let wire = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(wire.as_bytes());
            }
        });
        (base_url, captured)
    }

    fn tool_call_turn(id: &str, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": format!("chatcmpl-{id}"),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": id,
                        "type": "function",
                        "function": { "name": name, "arguments": arguments.to_string() }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        })
    }

    fn final_turn(text: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-final",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": text },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15 }
        })
    }

    /// `claude_cli` (fake script) plus two OpenAI-compatible fallbacks: one
    /// whose key env var is never set, and one pointed at `api_base_url`.
    fn failover_config(claude: &Path, api_base_url: &str, fallback_models: &[&str]) -> RokoConfig {
        let provider =
            |kind, command: Option<String>, base_url: Option<&str>, key_env: Option<&str>| {
                ProviderConfig {
                    kind,
                    base_url: base_url.map(str::to_string),
                    api_key_env: key_env.map(str::to_string),
                    command,
                    args: None,
                    timeout_ms: Some(15_000),
                    ttft_timeout_ms: Some(15_000),
                    connect_timeout_ms: Some(5_000),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                }
            };
        let api_model = |provider: &str, slug: &str| ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            context_window: 128_000,
            max_output: Some(1_024),
            // Unknown slugs default to a 3-tool cap, which would drop `bash`.
            max_tools: Some(32),
            supports_tools: true,
            tool_format: "openai_json".to_string(),
            ..ModelProfile::default()
        };
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            provider(
                ProviderKind::ClaudeCli,
                Some(claude.display().to_string()),
                None,
                None,
            ),
        );
        // `PATH` is always set, standing in for a key loaded from ~/.roko/.env.
        config.providers.insert(
            "mock_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some(api_base_url),
                Some("PATH"),
            ),
        );
        config.providers.insert(
            "keyless_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some("http://127.0.0.1:9/v1"),
                Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"),
            ),
        );
        config.models.insert(
            "claude-sonnet".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        config.models.insert(
            "keyless-model".to_string(),
            api_model("keyless_api", "keyless-1"),
        );
        config.models.insert(
            "api-model".to_string(),
            api_model("mock_api", "api-model-1"),
        );
        config.routing.fallback_models = fallback_models.iter().map(|m| m.to_string()).collect();
        config
    }

    /// A live cell for an implementer task hinted at `model_hint`.
    fn failover_cell(
        dispatcher: Arc<GraphTaskDispatcher>,
        model_hint: &str,
    ) -> roko_graph::cells::TaskExecutorCell {
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            description: Some("Edit notes and write hello.txt".to_string()),
            model_hint: Some(model_hint.to_string()),
            timeout_secs: 30,
            max_retries: 2,
            ..make_task_def("focused")
        };
        let config = toml::Value::Table(toml::map::Map::from_iter([
            (
                "plan_id".to_string(),
                toml::Value::String("p-failover".to_string()),
            ),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            ("timeout_secs".to_string(), toml::Value::Integer(30)),
            ("max_retries".to_string(), toml::Value::Integer(2)),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        roko_graph::cells::TaskExecutorCell::live(config, dispatcher)
    }

    #[tokio::test]
    async fn exhausted_claude_cli_fails_over_to_api_model_with_file_and_shell_tools() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        std::fs::write(workdir.join("notes.txt"), "draft notes\n").expect("seed notes");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "call-read",
                "read_file",
                serde_json::json!({ "path": "notes.txt" }),
            ),
            tool_call_turn(
                "call-edit",
                "edit_file",
                serde_json::json!({
                    "path": "notes.txt", "old_string": "draft", "new_string": "final"
                }),
            ),
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi from fallback" }),
            ),
            tool_call_turn(
                "call-shell",
                "bash",
                serde_json::json!({ "command": "cat hello.txt" }),
            ),
            final_turn("fallback finished"),
            final_turn("second task on fallback"),
        ]);
        let config = Arc::new(failover_config(
            &claude,
            &base_url,
            &["keyless-model", "api-model"],
        ));
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(Arc::clone(&health)),
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            workdir.clone(),
        ));
        let cell = failover_cell(dispatcher, "claude-sonnet-4-6");

        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("failover completes the task within one attempt");
        assert_eq!(output[0].body.as_text().expect("text"), "fallback finished");
        assert_eq!(invocations(&calls), 1, "the refusal is not retried");
        // Tool results the fallback model saw, in order: read, edit, write, shell.
        let tool_results: Vec<String> = requests.lock()[4]["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .filter(|message| message["role"] == "tool")
            .map(|message| message["content"].to_string())
            .collect();
        assert_eq!(tool_results.len(), 4, "{tool_results:#?}");
        assert_eq!(
            std::fs::read_to_string(workdir.join("notes.txt")).expect("notes"),
            "final notes\n",
            "{tool_results:#?}"
        );
        assert_eq!(
            std::fs::read_to_string(workdir.join("hello.txt")).expect("hello"),
            "hi from fallback",
            "{tool_results:#?}"
        );
        assert!(
            tool_results[3].contains("hi from fallback"),
            "shell output reaches the model: {tool_results:#?}"
        );

        // claude_cli stays quarantined until the parsed "resets 4pm".
        let claude_health = health.get("claude_cli");
        assert_eq!(
            claude_health.state,
            roko_learn::provider_health::CircuitState::Open
        );
        let until = claude_health.cooldown_until.expect("quarantine end");
        let now = chrono::Utc::now().timestamp_millis();
        assert!(until > now && until <= now + 24 * 3_600_000, "{until}");

        // The next dispatch skips the quarantined CLI without calling it.
        let second = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08-next".to_string()),
            )
            .await
            .expect("next dispatch runs on the fallback");
        assert_eq!(
            second[0].body.as_text().expect("text"),
            "second task on fallback"
        );
        assert_eq!(invocations(&calls), 1);
        let requests = requests.lock();
        assert_eq!(requests.len(), 6);
        assert!(
            requests
                .iter()
                .all(|request| request["model"] == "api-model-1"),
            "every API turn names the fallback model"
        );
    }

    #[tokio::test]
    async fn exhausted_provider_without_usable_fallback_fails_once_with_fix_hint() {
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let config = Arc::new(failover_config(
            &claude,
            "http://127.0.0.1:9/v1",
            &["keyless-model"],
        ));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell(dispatcher, "claude-sonnet-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("no usable provider fails the task");

        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_EXHAUSTED_CATEGORY);
        assert!(!retryable);
        assert!(
            message.contains("You’ve hit your session limit"),
            "{message}"
        );
        assert!(
            message.contains("keyless-model: ROKO_TEST_FAILOVER_KEY_NEVER_SET is not set"),
            "{message}"
        );
        assert!(message.contains("~/.roko/.env"), "{message}");
        assert_eq!(
            invocations(&calls),
            1,
            "max_retries = 2 must not re-run an exhausted provider"
        );
    }

    /// `roko init` workspaces configure only `claude_cli` and the default
    /// model; a hint naming another model must never fail the task.
    #[tokio::test]
    async fn hint_for_unconfigured_provider_runs_same_slug_then_default_model() {
        let temp = tempdir().expect("tempdir");
        let args_log = temp.path().join("claude-args.log");
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo "$@" >> '{}'
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","total_cost_usd":0,"usage":{{"input_tokens":1,"output_tokens":1}}}}'
"#,
                args_log.display()
            ),
        );
        // What `roko init` + `synthesize_claude_cli_config` produce: one CLI
        // provider and a `[models.*]` entry for the default model only.
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet-4-6".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(claude.display().to_string()),
                args: None,
                timeout_ms: Some(15_000),
                ttft_timeout_ms: Some(15_000),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "claude-sonnet-4-6".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        // An API key in the environment synthesizes a usable standard provider
        // (`OPENAI_API_KEY` would send `gpt-4o` to the real OpenAI API), and
        // gate commands inherit the keys roko loads from `~/.roko/.env`. Shadow
        // each one with a provider whose key is never set.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            config.providers.insert(
                id.to_string(),
                ProviderConfig {
                    kind,
                    base_url: None,
                    api_key_env: Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET".to_string()),
                    command: None,
                    args: None,
                    timeout_ms: Some(15_000),
                    ttft_timeout_ms: Some(15_000),
                    connect_timeout_ms: Some(5_000),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                },
            );
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));

        // 1. `claude-opus-4-6` resolves to an unconfigured `anthropic_api`;
        //    claude_cli serves the same slug.
        let output = failover_cell(Arc::clone(&dispatcher), "claude-opus-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T1".to_string()),
            )
            .await
            .expect("hinted slug runs on claude_cli");
        assert_eq!(output[0].body.as_text().expect("text"), "ran");
        // 2. No configured provider can serve `gpt-4o`: the default model runs.
        failover_cell(dispatcher, "gpt-4o")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T2".to_string()),
            )
            .await
            .expect("default model runs");

        let calls = std::fs::read_to_string(&args_log).expect("claude invocations");
        let models: Vec<&str> = calls
            .lines()
            .filter_map(|line| line.split("--model ").nth(1)?.split_whitespace().next())
            .collect();
        assert_eq!(models, ["claude-opus-4-6", "claude-sonnet-4-6"], "{calls}");
    }

    // ─── Streaming dispatch tests (#274) ─────────────────────────────────────

    /// Helper: create a `GraphTaskDispatcher` with a fake CLI provider.
    async fn make_streaming_dispatcher(
        temp: &tempfile::TempDir,
        script_content: &str,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        make_test_dispatcher(
            temp,
            script_content,
            |_| {},
            GraphFeedbackContext::default(),
        )
        .await
    }

    /// Like [`make_streaming_dispatcher`], with a config tweak and feedback.
    async fn make_test_dispatcher(
        temp: &tempfile::TempDir,
        script_content: &str,
        configure: impl FnOnce(&mut RokoConfig),
        feedback: GraphFeedbackContext,
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
        configure(&mut config);
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_plan_budget(1.00, 0.50, false)
                .with_feedback(feedback),
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

    // ── P3-AGT-2: Express mode unit tests ─────────────────────────────────

    fn make_task_def(tier: &str) -> TaskDef {
        TaskDef {
            id: "T-EXP".to_string(),
            title: "express test task".to_string(),
            description: None,
            role: Some("implementer".to_string()),
            status: "pending".to_string(),
            tier: tier.to_string(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: Some(20),
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: 0,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        }
    }

    #[test]
    fn express_mode_disabled_by_default() {
        let config = roko_core::config::schema::RokoConfig::default();
        assert!(
            !config.conductor.express_mode,
            "express_mode must be false by default"
        );
        let task = make_task_def("mechanical");
        assert!(
            !is_express_task(&config, &task),
            "is_express_task must return false when express_mode = false"
        );
    }

    #[test]
    fn express_mode_activates_for_mechanical_and_trivial() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.conductor.express_mode = true;
        config.routing.fast_task_model = "claude-haiku-4-5".to_string();

        for tier in &["mechanical", "trivial", "Mechanical", "TRIVIAL"] {
            let task = make_task_def(tier);
            assert!(
                is_express_task(&config, &task),
                "is_express_task must return true for tier={tier} when express_mode=true"
            );
        }
    }

    #[test]
    fn express_mode_does_not_activate_for_standard_or_above() {
        let mut config = roko_core::config::schema::RokoConfig::default();
        config.conductor.express_mode = true;

        for tier in &["focused", "standard", "integrative", "architectural", ""] {
            let task = make_task_def(tier);
            assert!(
                !is_express_task(&config, &task),
                "is_express_task must return false for tier={tier}"
            );
        }
    }

    #[test]
    fn express_max_turns_is_less_than_theta_default() {
        let theta_default = roko_core::operating_frequency::OperatingFrequency::Theta.turn_limit();
        assert!(
            EXPRESS_MAX_TURNS < theta_default,
            "EXPRESS_MAX_TURNS ({EXPRESS_MAX_TURNS}) must be less than Theta default ({theta_default})"
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

    // ─── Dispatch overhead and config wiring ─────────────────────────────────

    /// A dispatcher over `config` whose factory owns no live provider.
    async fn make_bare_dispatcher(config: RokoConfig, workdir: &Path) -> GraphTaskDispatcher {
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        GraphTaskDispatcher::new(factory, config, workdir.to_path_buf())
    }

    fn cli_provider(command: &str) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            api_key_env: None,
            command: Some(command.to_string()),
            args: None,
            timeout_ms: Some(5_000),
            ttft_timeout_ms: Some(5_000),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        }
    }

    fn model(provider: &str, slug: &str, prices: Option<(f64, f64)>) -> ModelProfile {
        ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            supports_tools: true,
            cost_input_per_m: prices.map(|(input, _)| input),
            cost_output_per_m: prices.map(|(_, output)| output),
            ..ModelProfile::default()
        }
    }

    fn config_with_models(models: Vec<(&str, ModelProfile)>) -> RokoConfig {
        let mut config = RokoConfig::default();
        config.models.clear();
        config.routing.fast_task_model = String::new();
        for (key, profile) in models {
            config.models.insert(key.to_string(), profile);
        }
        config
    }

    /// Batch-path fixture: a fake Claude CLI reporting `cost_usd` per call.
    async fn make_batch_dispatcher(
        temp: &tempfile::TempDir,
        cost_usd: f64,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        let script = format!(
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' "$*" >> "$(dirname -- "$0")/provider-args"
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"batch-output"}}}}'
printf '%s\n' '{{"type":"result","session_id":"sess-b","model":"claude-sonnet-4-6","total_cost_usd":{cost_usd},"usage":{{"input_tokens":5,"output_tokens":10}}}}'
"#
        );
        make_scripted_batch_dispatcher(temp, &script, configure).await
    }

    /// [`make_batch_dispatcher`] with a caller-supplied fake Claude CLI.
    async fn make_scripted_batch_dispatcher(
        temp: &tempfile::TempDir,
        script_body: &str,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        let script = temp.path().join("fake-claude-batch.sh");
        std::fs::write(&script, script_body).expect("write batch provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "batch-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "batch-cli".to_string(),
            cli_provider(&script.display().to_string()),
        );
        config.models.insert(
            "batch-model".to_string(),
            model("batch-cli", "claude-sonnet-4-6", None),
        );
        configure(&mut config);

        let mut task = make_task_def("focused");
        task.title = "Wire the batch fixture".to_string();
        task.model_hint = Some("batch-model".to_string());
        task.timeout_secs = 5;
        (make_bare_dispatcher(config, temp.path()).await, task)
    }

    fn batch_ctx() -> CellContext {
        CellContext::new().with_cell_id("T-EXP".to_string())
    }

    #[test]
    fn cheap_model_prefers_a_dispatchable_fast_task_model() {
        let mut config = config_with_models(vec![
            ("mini", model("openai", "gpt-4o-mini", Some((0.15, 0.6)))),
            ("haiku", model("claude_cli", "claude-haiku-4-5", None)),
        ]);
        config.routing.fast_task_model = "claude-haiku-4-5".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("haiku"),
            "fast_task_model matches by slug"
        );
        config.routing.fast_task_model = "haiku".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("haiku"),
            "fast_task_model matches by key"
        );
        assert_eq!(
            select_cheap_model_key_with(&config, |key| key != "haiku").as_deref(),
            Some("mini"),
            "an undispatchable fast_task_model falls back to the cheapest model"
        );
        config.routing.fast_task_model = "not-configured".to_string();
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("mini")
        );
    }

    #[test]
    fn cheap_model_ranks_by_input_then_output_price_not_by_name() {
        let config = config_with_models(vec![
            ("a-expensive", model("p", "aaa-large", Some((3.0, 15.0)))),
            ("b-cheap-input", model("p", "bbb", Some((0.10, 2.0)))),
            ("c-cheapest", model("p", "ccc", Some((0.10, 0.40)))),
            ("d-unpriced", model("p", "zz-unknown-model", None)),
        ]);
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("c-cheapest")
        );
    }

    #[test]
    fn cheap_model_prices_unpriced_claude_models_from_the_builtin_table() {
        // The dogfood shape: Claude CLI models with no `cost_*` keys. The old
        // alphabetical pick returned the first key, not the cheapest model.
        let config = config_with_models(vec![
            ("claude-opus", model("claude_cli", "claude-opus-4-6", None)),
            (
                "claude-sonnet",
                model("claude_cli", "claude-sonnet-4-6", None),
            ),
            ("zeta-haiku", model("claude_cli", "claude-haiku-4-5", None)),
        ]);
        assert_eq!(
            select_cheap_model_key_with(&config, |_| true).as_deref(),
            Some("zeta-haiku")
        );
    }

    #[test]
    fn cheap_model_skips_embedding_search_disabled_and_offline_models() {
        let mut embedding = model("p", "text-embedding-3-small", Some((0.02, 0.0)));
        embedding.is_embedding_model = true;
        let mut search = model("perplexity", "sonar", Some((0.01, 0.01)));
        search.supports_tools = false;
        let mut config = config_with_models(vec![
            ("embed", embedding),
            ("sonar", search),
            (
                "disabled",
                model("groq", "llama-3.3-70b", Some((0.05, 0.05))),
            ),
            (
                "offline",
                model("p", "cheap-but-offline", Some((0.01, 0.01))),
            ),
            ("ok", model("p", "gpt-4o-mini", Some((0.15, 0.6)))),
        ]);
        config.routing.disabled_providers = vec!["groq".to_string()];
        assert_eq!(
            select_cheap_model_key_with(&config, |key| key != "offline").as_deref(),
            Some("ok")
        );
        assert_eq!(select_cheap_model_key_with(&config, |_| false), None);
    }

    #[tokio::test]
    async fn cheap_agent_uses_llm_call_timeout_and_the_selected_model_key() {
        let temp = tempdir().expect("tempdir");
        let mut config = config_with_models(vec![(
            "cli-sonnet",
            model("cli", "claude-sonnet-4-6", None),
        )]);
        config.providers.clear();
        config
            .providers
            .insert("cli".to_string(), cli_provider("/bin/sh"));
        config.timeouts.llm_call_secs = 7;
        let dispatcher = make_bare_dispatcher(config, temp.path()).await;

        let agent = dispatcher.cheap_agent().expect("a dispatchable model");
        assert_eq!(agent.model_key, "cli-sonnet");
        assert_eq!(agent.timeout_ms, 7_000);
    }

    #[test]
    fn every_task_gets_a_bounded_tier_turn_cap() {
        let mut config = RokoConfig::default();
        let turns =
            |config: &RokoConfig, tier: &str| task_turn_limit(config, &make_task_def(tier), false);
        assert_eq!(turns(&config, "mechanical"), 40);
        assert_eq!(turns(&config, "focused"), 60);
        assert_eq!(turns(&config, "integrative"), 90);
        assert_eq!(turns(&config, "architectural"), 120);
        assert_eq!(
            turns(&config, "unheard-of"),
            60,
            "unknown tiers use the focused band"
        );

        config.pipeline.integrative.max_turns = 45;
        assert_eq!(turns(&config, "integrative"), 45);

        config.conductor.express_mode = true;
        let mechanical = make_task_def("mechanical");
        assert!(is_express_task(&config, &mechanical));
        assert_eq!(
            task_turn_limit(&config, &mechanical, true),
            EXPRESS_MAX_TURNS
        );
        config.pipeline.mechanical.max_turns = 3;
        assert_eq!(
            task_turn_limit(&config, &mechanical, true),
            3,
            "express mode never raises the tier cap"
        );
    }

    #[test]
    fn task_budget_ceiling_is_the_tighter_configured_limit() {
        let mut budget = roko_core::config::BudgetConfig::default();
        let focused = make_task_def("focused");
        let integrative = make_task_def("integrative");
        // Defaults: no base task budget, $5 across all retries.
        assert_eq!(task_budget_ceiling_usd(&budget, &focused), 5.0);
        budget.max_task_usd = 2.0;
        assert_eq!(task_budget_ceiling_usd(&budget, &focused), 2.0);
        // Integrative scales the base by 3 ($6), clipped by the $5 retry cap.
        assert_eq!(task_budget_ceiling_usd(&budget, &integrative), 5.0);
        budget.max_task_retry_usd = 0.0;
        assert_eq!(task_budget_ceiling_usd(&budget, &integrative), 6.0);
        budget.max_task_usd = 0.0;
        assert_eq!(
            task_budget_ceiling_usd(&budget, &integrative),
            0.0,
            "all-zero limits are unlimited"
        );
    }

    #[test]
    fn task_spend_ledger_blocks_at_the_ceiling_per_task() {
        let ledger = GraphTaskSpendLedger::default();
        ledger.record("plan/T1", 0.30);
        assert!(ledger.admit("plan/T1", 0.50).is_ok());
        ledger.record("plan/T1", 0.20);
        let error = ledger
            .admit("plan/T1", 0.50)
            .expect_err("the ceiling has been reached");
        assert!(matches!(
            error,
            RokoError::BudgetExceeded {
                dimension: "task_cost_micro_usd",
                ..
            }
        ));
        assert!(ledger.admit("plan/T2", 0.50).is_ok(), "spend is per task");
        assert!(ledger.admit("plan/T1", 0.0).is_ok(), "zero is unlimited");
    }

    #[tokio::test]
    async fn task_budget_refuses_attempts_past_the_task_ceiling() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.10, |config| {
            config.budget.max_task_retry_usd = 0.10;
        })
        .await;
        let spec = make_spec(&task);

        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("the first attempt is admitted");
        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the retry would exceed the task ceiling");
        assert!(
            matches!(
                error,
                RokoError::BudgetExceeded {
                    dimension: "task_cost_micro_usd",
                    ..
                }
            ),
            "got {error:?}"
        );

        let mut other = task.clone();
        other.id = "T-OTHER".to_string();
        dispatcher
            .dispatch(&make_spec(&other), Vec::new(), &batch_ctx())
            .await
            .expect("other tasks keep their own budget");
    }

    #[tokio::test]
    async fn batch_dispatch_passes_the_tier_turn_cap_to_the_provider() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.pipeline.focused.max_turns = 7;
        })
        .await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        let args = std::fs::read_to_string(temp.path().join("provider-args"))
            .expect("the provider recorded its arguments");
        assert!(args.contains("--max-turns 7"), "provider args: {args}");
    }

    #[tokio::test]
    async fn no_budget_override_disables_the_task_ceiling() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.10, |config| {
            config.budget.max_task_retry_usd = 0.10;
        })
        .await;
        // `--no-budget`: zero ceiling with the explicit-override flag.
        let dispatcher = dispatcher.with_plan_budget(0.0, 0.0, true);
        let spec = make_spec(&task);
        for attempt in 0..2 {
            dispatcher
                .dispatch(&spec, Vec::new(), &batch_ctx())
                .await
                .unwrap_or_else(|error| panic!("attempt {attempt} must be admitted: {error}"));
        }
    }

    #[tokio::test]
    async fn eval_artifacts_are_opt_in_and_never_written_to_the_repo_root() {
        for write_eval_artifacts in [false, true] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
                config.gates.write_eval_artifacts = write_eval_artifacts;
            })
            .await;
            let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
                eval_generation_enabled: true,
                ..GraphFeedbackContext::default()
            });
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
                .await
                .expect("dispatch");

            assert!(!temp.path().join("generated-tests").exists());
            let written = std::fs::read_dir(temp.path().join(".roko/generated-tests"))
                .map(|entries| entries.count())
                .unwrap_or(0);
            assert_eq!(
                written > 0,
                write_eval_artifacts,
                "write_eval_artifacts={write_eval_artifacts} wrote {written} artifacts"
            );
        }
    }

    #[tokio::test]
    async fn skip_enrichment_plan_meta_is_read_and_suppresses_eval_artifacts() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.gates.write_eval_artifacts = true;
        })
        .await;
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            eval_generation_enabled: true,
            ..GraphFeedbackContext::default()
        });
        let plan_dir = temp.path().join("plans/authored");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"authored\"\nskip_enrichment = true\n\n\
             [[task]]\nid = \"T-EXP\"\ntitle = \"Wire the batch fixture\"\n",
        )
        .expect("write tasks.toml");
        let mut spec = make_spec(&task);
        spec.plan_id = "authored".to_string();
        spec.plan_dir = plan_dir.display().to_string();

        assert!(dispatcher.plan_skips_enrichment(&spec));
        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        assert!(!temp.path().join(".roko/generated-tests").exists());

        let mut unflagged = make_spec(&task);
        unflagged.plan_id = "unflagged".to_string();
        unflagged.plan_dir = temp.path().join("plans/missing").display().to_string();
        assert!(!dispatcher.plan_skips_enrichment(&unflagged));
    }

    #[test]
    fn retrieval_strategy_assignment_rewrites_the_store_only_on_registration() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("experiments.json");
        let arm = assign_retrieval_strategy_arm(&path);
        assert!(path.is_file(), "registration persists the experiment");

        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open store")
            .set_modified(old)
            .expect("backdate store");
        assert_eq!(assign_retrieval_strategy_arm(&path), arm);
        assert_eq!(
            std::fs::metadata(&path)
                .expect("store metadata")
                .modified()
                .expect("mtime"),
            old,
            "a steady-state dispatch must not rewrite the store"
        );
    }

    #[test]
    fn inert_settings_list_only_changed_keys_the_graph_engine_ignores() {
        assert!(graph_engine_inert_settings(&RokoConfig::default()).is_empty());

        let mut config = RokoConfig::default();
        config
            .gates
            .domain_gates
            .insert("docs".to_string(), vec!["shell:true".to_string()]);
        config.runner.warm_pool_size = 4;
        // Wired keys are never reported.
        config.pipeline.focused.max_turns = 50;
        config.budget.max_task_usd = 2.0;
        config.gates.write_eval_artifacts = true;
        let keys = graph_engine_inert_settings(&config)
            .iter()
            .map(|setting| setting.key)
            .collect::<Vec<_>>();
        assert_eq!(keys, ["gates.domain_gates", "runner.warm_pool_size"]);

        config.pipeline.focused.strategist = true;
        assert!(
            graph_engine_inert_settings(&config)
                .iter()
                .any(|setting| setting.key == "pipeline.focused")
        );
    }

    #[test]
    fn a_turn_cap_retry_raises_the_cap_by_half() {
        assert_eq!(raised_turn_cap(60), 90);
        assert_eq!(raised_turn_cap(1), 2);
        assert_eq!(raised_turn_cap(2), 3);
        assert_eq!(raised_turn_cap(u32::MAX), u32::MAX);
    }

    #[test]
    fn provider_failures_get_a_class_prefixed_reason() {
        let hit = roko_agent::provider::error_classify::TurnCapHit {
            num_turns: Some(61),
            cap: Some(60),
        };
        assert!(provider_failure_reason(&hit.to_string()).starts_with("turn_cap: agent turn cap"));
        assert!(
            provider_failure_reason("provider usage exhausted: You've hit your session limit")
                .starts_with("provider_exhausted: ")
        );
        assert_eq!(
            provider_failure_reason("exit 1: claude failed\nmore"),
            "provider: exit 1: claude failed\nmore"
        );
    }

    #[test]
    fn a_verify_failure_reason_keeps_the_failing_step() {
        let summary = verify_failure_summary(
            "Write the greeting",
            3,
            &[
                "verify[1:test] `cargo test -p greet` failed: exit code: 101\n\
               thread 'greets' panicked at src/lib.rs:4:5"
                    .to_string(),
            ],
            &["verify[2:lint] (`cargo clippy`)".to_string()],
        );
        let reason = verify_failure_reason(&RokoError::Verify {
            gate: "graph-verify".into(),
            message: summary,
        });
        assert!(reason.starts_with("verify: 1/3 verify step(s) failed for task"));
        assert!(reason.contains("verify[1:test] `cargo test -p greet` failed"));
        assert!(reason.contains("panicked at src/lib.rs:4:5"));
        assert!(
            reason.ends_with("Skipped after the first failure: verify[2:lint] (`cargo clippy`)")
        );
        assert_eq!(
            attempt_failure_reason("verify", "  \n"),
            "verify: no detail"
        );
    }

    #[test]
    fn a_long_failure_reason_keeps_its_head_and_tail_within_the_bound() {
        let detail = (0..400)
            .map(|line| format!("line {line:03} of the failing é output"))
            .collect::<Vec<_>>()
            .join("\n");
        let reason = attempt_failure_reason("verify", &detail);

        assert!(reason.len() <= MAX_FAILURE_REASON_BYTES, "{}", reason.len());
        assert!(reason.starts_with("verify: line 000 of the failing é output\n"));
        assert!(reason.ends_with("line 399 of the failing é output"));
        assert!(reason.contains(" bytes omitted …\n"));
        // Cuts land on line breaks, so no line is kept in part.
        assert!(
            reason.lines().all(|line| (line.starts_with("verify: line ")
                || line.starts_with("line "))
                && line.ends_with(" output")
                || line.contains("bytes omitted")),
            "{reason}"
        );
    }

    /// A fake Claude CLI that stops at its turn cap on the first call and
    /// finishes on the second, recording each call's args and prompt.
    const TURN_CAP_THEN_SUCCESS_PROVIDER: &str = r#"#!/bin/sh
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/calls"
printf '%s\n' "$*" >> "$dir/provider-args"
cat > "$dir/prompt-$n"
if [ ! -f "$dir/stopped-once" ]; then
  touch "$dir/stopped-once"
  printf '%s\n' '{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":61,"total_cost_usd":0.02}'
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"finished"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"num_turns":7,"total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    #[tokio::test]
    async fn a_turn_cap_stop_is_classified_then_resumed_with_a_raised_cap() {
        let temp = tempdir().expect("tempdir");
        let episodes_path = temp.path().join("episodes.jsonl");
        let (dispatcher, task) =
            make_scripted_batch_dispatcher(&temp, TURN_CAP_THEN_SUCCESS_PROVIDER, |_| {}).await;
        let facade = crate::runtime_feedback::FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::EpisodeSink::at(&episodes_path),
        ));
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            ..GraphFeedbackContext::default()
        });
        let spec = make_spec(&task);

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the first attempt stops at the focused cap");
        assert!(
            matches!(
                error,
                RokoError::TurnLimitReached {
                    limit: 60,
                    num_turns: 61,
                    ..
                }
            ),
            "got {error:?}"
        );

        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("the resumed attempt finishes");
        let args = std::fs::read_to_string(temp.path().join("provider-args")).expect("args");
        let caps: Vec<&str> = args
            .split("--max-turns ")
            .skip(1)
            .filter_map(|rest| rest.split_whitespace().next())
            .collect();
        assert_eq!(caps, ["60", "90"], "the retry must not rerun the same cap");
        let resumed_prompt =
            std::fs::read_to_string(temp.path().join("prompt-2")).expect("second prompt");
        assert!(resumed_prompt.contains("Resuming a partially completed task"));
        assert!(
            !std::fs::read_to_string(temp.path().join("prompt-1"))
                .expect("first prompt")
                .contains("Resuming")
        );

        let episodes: Vec<serde_json::Value> = std::fs::read_to_string(&episodes_path)
            .expect("episodes")
            .lines()
            .map(|line| serde_json::from_str(line).expect("episode json"))
            .collect();
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0]["success"], false);
        assert_eq!(episodes[0]["turns"], 61);
        assert!(
            episodes[0]["failure_reason"]
                .as_str()
                .is_some_and(|reason| reason.starts_with("turn_cap: agent turn cap reached")),
            "{}",
            episodes[0]
        );
        assert_eq!(episodes[0]["extra"]["failure_class"], "turn_cap");
        assert_eq!(episodes[1]["success"], true);
        assert_eq!(episodes[1]["turns"], 7);
        assert!(episodes[1]["failure_reason"].is_null());
    }
}
