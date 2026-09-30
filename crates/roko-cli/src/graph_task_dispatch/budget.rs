//! Spend accounting at the Graph dispatch boundary: the per-plan budget ledger
//! and its reservations, and the per-task spend ledger.

use super::*;

const MICRO_USD_PER_USD: f64 = 1_000_000.0;

/// Per-plan cost policy applied at the Graph task-dispatch boundary.
///
/// A non-positive or non-finite ceiling means unlimited. When
/// `continue_on_exhaustion` is enabled, spend is still recorded and exposed
/// for observability, but new dispatches are not blocked. This mirrors the
/// existing Runner-v2 semantics for explicit CLI budget overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphPlanBudgetPolicy {
    pub(super) ceiling_micro_usd: Option<u64>,
    reservation_micro_usd: Option<u64>,
    pub(super) continue_on_exhaustion: bool,
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
pub(super) struct GraphPlanBudgetLedger {
    plans: parking_lot::Mutex<HashMap<String, PlanBudgetState>>,
}

impl GraphPlanBudgetLedger {
    pub(super) fn attach_checkpoint(
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

    pub(super) fn snapshot(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
    ) -> GraphPlanBudgetSnapshot {
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

    /// Why no further dispatch of `plan_id` can run: settled spend reached
    /// the ceiling with no override to continue, or the cost ledger cannot be
    /// persisted. In-flight reservations alone never stop a plan.
    pub(super) fn dispatch_stop(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
    ) -> Option<String> {
        let plans = self.plans.lock();
        let state = plans.get(plan_id)?;
        if let Some(error) = &state.persistence_error {
            return Some(format!("plan cost ledger unavailable: {error}"));
        }
        let ceiling = policy
            .ceiling_micro_usd
            .filter(|_| !policy.continue_on_exhaustion)?;
        (state.spent_micro_usd >= ceiling).then(|| {
            format!(
                "plan budget exhausted: ${:.4} spent of ${:.4}",
                micro_usd_to_usd(state.spent_micro_usd),
                micro_usd_to_usd(ceiling)
            )
        })
    }

    pub(super) fn reserve(
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

    pub(super) fn settle(
        &self,
        plan_id: &str,
        reserved_micro_usd: u64,
        cost_usd: f64,
    ) -> Result<()> {
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

pub(super) struct GraphPlanBudgetReservation<'a> {
    ledger: &'a GraphPlanBudgetLedger,
    plan_id: String,
    reserved_micro_usd: u64,
    routing_budget_micro_usd: Option<u64>,
    settled: bool,
}

impl GraphPlanBudgetReservation<'_> {
    pub(super) fn routing_budget_usd(&self) -> f64 {
        self.routing_budget_micro_usd
            .map_or(f64::INFINITY, micro_usd_to_usd)
    }

    pub(super) fn settle(mut self, cost_usd: f64) -> Result<()> {
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
pub(super) fn task_budget_ceiling_usd(
    budget: &roko_core::config::BudgetConfig,
    task: &TaskDef,
) -> f64 {
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
pub(super) struct GraphTaskSpendLedger {
    tasks: parking_lot::Mutex<HashMap<String, u64>>,
}

impl GraphTaskSpendLedger {
    pub(super) fn record(&self, task_key: &str, cost_usd: f64) {
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
    pub(super) fn admit(&self, task_key: &str, ceiling_usd: f64) -> Result<()> {
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

pub(super) fn effective_routing_budget(context_remaining: Option<f64>, plan_remaining: f64) -> f64 {
    let context_remaining = context_remaining
        .filter(|value| value.is_finite())
        .map_or(f64::INFINITY, |value| value.max(0.0));
    context_remaining.min(plan_remaining)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        STREAMS_THEN_TIMES_OUT_PROVIDER, TIMEOUT_SECS_UNDER_LOAD, batch_ctx, make_batch_dispatcher,
        make_scripted_batch_dispatcher, make_spec, make_task_def,
    };

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
    fn only_settled_spend_at_the_ceiling_stops_dispatch() {
        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::from_ceiling(0.10, false);
        assert_eq!(ledger.dispatch_stop("plan-a", policy), None);

        // A reservation of the whole remaining budget blocks further
        // reservations but does not stop the plan.
        let reservation = ledger.reserve("plan-a", policy).expect("reservation");
        assert!(ledger.snapshot("plan-a", policy).dispatch_blocked);
        assert_eq!(ledger.dispatch_stop("plan-a", policy), None);

        reservation.settle(0.10).expect("settle at the ceiling");
        let stop = ledger
            .dispatch_stop("plan-a", policy)
            .expect("spent plan stops");
        assert!(stop.starts_with("plan budget exhausted"), "{stop}");
        assert_eq!(
            ledger.dispatch_stop("plan-a", GraphPlanBudgetPolicy::from_ceiling(0.10, true)),
            None,
            "an explicit override keeps the plan going"
        );
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

    /// The first `costs.jsonl` record, once the background writer lands it.
    async fn first_cost_record(path: &Path) -> serde_json::Value {
        crate::background_writes::settled(path.parent().unwrap_or(path)).await;
        for _ in 0..600 {
            if let Some(record) = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .find_map(|line| serde_json::from_str(line).ok())
            {
                return record;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("no cost record was written to {}", path.display());
    }

    #[tokio::test]
    async fn a_timed_out_attempt_settles_the_spend_it_streamed() {
        // Sonnet per million: $3 in, $15 out.
        let streamed_usd = (1_000.0 * 3.0 + 200.0 * 15.0) / 1e6;
        for timeout_secs in TIMEOUT_SECS_UNDER_LOAD {
            let temp = tempdir().expect("tempdir");
            let costs_path = temp.path().join("costs.jsonl");
            let (dispatcher, mut task) =
                make_scripted_batch_dispatcher(&temp, STREAMS_THEN_TIMES_OUT_PROVIDER, |_| {})
                    .await;
            let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
                costs_path: Some(costs_path.clone()),
                ..GraphFeedbackContext::default()
            });
            task.timeout_secs = timeout_secs;
            let spec = make_spec(&task);

            let expected = format!("timed out after {} ms", timeout_secs * 1_000);
            let error = dispatcher
                .dispatch(&spec, Vec::new(), &batch_ctx())
                .await
                .expect_err("the attempt runs out of time");
            assert!(
                matches!(&error, RokoError::Agent { message, .. } if *message == expected),
                "got {error:?}"
            );

            let plan_spent = dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd;
            if plan_spent <= 0.0 {
                // The provider ran out of time before its message arrived.
                continue;
            }
            assert!(
                (plan_spent - streamed_usd).abs() < 1e-6,
                "plan ledger settled {plan_spent}"
            );
            let task_spent = dispatcher
                .task_spend
                .tasks
                .lock()
                .get(&format!("{}/{}", spec.plan_id, task.id))
                .copied();
            assert_eq!(task_spent, Some(usd_to_micro_usd(streamed_usd)));

            let record = first_cost_record(&costs_path).await;
            assert_eq!(record["success"], false, "{record}");
            assert_eq!(record["input_tokens"], 1_000, "{record}");
            assert_eq!(record["output_tokens"], 200, "{record}");
            let recorded_usd = record["cost_usd"].as_f64().expect("cost_usd");
            assert!((recorded_usd - streamed_usd).abs() < 1e-6, "{record}");
            return;
        }
        panic!("no provider streamed its message before its time ran out");
    }
}
