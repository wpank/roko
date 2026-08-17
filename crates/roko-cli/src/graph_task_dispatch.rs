//! Host adapter that makes converted Graph plan tasks execute real agents.

use std::collections::{HashMap, hash_map::Entry};
use std::path::PathBuf;
use std::sync::Arc;

use roko_agent::safety::contract::{AgentContract, ContractLoadMode};
use roko_core::config::schema::RokoConfig;
use roko_core::error::{Result, RokoError};
use roko_core::{Body, Kind, Signal};
use roko_graph::cell::CellContext;
use roko_graph::cells::{TaskDispatcher, TaskExecutionSpec};

use crate::dispatch::{AgentDispatchRequest, DispatchContext, SharedAgentFactory};
use crate::graph_checkpoint::GraphCostLedgerCheckpoint;
use crate::task_parser::TaskDef;

const MICRO_USD_PER_USD: f64 = 1_000_000.0;

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

/// Real runner/provider adapter injected into `TaskExecutorCell` factories.
pub struct GraphTaskDispatcher {
    factory: Arc<SharedAgentFactory>,
    config: Arc<RokoConfig>,
    workdir: PathBuf,
    budget_policy: GraphPlanBudgetPolicy,
    budget_ledger: GraphPlanBudgetLedger,
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
        }
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
        let role = task.role.as_deref().unwrap_or("implementer");
        let dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: self.workdir.clone(),
            // Preserve the configured default when the task has no author hint.
            model_hint: Some(self.config.agent.default_model.clone()),
            force_backend: None,
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: 0,
            // Graph does not yet own runner terminal feedback receipts.
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: None,
            routing_bias: None,
            dependency_outputs: upstream_outputs(&input),
        };
        let dispatch_plan = self
            .factory
            .dispatcher()
            .plan(&task, &dispatch_ctx)
            .map_err(|error| RokoError::Planning(error.to_string()))?;
        let contract = effective_agent_contract(role, &task);
        let timeout_ms = spec.timeout_secs.max(1).saturating_mul(1_000);
        let request = AgentDispatchRequest {
            model_key: dispatch_plan.model.slug.clone(),
            prompt: dispatch_plan.prompt.user_prompt,
            system_prompt: dispatch_plan.prompt.system_prompt,
            workdir: self.workdir.clone(),
            immune_root: Some(self.workdir.clone()),
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
            // Graph execution never silently bypasses provider permissions.
            dangerously_skip_permissions: false,
        };

        let dispatch = self
            .factory
            .run_shared_agent_bridge(request)
            .await
            .map_err(|error| RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: error.to_string(),
            })?;

        // Account for every completed provider call, including unsuccessful
        // results: callers may still have incurred the reported cost.
        budget_reservation.settle(f64::from(dispatch.result.usage.cost_usd))?;

        if !dispatch.result.success {
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

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
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
}
