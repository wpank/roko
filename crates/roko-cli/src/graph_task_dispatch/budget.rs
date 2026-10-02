//! Spend accounting at the Graph dispatch boundary: the per-plan budget ledger
//! and its reservations and alerts, the per-task spend ledger, and the daily
//! ceiling.

use roko_core::dashboard_snapshot::{InboxCategory, inbox_routing};

use super::*;

const MICRO_USD_PER_USD: f64 = 1_000_000.0;

/// Per-plan cost policy applied at the Graph task-dispatch boundary.
///
/// A non-positive or non-finite ceiling means unlimited. When
/// `continue_on_exhaustion` is enabled, spend is still recorded and exposed
/// for observability, but new dispatches are not blocked. Only `--no-budget`
/// enables it, with an unlimited ceiling, which also turns off the per-task
/// and daily checks; `--budget-override` is a hard ceiling (gap-d31457).
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
                    // can be in flight at a time; the others wait for it to
                    // settle (bug-0bc2b4).
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
    /// Calls of the plan whose cost was never priced (backlog 2111).
    unpriced_calls: usize,
    /// The spend a resumed run restored from the plan's `costs.json`: an
    /// earlier process of the run announced the alerts it passed.
    restored_micro_usd: u64,
    /// The `budget.alert_at_percent` thresholds announced so far (backlog
    /// 2116).
    alerted_percent: Vec<u8>,
    checkpoint: Option<GraphCostLedgerCheckpoint>,
    persistence_error: Option<String>,
}

/// A `budget.alert_at_percent` threshold of a plan's ceiling that its
/// settled spend crossed (backlog 2116).
#[derive(Debug, Clone, Copy, PartialEq)]
struct PlanBudgetAlert {
    /// The threshold, in percent of the ceiling.
    percent: u8,
    spent_usd: f64,
    ceiling_usd: f64,
}

impl PlanBudgetAlert {
    /// The Inbox item id: one per plan and threshold.
    fn item_id(self, plan_id: &str) -> String {
        format!("budget:{plan_id}:{}", self.percent)
    }

    fn summary(self, plan_id: &str) -> String {
        let Self {
            percent,
            spent_usd,
            ceiling_usd,
        } = self;
        format!("plan {plan_id} has spent ${spent_usd:.4} of ${ceiling_usd:.4} ({percent}%)")
    }
}

#[derive(Debug, Default)]
pub(super) struct GraphPlanBudgetLedger {
    plans: parking_lot::Mutex<HashMap<String, PlanBudgetState>>,
    /// Woken whenever a reservation settles or is released, so a reservation
    /// waiting for capacity tries again (bug-0bc2b4).
    capacity: tokio::sync::Notify,
}

/// How often a reservation waiting for capacity checks again without a
/// wake-up, and whether its run stopped.
const RESERVE_RECHECK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Why [`GraphPlanBudgetLedger::try_reserve`] refused a reservation.
struct ReserveRefusal {
    error: RokoError,
    /// Only reservations in flight hold the plan's remaining budget: it has
    /// capacity again once they settle. Otherwise settled spend reached the
    /// ceiling, or the ledger cannot be persisted.
    blocked: bool,
}

impl ReserveRefusal {
    const fn failed(error: RokoError) -> Self {
        Self {
            error,
            blocked: false,
        }
    }
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
                    restored_micro_usd: checkpoint.spent_micro_usd(),
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
    /// the ceiling with no override to continue, a call of the plan settled
    /// unpriced so its spend is unknown (decision 2110), or the cost ledger
    /// cannot be persisted. In-flight reservations alone never stop a plan.
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
        if state.unpriced_calls > 0 {
            return Some(unpriced_plan_stop(state.unpriced_calls, ceiling));
        }
        (state.spent_micro_usd >= ceiling).then(|| {
            format!(
                "plan budget exhausted: ${:.4} spent of ${:.4}",
                micro_usd_to_usd(state.spent_micro_usd),
                micro_usd_to_usd(ceiling)
            )
        })
    }

    /// Reserve capacity for one provider call of `plan_id`, failing at once
    /// when there is none.
    #[cfg(test)]
    pub(super) fn reserve(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
    ) -> Result<GraphPlanBudgetReservation<'_>> {
        self.try_reserve(plan_id, policy)
            .map_err(|refusal| refusal.error)
    }

    /// [`Self::reserve`], waiting while only reservations in flight leave the
    /// plan no capacity (bug-0bc2b4). Without `max_turn_usd` a call reserves
    /// the plan's whole remaining budget, so a task that starts beside it
    /// waits for it to settle instead of failing. It fails once settled spend
    /// reaches the ceiling, and ends with a cancellation once `stopped`.
    pub(super) async fn reserve_waiting(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
        stopped: impl Fn() -> bool,
    ) -> Result<GraphPlanBudgetReservation<'_>> {
        loop {
            let capacity = self.capacity.notified();
            match self.try_reserve(plan_id, policy) {
                Ok(reservation) => return Ok(reservation),
                Err(refusal) if !refusal.blocked => return Err(refusal.error),
                Err(_) if stopped() => {
                    return Err(RokoError::cancelled(format!(
                        "the run stopped while a task of plan `{plan_id}` waited for its budget"
                    )));
                }
                Err(_) => {
                    let _ = tokio::time::timeout(RESERVE_RECHECK_INTERVAL, capacity).await;
                }
            }
        }
    }

    fn try_reserve(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
    ) -> std::result::Result<GraphPlanBudgetReservation<'_>, ReserveRefusal> {
        let mut plans = self.plans.lock();
        let state = plans.entry(plan_id.to_string()).or_default();
        if let Some(error) = &state.persistence_error {
            return Err(ReserveRefusal::failed(RokoError::Store(format!(
                "Graph cost ledger for plan `{plan_id}` is unavailable: {error}"
            ))));
        }

        let mut reserved_micro_usd = 0;
        let routing_budget_micro_usd = match policy.ceiling_micro_usd {
            None => None,
            Some(ceiling) if policy.continue_on_exhaustion => {
                Some(ceiling.saturating_sub(state.spent_micro_usd))
            }
            Some(ceiling) => {
                // A plan whose spend is unknown admits no further call
                // (decision 2110), as the daily ceiling does (bug-ae28ac).
                if state.unpriced_calls > 0 {
                    let stop = unpriced_plan_stop(state.unpriced_calls, ceiling);
                    return Err(ReserveRefusal::failed(RokoError::Config(stop)));
                }
                let committed = state
                    .spent_micro_usd
                    .saturating_add(state.reserved_micro_usd);
                let available = ceiling.saturating_sub(committed);
                if available == 0 {
                    let exceeded = RokoError::BudgetExceeded {
                        dimension: "plan_cost_micro_usd",
                        used: micro_usd_to_usize(committed),
                        limit: micro_usd_to_usize(ceiling),
                    };
                    return Err(ReserveRefusal {
                        error: exceeded,
                        blocked: state.spent_micro_usd < ceiling,
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
                    return Err(ReserveRefusal::failed(RokoError::Store(message)));
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
            drop(plans);
            self.capacity.notify_waiters();
            return Err(RokoError::Store(message));
        }
        drop(plans);
        self.capacity.notify_waiters();
        Ok(())
    }

    /// The thresholds of `alert_at_percent` that `plan_id`'s settled spend
    /// crossed since the last call, against the policy's ceiling, lowest
    /// first (backlog 2116). Each threshold is returned once. One that the
    /// spend a resumed run restored had passed is not returned: the earlier
    /// process of the run announced it.
    fn take_threshold_alerts(
        &self,
        plan_id: &str,
        policy: GraphPlanBudgetPolicy,
        alert_at_percent: &[u8],
    ) -> Vec<PlanBudgetAlert> {
        let Some(ceiling) = policy.ceiling_micro_usd else {
            return Vec::new();
        };
        let mut plans = self.plans.lock();
        let Some(state) = plans.get_mut(plan_id) else {
            return Vec::new();
        };
        let reached = |micro_usd: u64, percent: u8| {
            u128::from(micro_usd) * 100 >= u128::from(ceiling) * u128::from(percent)
        };
        let mut percents = alert_at_percent
            .iter()
            .copied()
            .filter(|percent| *percent > 0)
            .collect::<Vec<_>>();
        percents.sort_unstable();
        percents.dedup();
        let mut alerts = Vec::new();
        for percent in percents {
            let announced = state.alerted_percent.contains(&percent);
            if announced || !reached(state.spent_micro_usd, percent) {
                continue;
            }
            state.alerted_percent.push(percent);
            if !reached(state.restored_micro_usd, percent) {
                alerts.push(PlanBudgetAlert {
                    percent,
                    spent_usd: micro_usd_to_usd(state.spent_micro_usd),
                    ceiling_usd: micro_usd_to_usd(ceiling),
                });
            }
        }
        alerts
    }

    /// Count a call of `plan_id` whose cost was never priced (backlog 2111).
    /// Under a plan ceiling the plan then admits no further call: its spend
    /// is unknown, so the ceiling cannot be enforced.
    pub(super) fn record_unpriced(&self, plan_id: &str) {
        let mut plans = self.plans.lock();
        let state = plans.entry(plan_id.to_string()).or_default();
        state.unpriced_calls = state.unpriced_calls.saturating_add(1);
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
        drop(plans);
        self.capacity.notify_waiters();
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
/// attempt of the run, for per-task ceiling admission, and the process's
/// spend across all tasks, for the daily ceiling.
///
/// The run's retry state keeps each task's spend beside its Graph checkpoint
/// ([`GraphTaskDispatcher::record_task_spend`]), and a resumed run restores
/// it, so the ceiling counts every attempt of the run (gap-34b2ed).
#[derive(Debug, Default)]
pub(super) struct GraphTaskSpendLedger {
    /// This process's spend per task.
    tasks: parking_lot::Mutex<HashMap<String, u64>>,
    /// Calls whose cost was never priced: they used tokens at $0.
    unpriced_calls: std::sync::atomic::AtomicUsize,
    /// Spend per task that earlier processes of a resumed run recorded. It
    /// counts toward the task's ceiling, not toward this process's spend.
    earlier: parking_lot::Mutex<HashMap<String, u64>>,
}

/// What this process has spent on provider calls so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ProcessSpend {
    micro_usd: u64,
    /// Calls whose cost was never priced.
    unpriced_calls: usize,
}

impl GraphTaskSpendLedger {
    /// Record one provider call of `task_key`, at the cost its usage
    /// reports. A call that used tokens at $0 was never priced: its cost
    /// is unknown, not zero ([`roko_core::Usage::has_known_cost`]).
    pub(super) fn record(&self, task_key: &str, usage: &roko_core::Usage) {
        if !usage.has_known_cost() {
            self.unpriced_calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        self.record_usd(task_key, f64::from(usage.cost_usd));
    }

    fn record_usd(&self, task_key: &str, cost_usd: f64) {
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
        let spent_micro_usd = self.task_total(task_key);
        if spent_micro_usd >= ceiling_micro_usd {
            return Err(RokoError::BudgetExceeded {
                dimension: "task_cost_micro_usd",
                used: micro_usd_to_usize(spent_micro_usd),
                limit: micro_usd_to_usize(ceiling_micro_usd),
            });
        }
        Ok(())
    }

    /// `task_key`'s spend over the run's attempts: this process's, and what
    /// earlier processes of a resumed run recorded.
    pub(super) fn task_total(&self, task_key: &str) -> u64 {
        let own = self.tasks.lock().get(task_key).copied().unwrap_or(0);
        let earlier = self.earlier.lock().get(task_key).copied().unwrap_or(0);
        own.saturating_add(earlier)
    }

    /// Count `micro_usd`, which an earlier process of a resumed run recorded
    /// for `task_key`, toward the task's ceiling (gap-34b2ed).
    pub(super) fn restore(&self, task_key: &str, micro_usd: u64) {
        self.earlier.lock().insert(task_key.to_string(), micro_usd);
    }

    /// Everything this process has recorded, across all tasks.
    pub(super) fn process_spend(&self) -> ProcessSpend {
        ProcessSpend {
            micro_usd: self
                .tasks
                .lock()
                .values()
                .fold(0, |total, spent| total.saturating_add(*spent)),
            unpriced_calls: self
                .unpriced_calls
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// `budget.max_daily_usd` at the Graph dispatch boundary (bug-ae28ac).
///
/// Today's spend (the UTC calendar day, across every run and command that
/// records to `.roko/learn/costs.jsonl`) is what that log held when this
/// process read it for today, plus what this process has recorded since. The
/// log is read once a day: when a run starts
/// ([`GraphTaskDispatcher::prime_daily_budget`]), and again by the first
/// dispatch after midnight UTC, so spend that other processes record
/// meanwhile is not seen. Calls count once they settle, so the day can
/// overshoot its ceiling by the calls running when it is reached.
///
/// A call whose cost was never priced makes the day's spend unknown, and an
/// unknown spend counts as over the ceiling: a money control fails closed.
#[derive(Debug, Default)]
pub(super) struct GraphDailyBudget {
    /// The day the log was last read for, and what it held then.
    baseline: parking_lot::Mutex<Option<DailyBaseline>>,
}

/// What the costs log held for `day`, and this process's spend just before
/// it was read.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DailyBaseline {
    day: chrono::NaiveDate,
    logged: roko_learn::costs_log::DaySpend,
    process_at_read: ProcessSpend,
}

/// `budget.max_daily_usd`, read as a ceiling.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DailyCeiling {
    /// `0.0`, the documented default.
    Unlimited,
    MicroUsd(u64),
    /// Negative, NaN or infinite: no dispatch can be shown to fit under it.
    Malformed(f32),
}

impl DailyCeiling {
    fn from_config(max_daily_usd: f32) -> Self {
        if max_daily_usd == 0.0 {
            Self::Unlimited
        } else if max_daily_usd.is_finite() && max_daily_usd > 0.0 {
            Self::MicroUsd(usd_to_micro_usd(f64::from(max_daily_usd)).max(1))
        } else {
            Self::Malformed(max_daily_usd)
        }
    }
}

/// Why `budget.max_daily_usd` allows no further provider call today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DailyStop {
    /// Today's spend reached the ceiling.
    Spent {
        spent_micro_usd: u64,
        ceiling_micro_usd: u64,
    },
    /// Today's spend includes calls whose cost was never priced.
    Unpriced {
        calls: usize,
        ceiling_micro_usd: u64,
    },
    /// The ceiling is negative, NaN or infinite; the value's bits.
    Malformed(u32),
}

impl DailyStop {
    fn error(self) -> RokoError {
        match self {
            Self::Spent {
                spent_micro_usd,
                ceiling_micro_usd,
            } => RokoError::BudgetExceeded {
                dimension: "daily spend in micro-USD (budget.max_daily_usd)",
                used: micro_usd_to_usize(spent_micro_usd),
                limit: micro_usd_to_usize(ceiling_micro_usd),
            },
            Self::Unpriced { .. } | Self::Malformed(_) => RokoError::Config(self.to_string()),
        }
    }
}

impl std::fmt::Display for DailyStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Spent {
                spent_micro_usd,
                ceiling_micro_usd,
            } => write!(
                f,
                "daily budget exhausted: ${:.4} spent today (UTC) of budget.max_daily_usd = ${:.4}",
                micro_usd_to_usd(spent_micro_usd),
                micro_usd_to_usd(ceiling_micro_usd)
            ),
            Self::Unpriced {
                calls,
                ceiling_micro_usd,
            } => write!(
                f,
                "budget.max_daily_usd = ${:.4} cannot be enforced: today's spend (UTC) includes \
                 {calls} provider call(s) whose cost was never priced; give their models a price \
                 in [models], or run with --no-budget",
                micro_usd_to_usd(ceiling_micro_usd)
            ),
            Self::Malformed(bits) => write!(
                f,
                "budget.max_daily_usd = {} is not a ceiling: set a positive amount, or 0 for \
                 unlimited",
                f32::from_bits(bits)
            ),
        }
    }
}

/// Why a plan under `ceiling_micro_usd` admits no further call once `calls`
/// of its calls settled unpriced (decision 2110, backlog 2111).
fn unpriced_plan_stop(calls: usize, ceiling_micro_usd: u64) -> String {
    format!(
        "plan budget cannot be enforced: {calls} unpriced call(s) against max_plan_usd = ${:.4}; \
         give their models a price in [models], or run with --no-budget",
        micro_usd_to_usd(ceiling_micro_usd)
    )
}

/// Whether today's spend, `baseline` plus what the process recorded since
/// (`now`), allows no further call under `ceiling`.
fn daily_stop(
    ceiling: DailyCeiling,
    baseline: DailyBaseline,
    now: ProcessSpend,
) -> Option<DailyStop> {
    let ceiling_micro_usd = match ceiling {
        DailyCeiling::Unlimited => return None,
        DailyCeiling::Malformed(value) => return Some(DailyStop::Malformed(value.to_bits())),
        DailyCeiling::MicroUsd(ceiling) => ceiling,
    };
    let at_read = baseline.process_at_read;
    let unpriced = baseline
        .logged
        .unpriced_calls
        .saturating_add(now.unpriced_calls.saturating_sub(at_read.unpriced_calls));
    if unpriced > 0 {
        return Some(DailyStop::Unpriced {
            calls: unpriced,
            ceiling_micro_usd,
        });
    }
    let spent_micro_usd = usd_to_micro_usd(baseline.logged.cost_usd)
        .saturating_add(now.micro_usd.saturating_sub(at_read.micro_usd));
    (spent_micro_usd >= ceiling_micro_usd).then_some(DailyStop::Spent {
        spent_micro_usd,
        ceiling_micro_usd,
    })
}

impl GraphTaskDispatcher {
    /// Announce each `budget.alert_at_percent` threshold of `plan_id`'s
    /// ceiling that its settled spend crossed since the last call (backlog
    /// 2116): one warning line and one `budget_alert` Inbox item per
    /// threshold, once. Alerts only notify: the plan still stops at its
    /// ceiling.
    pub(super) fn announce_budget_alerts(&self, plan_id: &str) {
        let alerts = self.budget_ledger.take_threshold_alerts(
            plan_id,
            self.budget_policy,
            &self.config.budget.alert_at_percent,
        );
        for alert in alerts {
            let summary = alert.summary(plan_id);
            tracing::warn!(
                plan_id,
                percent = alert.percent,
                spent_usd = alert.spent_usd,
                ceiling_usd = alert.ceiling_usd,
                "budget alert: {summary}"
            );
            if let Some(tui) = &self.tui_bridge {
                let category = InboxCategory::BudgetAlert;
                tui.inbox_item(
                    &alert.item_id(plan_id),
                    category,
                    inbox_routing(category).urgency,
                    &summary,
                );
            }
        }
    }

    /// Record a provider call of `plan_id/task_id` toward the task's ceiling,
    /// and keep the task's spend with the run's retry state, so a resumed run
    /// counts it as well (gap-34b2ed).
    pub(super) fn record_task_spend(&self, plan_id: &str, task_id: &str, usage: &roko_core::Usage) {
        let key = format!("{plan_id}/{task_id}");
        self.task_spend.record(&key, usage);
        // An unpriced call leaves the plan's spend unknown (backlog 2111).
        if !usage.has_known_cost() {
            self.budget_ledger.record_unpriced(plan_id);
        }
        let spent = self.task_spend.task_total(&key);
        self.gate_retry_context
            .set_task_spend(plan_id, task_id, spent);
    }

    /// Read today's spend so far from the costs log, so that a run whose day
    /// is already spent starts no task ([`Self::plan_dispatch_stop`]).
    /// Dispatches read it themselves otherwise.
    pub async fn prime_daily_budget(&self) {
        if DailyCeiling::from_config(self.config.budget.max_daily_usd) != DailyCeiling::Unlimited {
            self.daily_baseline().await;
        }
    }

    /// Refuse a provider dispatch once today's spend reached
    /// `budget.max_daily_usd`, mirroring the plan ceiling: a policy that
    /// continues on exhaustion only warns, and `--no-budget` disables the
    /// check.
    pub(super) async fn admit_daily_budget(&self, spec: &TaskExecutionSpec) -> Result<()> {
        let policy = self.budget_policy;
        if policy.continue_on_exhaustion && policy.ceiling_micro_usd.is_none() {
            return Ok(());
        }
        let ceiling = DailyCeiling::from_config(self.config.budget.max_daily_usd);
        if ceiling == DailyCeiling::Unlimited {
            return Ok(());
        }
        let baseline = self.daily_baseline().await;
        let Some(stop) = daily_stop(ceiling, baseline, self.task_spend.process_spend()) else {
            return Ok(());
        };
        if policy.continue_on_exhaustion {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task = %spec.title,
                "{stop}; continuing, as the plan's budget policy allows"
            );
            return Ok(());
        }
        tracing::warn!(
            plan_id = %spec.plan_id,
            task = %spec.title,
            "{stop}; refusing the provider dispatch"
        );
        Err(stop.error())
    }

    /// Why no further task may start today under `budget.max_daily_usd`, as
    /// far as the last read of the costs log for today tells: before that
    /// read, or once the day is over, the next dispatch decides.
    pub(super) fn daily_dispatch_stop(&self) -> Option<String> {
        let policy = self.budget_policy;
        if policy.continue_on_exhaustion {
            return None;
        }
        let ceiling = DailyCeiling::from_config(self.config.budget.max_daily_usd);
        let today = chrono::Utc::now().date_naive();
        let baseline =
            (*self.daily_budget.baseline.lock()).filter(|baseline| baseline.day == today);
        daily_stop(ceiling, baseline?, self.task_spend.process_spend()).map(|stop| stop.to_string())
    }

    /// Today's baseline: the one read for today, or a fresh read of the
    /// costs log. An unreadable log counts as no spend, with a warning.
    async fn daily_baseline(&self) -> DailyBaseline {
        let today = chrono::Utc::now().date_naive();
        let cached = *self.daily_budget.baseline.lock();
        if let Some(baseline) = cached
            && baseline.day == today
        {
            return baseline;
        }
        // Taken before the read: a call that settles meanwhile may count
        // twice, never not at all.
        let process_at_read = self.task_spend.process_spend();
        let logged = match &self.feedback.costs_path {
            Some(path) => roko_learn::costs_log::CostsLog::at(path)
                .spend_on(today)
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        path = %path.display(),
                        %error,
                        "cannot read today's spend for budget.max_daily_usd; counting none"
                    );
                    roko_learn::costs_log::DaySpend::default()
                }),
            None => roko_learn::costs_log::DaySpend::default(),
        };
        let read = DailyBaseline {
            day: today,
            logged,
            process_at_read,
        };
        let mut baseline = self.daily_budget.baseline.lock();
        match *baseline {
            Some(stored) if stored.day == today => stored,
            _ => {
                *baseline = Some(read);
                read
            }
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

pub(super) fn effective_routing_budget(context_remaining: Option<f64>, plan_remaining: f64) -> f64 {
    let context_remaining = context_remaining
        .filter(|value| value.is_finite())
        .map_or(f64::INFINITY, |value| value.max(0.0));
    context_remaining.min(plan_remaining)
}

#[cfg(test)]
mod tests {
    use roko_core::dashboard_snapshot::UrgencyLevel;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        STREAMS_THEN_TIMES_OUT_PROVIDER, TIMEOUT_SECS_UNDER_LOAD, VERIFY_PROVIDER, batch_ctx,
        make_bare_dispatcher, make_batch_dispatcher, make_scripted_batch_dispatcher, make_spec,
        make_task_def,
    };
    use crate::state_hub::StateHub;

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

    /// backlog 2116: each alert threshold is announced once, lowest first,
    /// against the ceiling. A resumed run does not announce again the
    /// thresholds that the spend it restored had passed; no ceiling, or an
    /// empty list, announces nothing.
    #[test]
    fn threshold_alerts_are_announced_once_and_not_again_on_resume() {
        let policy = GraphPlanBudgetPolicy::from_ceiling(1.0, false);
        let thresholds = [80, 50, 50, 0];
        let take = |ledger: &GraphPlanBudgetLedger, policy: GraphPlanBudgetPolicy| -> Vec<u8> {
            ledger
                .take_threshold_alerts("plan-a", policy, &thresholds)
                .into_iter()
                .map(|alert| alert.percent)
                .collect()
        };
        let ledger = GraphPlanBudgetLedger::default();
        ledger.record_cost("plan-a", 0.49);
        assert!(take(&ledger, policy).is_empty());
        ledger.record_cost("plan-a", 0.40);
        assert_eq!(take(&ledger, policy), [50, 80]);
        assert!(take(&ledger, policy).is_empty(), "each is announced once");

        // An earlier process of the run spent $0.60 and announced 50%.
        let resumed = GraphPlanBudgetLedger::default();
        let restored = PlanBudgetState {
            spent_micro_usd: 600_000,
            restored_micro_usd: 600_000,
            ..PlanBudgetState::default()
        };
        resumed.plans.lock().insert("plan-a".to_string(), restored);
        assert!(take(&resumed, policy).is_empty());
        resumed.record_cost("plan-a", 0.25);
        let alerts = resumed.take_threshold_alerts("plan-a", policy, &thresholds);
        let crossed = PlanBudgetAlert {
            percent: 80,
            spent_usd: 0.85,
            ceiling_usd: 1.0,
        };
        assert_eq!(alerts, [crossed]);
        assert_eq!(
            crossed.summary("plan-a"),
            "plan plan-a has spent $0.8500 of $1.0000 (80%)"
        );

        let unlimited = GraphPlanBudgetLedger::default();
        unlimited.record_cost("plan-a", 5.0);
        assert!(take(&unlimited, GraphPlanBudgetPolicy::unlimited()).is_empty());
        let off = GraphPlanBudgetLedger::default();
        off.record_cost("plan-a", 0.90);
        let alerts = off.take_threshold_alerts("plan-a", policy, &[]);
        assert!(alerts.is_empty(), "an empty list turns alerts off");
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

    /// bug-0bc2b4: with a plan budget and no `max_turn_usd` a call reserves
    /// the plan's whole remaining budget. A task dispatched beside it waits
    /// for that reservation to settle instead of failing, and both run.
    #[tokio::test]
    async fn concurrent_tasks_wait_for_a_reserved_plan_budget() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.10, |_| {}).await;
        let dispatcher = dispatcher.with_plan_budget(1.0, 0.0, false);
        let mut other = task.clone();
        other.id = "T-OTHER".to_string();
        let (spec, other_spec) = (make_spec(&task), make_spec(&other));
        let ctx = batch_ctx();

        let first = dispatcher.dispatch(&spec, Vec::new(), &ctx);
        let second = dispatcher.dispatch(&other_spec, Vec::new(), &ctx);
        let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            tokio::join!(first, second)
        })
        .await
        .expect("both tasks finish");
        first.expect("the first task runs");
        second.expect("the second waits for the budget, then runs");
        let spent = dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd;
        assert!((spent - 0.20).abs() < 1e-6, "{spent}");
    }

    /// backlog 2116: the plan budget raises a `budget_alert` Inbox item at
    /// 50% and at 80% of its ceiling, once each, before the ceiling stops
    /// the plan. At $0.03 a call under a $0.10 ceiling, the second call
    /// ($0.06) crosses 50% and the third ($0.09) 80%. The fourth is still
    /// admitted, takes the plan past its ceiling, and the plan stops.
    #[tokio::test]
    async fn plan_budget_emits_threshold_events() {
        let temp = tempdir().expect("tempdir");
        let hub = StateHub::new(64);
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.03, |_| {}).await;
        let dispatcher = dispatcher
            .with_plan_budget(0.10, 0.0, false)
            .with_tui_bridge(TuiBridge::new(hub.sender()));
        let ctx = batch_ctx();
        // The budget alerts in the Inbox, by id.
        let budget_alerts = || {
            let mut alerts = hub
                .current_snapshot()
                .inbox_items
                .into_values()
                .filter(|item| item.category == InboxCategory::BudgetAlert)
                .map(|item| (item.item_id, item.urgency, item.summary))
                .collect::<Vec<_>>();
            alerts.sort_by(|left, right| left.0.cmp(&right.0));
            alerts
        };
        let alert = |percent: u8, spent: &str| {
            (
                format!("budget:stream-plan:{percent}"),
                UrgencyLevel::Question,
                format!("plan stream-plan has spent {spent} of $0.1000 ({percent}%)"),
            )
        };

        let mut seen = Vec::new();
        for call in 1..=4 {
            let mut next = task.clone();
            next.id = format!("T-{call}");
            let spec = make_spec(&next);
            assert_eq!(dispatcher.plan_dispatch_stop(&spec.plan_id), None, "{call}");
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .unwrap_or_else(|error| panic!("call {call} is admitted: {error}"));
            seen.push(budget_alerts());
        }
        assert!(seen[0].is_empty(), "$0.03 is 30% of the ceiling");
        assert_eq!(seen[1], [alert(50, "$0.0600")]);
        assert_eq!(seen[2], [alert(50, "$0.0600"), alert(80, "$0.0900")]);
        assert_eq!(seen[3], seen[2], "each threshold is announced once");

        let stop = dispatcher
            .plan_dispatch_stop("stream-plan")
            .expect("the plan is spent");
        assert!(
            stop.starts_with("plan budget exhausted: $0.1200 spent of $0.1000"),
            "{stop}"
        );
        let mut last = task.clone();
        last.id = "T-5".to_string();
        let error = dispatcher
            .dispatch(&make_spec(&last), Vec::new(), &ctx)
            .await
            .expect_err("the spent plan admits no further call");
        assert!(
            matches!(
                error,
                RokoError::BudgetExceeded {
                    dimension: "plan_cost_micro_usd",
                    ..
                }
            ),
            "got {error:?}"
        );
    }

    /// A reservation waiting for capacity fails once settled spend reaches
    /// the ceiling, and ends with a cancellation once its run stops.
    #[tokio::test]
    async fn a_waiting_reservation_ends_at_the_ceiling_or_a_stop() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let ledger = GraphPlanBudgetLedger::default();
        let policy = GraphPlanBudgetPolicy::from_ceiling(0.50, false);
        let first = ledger
            .reserve_waiting("plan-a", policy, || false)
            .await
            .expect("capacity");
        let stopped = AtomicBool::new(false);
        let waiting = ledger.reserve_waiting("plan-a", policy, || stopped.load(Ordering::SeqCst));
        let stop = async {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            stopped.store(true, Ordering::SeqCst);
        };
        let (waited, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(waiting, stop)
        })
        .await
        .expect("the stop ends the wait");
        assert!(matches!(waited.err(), Some(RokoError::Cancelled(_))));

        first.settle(0.50).expect("settle at the ceiling");
        let spent = ledger
            .reserve_waiting("plan-a", policy, || false)
            .await
            .err()
            .expect("the plan is spent");
        assert!(
            matches!(spent, RokoError::BudgetExceeded { .. }),
            "{spent:?}"
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
        ledger.record_usd("plan/T1", 0.30);
        assert!(ledger.admit("plan/T1", 0.50).is_ok());
        ledger.record_usd("plan/T1", 0.20);
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

    /// gap-288e38: a timed-out attempt's `costs.jsonl` row says its cost was
    /// priced from the usage it streamed (`estimated`), `CostRecord` reads
    /// that back, and the log's totals show it apart. A row whose usage the
    /// provider reported says so (`cli_usage` for the Claude CLI).
    #[tokio::test]
    async fn a_timed_out_attempt_cost_record_is_marked_estimated() {
        let temp = tempdir().expect("tempdir");
        let costs_path = temp.path().join("costs.jsonl");
        let (dispatcher, task) =
            make_scripted_batch_dispatcher(&temp, VERIFY_PROVIDER, |_| {}).await;
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            costs_path: Some(costs_path.clone()),
            ..GraphFeedbackContext::default()
        });
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("the attempt completes");
        let reported = first_cost_record(&costs_path).await;
        assert_eq!(reported["cost_source"], "cli_usage", "{reported}");

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
            dispatcher
                .dispatch(&spec, Vec::new(), &batch_ctx())
                .await
                .expect_err("the attempt runs out of time");
            if dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd <= 0.0 {
                // The provider ran out of time before its message arrived.
                continue;
            }

            let row = first_cost_record(&costs_path).await;
            assert_eq!(row["cost_source"], "estimated", "{row}");
            let record: roko_learn::costs_db::CostRecord =
                serde_json::from_value(row).expect("the row is a CostRecord");
            assert_eq!(
                record.cost_source,
                roko_learn::telemetry::CostSource::Estimated
            );
            let estimated = roko_learn::costs_log::CostsLog::at(&costs_path)
                .estimated_cost()
                .await
                .expect("read the costs");
            assert!(estimated > 0.0 && (estimated - record.cost_usd).abs() < 1e-9);
            return;
        }
        panic!("no provider streamed its message before its time ran out");
    }

    /// gap-34b2ed: a task's spend and the turn-cap retry it is owed are kept
    /// with the run's retry state. A resumed process of the run counts the
    /// spend toward the task's ceiling, not as its own, and raises the cap;
    /// a fresh run starts from nothing.
    #[tokio::test]
    async fn task_spend_and_turn_cap_retry_survive_resume() {
        let temp = tempdir().expect("tempdir");
        let kept = temp
            .path()
            .join(".roko/state/graph/plan/retry-feedback.json");
        let usage = roko_core::Usage {
            input_tokens: 1_000,
            output_tokens: 200,
            cost_usd: 0.10,
            ..roko_core::Usage::default()
        };
        let retry = TurnCapRetry {
            cap: 60,
            num_turns: Some(61),
        };

        let first = make_bare_dispatcher(RokoConfig::default(), temp.path()).await;
        first.attach_retry_feedback("plan", kept.clone(), "run-1");
        first.record_task_spend("plan", "T1", &usage);
        first.keep_turn_cap_retry("plan", "T1", retry);
        drop(first);

        let resumed = make_bare_dispatcher(RokoConfig::default(), temp.path()).await;
        resumed.attach_retry_feedback("plan", kept.clone(), "run-1");
        assert!(
            resumed.task_spend.admit("plan/T1", 0.10).is_err(),
            "the earlier process's spend counts toward the task"
        );
        assert_eq!(
            resumed.task_spend.process_spend(),
            ProcessSpend::default(),
            "but not as this process's spend"
        );
        assert_eq!(resumed.take_turn_cap_retry("plan", "T1"), Some(retry));
        drop(resumed);

        let fresh = make_bare_dispatcher(RokoConfig::default(), temp.path()).await;
        fresh.attach_retry_feedback("plan", kept, "run-2");
        assert!(fresh.task_spend.admit("plan/T1", 0.10).is_ok());
        assert_eq!(fresh.take_turn_cap_retry("plan", "T1"), None);
    }

    // ── budget.max_daily_usd (bug-ae28ac) ───────────────────────────────

    /// A call an earlier run recorded at `timestamp`.
    fn earlier_call(timestamp: chrono::DateTime<chrono::Utc>, cost_usd: f64) -> CostRecord {
        CostRecord {
            timestamp: timestamp.to_rfc3339(),
            model: "claude-sonnet-4-6".to_string(),
            provider: "claude_cli".to_string(),
            role: "implementer".to_string(),
            plan_id: "earlier-plan".to_string(),
            task_id: "T1".to_string(),
            complexity_band: "focused".to_string(),
            input_tokens: 1_000,
            output_tokens: 200,
            cached_tokens: 0,
            cost_usd,
            duration_ms: 1_000,
            success: true,
            session_id: "earlier-run".to_string(),
            cost_source: roko_learn::telemetry::CostSource::CliUsage,
            priced: None,
        }
    }

    /// A batch dispatcher whose workspace costs log holds `earlier` calls,
    /// under `budget.max_daily_usd = max_daily_usd`. Each call costs $0.10,
    /// or, unless `priced`, uses tokens at $0 of a model with no price.
    async fn daily_dispatcher(
        temp: &tempfile::TempDir,
        max_daily_usd: f32,
        earlier: &[CostRecord],
        priced: bool,
    ) -> (GraphTaskDispatcher, TaskDef) {
        let costs_path = temp.path().join(".roko/learn/costs.jsonl");
        std::fs::create_dir_all(costs_path.parent().expect("learn dir")).expect("learn dir");
        let lines = earlier
            .iter()
            .map(|record| serde_json::to_string(record).expect("cost record") + "\n")
            .collect::<String>();
        std::fs::write(&costs_path, lines).expect("write the costs log");
        let cost_usd = if priced { 0.10 } else { 0.0 };
        let (dispatcher, task) = make_batch_dispatcher(temp, cost_usd, |config| {
            config.budget.max_daily_usd = max_daily_usd;
            if !priced && let Some(profile) = config.models.get_mut("batch-model") {
                profile.slug = "unpriced-test-model".to_string();
            }
        })
        .await;
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            costs_path: Some(costs_path),
            ..GraphFeedbackContext::default()
        });
        (dispatcher, task)
    }

    /// Whether the fake provider was launched.
    fn provider_ran(temp: &tempfile::TempDir) -> bool {
        temp.path().join("provider-args").exists()
    }

    /// Once today's spend before the run reaches `budget.max_daily_usd`,
    /// the run starts no task, and a dispatch is refused before any provider
    /// call. Yesterday's spend does not count.
    #[tokio::test]
    async fn graph_daily_budget_blocks_dispatch() {
        let temp = tempdir().expect("tempdir");
        let now = chrono::Utc::now();
        let earlier = [
            earlier_call(now - chrono::Duration::days(1), 5.0),
            earlier_call(now, 0.60),
            earlier_call(now, 0.40),
        ];
        let (dispatcher, task) = daily_dispatcher(&temp, 1.0, &earlier, true).await;
        let spec = make_spec(&task);

        dispatcher.prime_daily_budget().await;
        let stop = dispatcher
            .plan_dispatch_stop(&spec.plan_id)
            .expect("the day is spent");
        assert!(
            stop.starts_with("daily budget exhausted: $1.0000 spent today"),
            "{stop}"
        );
        assert!(stop.contains("budget.max_daily_usd"), "{stop}");

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the day is spent");
        assert!(
            matches!(
                error,
                RokoError::BudgetExceeded {
                    dimension,
                    used: 1_000_000,
                    limit: 1_000_000,
                } if dimension.contains("budget.max_daily_usd")
            ),
            "got {error:?}"
        );
        assert!(!provider_ran(&temp), "the provider was never called");
    }

    /// This process's own calls count toward the day: once they take it to
    /// the ceiling, the next dispatch is refused.
    #[tokio::test]
    async fn the_runs_own_spend_counts_toward_the_daily_ceiling() {
        let temp = tempdir().expect("tempdir");
        let earlier = [earlier_call(chrono::Utc::now(), 0.15)];
        let (dispatcher, task) = daily_dispatcher(&temp, 0.20, &earlier, true).await;
        let spec = make_spec(&task);
        dispatcher.prime_daily_budget().await;
        assert_eq!(dispatcher.plan_dispatch_stop(&spec.plan_id), None);

        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("$0.15 of $0.20 spent: the call is admitted");
        assert!(dispatcher.plan_dispatch_stop(&spec.plan_id).is_some());
        let mut other = task.clone();
        other.id = "T-OTHER".to_string();
        let error = dispatcher
            .dispatch(&make_spec(&other), Vec::new(), &batch_ctx())
            .await
            .expect_err("$0.25 of $0.20 spent");
        assert!(
            matches!(
                error,
                RokoError::BudgetExceeded { dimension, used, .. }
                    if dimension.contains("budget.max_daily_usd") && used >= 250_000
            ),
            "got {error:?}"
        );
    }

    /// `--no-budget` turns the daily check off, and a daily ceiling of `0.0`
    /// is unlimited, so a spent day still dispatches. `--budget-override`
    /// lifts only the plan ceiling (`a_budget_override_leaves_a_spent_day_spent`).
    #[tokio::test]
    async fn no_budget_and_a_zero_daily_ceiling_let_a_spent_day_dispatch() {
        let earlier = [earlier_call(chrono::Utc::now(), 3.0)];
        for (label, max_daily_usd, override_ceiling) in [
            ("--no-budget", 1.0, Some(0.0)),
            ("max_daily_usd = 0", 0.0, None),
        ] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, task) = daily_dispatcher(&temp, max_daily_usd, &earlier, true).await;
            let dispatcher = match override_ceiling {
                Some(ceiling) => dispatcher.with_plan_budget(ceiling, 0.0, true),
                None => dispatcher,
            };
            let spec = make_spec(&task);
            dispatcher.prime_daily_budget().await;
            assert_eq!(
                dispatcher.plan_dispatch_stop(&spec.plan_id),
                None,
                "{label}"
            );
            dispatcher
                .dispatch(&spec, Vec::new(), &batch_ctx())
                .await
                .unwrap_or_else(|error| panic!("{label}: the call is admitted: {error}"));
            assert!(provider_ran(&temp), "{label}");
        }
    }

    /// gap-d31457: `--budget-override` sets a hard plan ceiling and lifts no
    /// other limit, so a spent day still starts no task.
    #[tokio::test]
    async fn a_budget_override_leaves_a_spent_day_spent() {
        let temp = tempdir().expect("tempdir");
        let earlier = [earlier_call(chrono::Utc::now(), 3.0)];
        let (dispatcher, task) = daily_dispatcher(&temp, 1.0, &earlier, true).await;
        // The policy `--budget-override 10` runs with.
        let dispatcher = dispatcher.with_plan_budget(10.0, 0.0, false);
        let spec = make_spec(&task);
        dispatcher.prime_daily_budget().await;
        assert!(dispatcher.plan_dispatch_stop(&spec.plan_id).is_some());

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the day is spent");
        assert!(
            matches!(
                error,
                RokoError::BudgetExceeded { dimension, .. }
                    if dimension.contains("budget.max_daily_usd")
            ),
            "got {error:?}"
        );
        assert!(!provider_ran(&temp), "the provider was never called");
    }

    /// A call whose cost was never priced makes the day's spend unknown,
    /// which counts as over the ceiling: earlier runs' calls, and this
    /// process's own.
    #[tokio::test]
    async fn unknown_spend_counts_as_over_the_daily_ceiling() {
        let temp = tempdir().expect("tempdir");
        let earlier = [earlier_call(chrono::Utc::now(), 0.0)];
        let (dispatcher, task) = daily_dispatcher(&temp, 5.0, &earlier, true).await;
        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect_err("today's spend is unknown");
        assert!(
            matches!(&error, RokoError::Config(message) if message.contains("1 provider call(s) whose cost was never priced")),
            "got {error:?}"
        );
        assert!(!provider_ran(&temp));

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = daily_dispatcher(&temp, 5.0, &[], false).await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("nothing is unknown yet");
        let mut other = task.clone();
        other.id = "T-OTHER".to_string();
        let error = dispatcher
            .dispatch(&make_spec(&other), Vec::new(), &batch_ctx())
            .await
            .expect_err("the first call used tokens at $0");
        assert!(matches!(error, RokoError::Config(_)), "got {error:?}");
    }

    /// backlog 2111 (decision 2110, option a): once a call of a plan with a
    /// ceiling settles unpriced, the plan's spend is unknown, and it admits
    /// no further call, as the daily ceiling fails closed. Without a ceiling
    /// the plan dispatches on.
    #[tokio::test]
    async fn plan_ceiling_refuses_an_unpriced_call() {
        for ceiling in [Some(1.0), None] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, task) = daily_dispatcher(&temp, 0.0, &[], false).await;
            let dispatcher = match ceiling {
                Some(ceiling) => dispatcher.with_plan_budget(ceiling, 0.0, false),
                None => dispatcher,
            };
            let spec = make_spec(&task);
            dispatcher
                .dispatch(&spec, Vec::new(), &batch_ctx())
                .await
                .expect("nothing is unknown yet");
            let stop = dispatcher.plan_dispatch_stop(&spec.plan_id);
            let mut other = task.clone();
            other.id = "T-OTHER".to_string();
            let next = dispatcher
                .dispatch(&make_spec(&other), Vec::new(), &batch_ctx())
                .await;
            if ceiling.is_none() {
                assert_eq!(stop, None);
                next.expect("without a ceiling the plan dispatches on");
                continue;
            }
            let stop = stop.expect("the first call used tokens at $0");
            assert!(
                stop.starts_with("plan budget cannot be enforced: 1 unpriced call(s)"),
                "{stop}"
            );
            let error = next.expect_err("no further call of the plan is admitted");
            assert!(
                matches!(&error, RokoError::Config(message) if message == &stop),
                "got {error:?}"
            );
        }
    }

    /// A negative, NaN or infinite ceiling refuses every dispatch.
    #[tokio::test]
    async fn a_malformed_daily_ceiling_fails_closed() {
        assert_eq!(DailyCeiling::from_config(0.0), DailyCeiling::Unlimited);
        assert_eq!(
            DailyCeiling::from_config(1.5),
            DailyCeiling::MicroUsd(1_500_000)
        );
        for value in [-1.0, f32::NAN, f32::INFINITY] {
            assert!(
                matches!(DailyCeiling::from_config(value), DailyCeiling::Malformed(_)),
                "{value}"
            );
        }

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = daily_dispatcher(&temp, -1.0, &[], true).await;
        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect_err("a negative ceiling admits nothing");
        assert!(
            matches!(&error, RokoError::Config(message) if message.contains("is not a ceiling")),
            "got {error:?}"
        );
    }

    /// Yesterday's total is never carried into a new UTC day: the first
    /// dispatch of the day reads the costs log again.
    #[tokio::test]
    async fn a_new_utc_day_reads_the_costs_log_again() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = daily_dispatcher(&temp, 1.0, &[], true).await;
        let today = chrono::Utc::now().date_naive();
        *dispatcher.daily_budget.baseline.lock() = Some(DailyBaseline {
            day: today.pred_opt().expect("yesterday"),
            logged: roko_learn::costs_log::DaySpend {
                cost_usd: 5.0,
                unpriced_calls: 0,
            },
            process_at_read: ProcessSpend::default(),
        });
        let spec = make_spec(&task);
        assert_eq!(dispatcher.plan_dispatch_stop(&spec.plan_id), None);
        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("today's log holds nothing");
        let baseline = (*dispatcher.daily_budget.baseline.lock()).expect("read today");
        assert_eq!(baseline.day, today);
    }
}
