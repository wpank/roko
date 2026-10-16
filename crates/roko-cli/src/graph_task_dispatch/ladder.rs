//! Verify-then-escalate on the model ladder (gap-460230).
//!
//! A task that `[routing.ladder]` routes climbs one runnable rung after
//! [`FAILURES_PER_RUNG`] agent-blamed failures on the rung it runs on (a
//! failed verify step, a turn-cap stop, a try that spent its rung's
//! allowance, or a timeout after the first token), at most
//! [`MAX_ESCALATIONS`] times and never past its top rung. Each try on a rung
//! the task can still climb from is bounded (`try_allowance`). Provider and
//! harness failures never count; they stay with provider failover. A pinned
//! model (`--model`, a task's `model_hint`, express routing) never moves, and
//! the dispatch log says so.
//!
//! Every attempt's verdict records its rung, the rungs the task climbed and
//! why it ran there ([`AttemptLadder`]). The task's standing is kept beside
//! its retry feedback ([`LadderStanding`]), so a resumed run climbs from where
//! the last one stopped. A task whose last attempt fails on its top rung has
//! exhausted the ladder: dispatch logs `ladder_exhausted` and tells the
//! dashboard to split or replan the task. Nothing splits it automatically.
//!
//! M1's B1 knobs bound the ladder per attempt (S06, decision 8101, 8124):
//! the θ the attempt runs raises its tier's start rung to `tier_floor`, and
//! `tier_cap` stops its climb as the top rung does. Only what M1 moved from
//! θ₀ binds, so θ₀ moves nothing; a pinned model ignores both, and a task
//! already above a cap M1 lowered keeps its rung. A floor below θ₀'s binds
//! nothing either, since the floor only raises the start rung, and M1's
//! search never sets one (bug-35a738).

use std::borrow::Cow;
use std::collections::BTreeMap;

use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};
use roko_core::task::TaskTier;
use roko_learn::routing_log::RoutingDecisionLog;
use roko_learn::self_model::cascade::StepAction;
use roko_learn::telemetry::{AttemptLadder, AttemptVerdictRecord, Blame, LadderReason};

use super::attempt::AttemptContext;
use super::retry_feedback::LadderStanding;
use super::try_allowance::RungAllowance;
use super::*;
use crate::dispatch::model_routing::SELF_MODEL_LOOP;
use crate::dispatch::{RoutingLadder, RunnerDispatchPlan};
use crate::runtime_feedback::homeostasis::HarnessDecision;

/// Agent-blamed failures on one rung before the task climbs to the next.
const FAILURES_PER_RUNG: u32 = 2;

/// Most rungs a task climbs above its start rung (`K_max`).
const MAX_ESCALATIONS: u32 = 2;

/// What a person should do about a task that exhausted the ladder.
const SPLIT_OR_REPLAN: &str = "Split the task into smaller tasks, or replan it.";

/// Retry budget of a task that does not author `max_retries` while the
/// ladder routes it: [`FAILURES_PER_RUNG`] attempts on its start rung and on
/// each rung it may climb, less the first attempt.
pub(super) const LADDER_MIN_RETRIES: u32 = FAILURES_PER_RUNG * (MAX_ESCALATIONS + 1) - 1;

/// Why a try runs where it does, as `route_decided` and `attempt_settled`
/// say it (gap-aaaaae).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TryReason {
    /// The task's first try.
    Start,
    /// Another try on the rung its last try ran on.
    Retry,
    /// Its first try on a higher rung.
    Climb,
    /// Failover moved the try to another model.
    Failover,
    /// A pin chose the model.
    Pinned,
}

impl TryReason {
    /// The reason's word.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Retry => "retry",
            Self::Climb => "climb",
            Self::Failover => "failover",
            Self::Pinned => "pinned",
        }
    }
}

/// What dispatch remembers of each task's tries (gap-aaaaae): the step of
/// its last routed try, by task (`"{plan_id}/{task_id}"`), and why each
/// unsettled try runs where it does, by attempt key.
#[derive(Debug, Default)]
pub(super) struct TryBook {
    last_step: HashMap<String, u32>,
    reasons: HashMap<String, TryReason>,
}

/// M1's B1 bounds on one task's rungs, as indices among its role's rungs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TierBounds {
    /// The lowest rung the task starts on (`tier_floor`).
    floor: Option<usize>,
    /// The highest rung the task climbs to (`tier_cap`).
    cap: Option<usize>,
}

impl TierBounds {
    /// Whether rung `index` is at or above the cap, so the task climbs no
    /// further from it.
    fn caps(self, index: usize) -> bool {
        self.cap.is_some_and(|cap| index >= cap)
    }
}

impl GraphTaskDispatcher {
    /// `[routing.ladder]` as dispatch binds it: `None` when it is off or no
    /// rung can run here.
    fn routing_ladder(&self) -> Option<&RoutingLadder> {
        self.factory.dispatcher().routing_ladder()
    }

    /// Least retry budget of a task that does not author `max_retries`:
    /// enough to climb the ladder while it routes tasks (`--model` stops
    /// that), else none.
    pub(super) fn ladder_min_retries(&self) -> u32 {
        if self.cli_model_override.is_none() && self.routing_ladder().is_some() {
            LADDER_MIN_RETRIES
        } else {
            0
        }
    }

    /// The retry budget `task` runs under now: the one M1's live source last
    /// handed out (B2, 8126), else its plan-load budget. The ladder's last
    /// chance is the attempt this budget ends on (gap-26c055).
    fn live_max_retries(&self, spec: &TaskExecutionSpec, task: &TaskDef) -> u32 {
        self.homeostasis_sink()
            .and_then(|sink| sink.retry_limit(&spec.plan_id, &task.id))
            .unwrap_or(spec.max_retries)
    }

    /// Rungs `task` climbed above its start rung, where its next attempt
    /// routes.
    pub(super) fn ladder_step(&self, spec: &TaskExecutionSpec, task: &TaskDef) -> u32 {
        self.gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id)
            .escalations
    }

    /// Whether a pin chooses `task`'s model: `--model`, express routing, or
    /// the task's `model_hint` or `preferred_model`. The ladder never moves a
    /// pinned model, and M1's B1 knobs with it (8124).
    pub(super) fn model_pinned(&self, task: &TaskDef) -> bool {
        self.cli_model_override.is_some()
            || is_express_task(&self.config, task)
            || task.model_hint.is_some()
            || task.hints.preferred_model.is_some()
    }

    /// The bounds M1's `decision` puts on `task`'s rungs: the rungs its θ
    /// names for the task's tier, where they differ from θ₀'s. θ is
    /// role-less, so θ₀'s floor and cap would move a role's own start or
    /// rungs; only what M1 moved binds (decision 8101: θ₀ moves nothing). A
    /// rung the role's ladder lacks bounds nothing.
    fn tier_bounds(&self, task: &TaskDef, decision: Option<&HarnessDecision>) -> TierBounds {
        let (Some(decision), Some(ladder)) = (decision, self.routing_ladder()) else {
            return TierBounds::default();
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let tier = task.tier_class();
        let moved = |theta: &BTreeMap<TaskTier, String>, theta0: &BTreeMap<TaskTier, String>| {
            let name = theta
                .get(&tier)
                .filter(|&name| theta0.get(&tier) != Some(name))?;
            ladder.rung_index(role, name)
        };
        let (theta, theta0) = (&decision.applied, &decision.default);
        TierBounds {
            floor: moved(&theta.tier_floor, &theta0.tier_floor),
            cap: moved(&theta.tier_cap, &theta0.tier_cap),
        }
    }

    /// `task` as the router sees it on `attempt`: when the θ the attempt
    /// runs puts the tier's floor above the rung the task would start on, a
    /// `rung` hint names the floor rung, and the ladder starts the task there
    /// (B1, 8124). A pin beats any rung, so a pinned task runs its model
    /// either way.
    ///
    /// The router climbs the task's escalations from the rung that starts it,
    /// so a floor raised after the task climbed lifts its climb too. A cap
    /// stops that lift as it stops a climb (gap-26c055): the hint names the
    /// highest rung up to the floor from which the climb stays at or under
    /// the cap, and there is none when every such rung would carry the task
    /// past it.
    pub(super) fn routed_task<'a>(
        &self,
        task: &'a TaskDef,
        attempt: &AttemptContext,
    ) -> Cow<'a, TaskDef> {
        let bounds = self.tier_bounds(task, attempt.harness_decision());
        let (Some(floor), Some(ladder)) = (bounds.floor, self.routing_ladder()) else {
            return Cow::Borrowed(task);
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let tier = task.tier_class();
        let Some(start) = ladder.start(role, tier, task.hints.rung.as_deref()) else {
            return Cow::Borrowed(task);
        };
        let steps = self
            .gate_retry_context
            .ladder_standing(&attempt.identity().plan_id, &task.id)
            .escalations;
        let hint = (start.index + 1..=floor).rev().find_map(|index| {
            let name = ladder.rung_name(role, index)?;
            let from = ladder.start(role, tier, Some(name))?;
            let lands = ladder.climb(role, from, steps).index;
            bounds.cap.is_none_or(|cap| lands <= cap).then_some(name)
        });
        match hint {
            Some(name) => {
                let mut floored = task.clone();
                floored.hints.rung = Some(name.to_string());
                Cow::Owned(floored)
            }
            None => Cow::Borrowed(task),
        }
    }

    /// Record on `attempt` where `plan` put it on the ladder, `step` rungs
    /// above the task's start rung, and log it, with why the try runs there
    /// ([`Self::try_reason`]). An attempt the ladder did not route (it is
    /// off, or no rung of the task can run) records no rung.
    pub(super) fn record_attempt_ladder(
        &self,
        attempt: &mut AttemptContext,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        plan: &RunnerDispatchPlan,
        step: u32,
    ) {
        let reason = self.try_reason(spec, task, attempt, plan, step);
        self.tries
            .lock()
            .reasons
            .insert(attempt.key.attempt_key(), reason);
        let Some(ladder) = self.routing_ladder() else {
            return;
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let (record, last_chance) = match plan.source {
            ModelChoiceSource::Ladder { rung } | ModelChoiceSource::SelfModel { rung } => {
                // A `rung` hint that names one of the task's rungs replaced
                // its start rung (gap-dbf2a6).
                let hinted = task
                    .hints
                    .rung
                    .as_deref()
                    .is_some_and(|hint| ladder.has_rung(role, hint));
                let reason = if step > 0 && self.self_model_climbed(&attempt.identity().chain_key) {
                    // The self-model climbed after one failure (6131).
                    LadderReason::SelfModel
                } else if step > 0 {
                    LadderReason::Escalated
                } else if matches!(plan.source, ModelChoiceSource::SelfModel { .. }) {
                    // An active self-model chose the start rung (6130).
                    LadderReason::SelfModel
                } else if hinted {
                    LadderReason::Hint
                } else {
                    LadderReason::Start
                };
                let record = AttemptLadder {
                    rung: ladder.rung_name(role, rung).map(str::to_string),
                    index: u32::try_from(rung).ok(),
                    step,
                    reason,
                    exhausted: false,
                    // The cascade router's shadow pick beside the rung (G56).
                    router_pick: router_pick(plan.route_decision.as_ref()),
                };
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    model = %plan.model.slug,
                    rung = record.rung.as_deref().unwrap_or("-"),
                    step,
                    reason = ?reason,
                    "attempt routed on the model ladder"
                );
                // The task's last attempt, with no rung left to climb: M1's
                // cap stops the climb as the top rung does (8124).
                let top = self.climbs_left(task, attempt.harness_decision(), rung, step) == 0;
                let task_key = format!("{}/{}", spec.plan_id, task.id);
                (
                    record,
                    top && self.attempt_in_run(&task_key) >= self.live_max_retries(spec, task),
                )
            }
            ModelChoiceSource::Override | ModelChoiceSource::TaskHint => {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    model = %plan.model.slug,
                    source = ?plan.source,
                    "the model is pinned; the ladder never escalates it"
                );
                let pinned = AttemptLadder {
                    rung: None,
                    index: None,
                    step: 0,
                    reason: LadderReason::Pinned,
                    exhausted: false,
                    router_pick: None,
                };
                (pinned, false)
            }
            ModelChoiceSource::Router
            | ModelChoiceSource::Explore
            | ModelChoiceSource::Fallback { .. }
            | ModelChoiceSource::Default => return,
        };
        attempt.record_ladder(record, last_chance);
    }

    /// Why the attempt keyed `attempt_key` runs where it does, as routing
    /// recorded it ([`Self::try_reason`]); `None` before it is routed.
    pub(super) fn routed_try_reason(&self, attempt_key: &str) -> Option<TryReason> {
        self.tries.lock().reasons.get(attempt_key).copied()
    }

    /// Why `attempt` of `task` runs where `plan` put it, `step` rungs above
    /// the task's start rung (gap-aaaaae): a pin is `pinned`, the task's
    /// first try `start`, a try on a higher step than the task's last try
    /// `climb`, and any other try `retry`. A resumed run has not seen the
    /// task's last try: there a task whose standing climbed and has failed
    /// nothing since climbed just before.
    fn try_reason(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt: &AttemptContext,
        plan: &RunnerDispatchPlan,
        step: u32,
    ) -> TryReason {
        if matches!(
            plan.source,
            ModelChoiceSource::Override | ModelChoiceSource::TaskHint
        ) {
            return TryReason::Pinned;
        }
        let last = self
            .tries
            .lock()
            .last_step
            .insert(format!("{}/{}", spec.plan_id, task.id), step);
        if attempt.key.attempt <= 1 {
            return TryReason::Start;
        }
        let climbed = last.map_or_else(
            || {
                step > 0
                    && self
                        .gate_retry_context
                        .ladder_standing(&spec.plan_id, &task.id)
                        .failures_on_rung
                        == 0
            },
            |last| step > last,
        );
        if climbed {
            TryReason::Climb
        } else {
            TryReason::Retry
        }
    }

    /// Why the attempt `verdict` settles ran where it did, once: the reason
    /// routing recorded ([`Self::try_reason`]), `failover` when failover ran
    /// another model, and for an attempt never routed `start` or `retry` by
    /// its ordinal.
    fn take_try_reason(&self, verdict: &AttemptVerdictRecord) -> TryReason {
        let routed = self
            .tries
            .lock()
            .reasons
            .remove(&verdict.identity.attempt_key);
        if !verdict.executed.failover_chain.is_empty() {
            return TryReason::Failover;
        }
        routed.unwrap_or(if verdict.identity.key().attempt <= 1 {
            TryReason::Start
        } else {
            TryReason::Retry
        })
    }

    /// Tell the hub how `settled`, an attempt of `task`, ended
    /// (gap-4e852d): its verdict and blame, where it ran on the ladder and
    /// why, the task's standing once the try counts (`before` is the
    /// standing it ran under), and what it used.
    pub(super) fn publish_attempt_settled(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
        before: LadderStanding,
    ) {
        let verdict = &settled.verdict;
        let reason = self.take_try_reason(verdict);
        let Some(tui) = &self.tui_bridge else {
            return;
        };
        let after = self
            .gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id);
        let ladder = verdict.ladder.as_ref();
        // A pass clears the standing: it reports the rungs it had climbed.
        let (escalations, failures_on_rung) = if settled.succeeded() {
            (ladder.map_or(before.escalations, |ladder| ladder.step), 0)
        } else {
            (after.escalations, after.failures_on_rung)
        };
        let failed_check = verdict
            .failure_class
            .as_ref()
            .and_then(|class| class.rung.clone())
            .or_else(|| {
                verdict
                    .steps
                    .iter()
                    .find(|step| step.passed == Some(false))
                    .map(|step| step.rung.clone())
            });
        let executed = &verdict.executed;
        let timing = &verdict.timing;
        let cost = &verdict.cost;
        let settlement = roko_core::dashboard_snapshot::AttemptSettlement {
            rung: ladder.and_then(|ladder| ladder.rung.clone()),
            escalations,
            failures_on_rung,
            climbed: after.escalations > before.escalations,
            reason: reason.as_str().to_string(),
            verdict: wire_word(&verdict.outcome),
            blame: wire_word(&verdict.blame),
            failed_check,
            detail: settled.failure_reason.as_deref().map(first_line),
            model: executed
                .model_dispatched
                .clone()
                .or_else(|| executed.model_requested.clone())
                .unwrap_or_default(),
            provider: executed.provider.clone().unwrap_or_default(),
            usage: roko_core::dashboard_snapshot::AttemptTokens {
                input: verdict.usage.tokens_in,
                output: verdict.usage.tokens_out,
                cache_read: verdict.usage.tokens_cache_read,
                reasoning: verdict.usage.tokens_reasoning,
            },
            // What the plan's budget counted: a CLI agent's own figure, else
            // the provider's charge, else the snapshot's price.
            cost_usd: cost.vendor_usd.or(cost.billed_usd).or(cost.api_equiv_usd),
            duration_ms: timing
                .settled_at
                .zip(timing.attempt_started_at)
                .and_then(|(settled, started)| u64::try_from(settled - started).ok()),
            exhausted: ladder.is_some_and(|ladder| ladder.exhausted),
        };
        tui.publish_event(roko_core::DashboardEvent::AttemptSettled {
            plan_id: spec.plan_id.clone(),
            task_id: task.id.clone(),
            attempt: settled.key().attempt,
            settled: Box::new(settlement),
        });
    }

    /// How many rungs `task` can still climb from rung `rung`, `step` rungs
    /// above its start rung: the runnable rungs above it, at most the climbs
    /// [`MAX_ESCALATIONS`] leaves, and none from a rung at or above the cap
    /// of `decision`'s θ (8124).
    fn climbs_left(
        &self,
        task: &TaskDef,
        decision: Option<&HarnessDecision>,
        rung: usize,
        step: u32,
    ) -> u32 {
        let Some(ladder) = self.routing_ladder() else {
            return 0;
        };
        if self.tier_bounds(task, decision).caps(rung) {
            return 0;
        }
        let role = task.role.as_deref().unwrap_or("implementer");
        let above = u32::try_from(ladder.runnable_above(role, rung)).unwrap_or(u32::MAX);
        above.min(MAX_ESCALATIONS.saturating_sub(step))
    }

    /// What one try of `task` may use on the rung `plan` routed `attempt`
    /// to, `step` rungs above its start rung, when its tier gives an attempt
    /// `tier_turns` (gap-3e770b): the rung's turns and spend allowance from
    /// `[routing.ladder]`. `None` for an attempt the ladder did not route (a
    /// pin among them), and on a rung with neither bound, such as the
    /// highest one the task can reach.
    pub(super) fn rung_allowance(
        &self,
        task: &TaskDef,
        attempt: &AttemptContext,
        plan: &RunnerDispatchPlan,
        step: u32,
        tier_turns: u32,
    ) -> Option<RungAllowance> {
        let rung = match plan.source {
            ModelChoiceSource::Ladder { rung } | ModelChoiceSource::SelfModel { rung } => rung,
            _ => return None,
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let name = self.routing_ladder()?.rung_name(role, rung)?;
        let climbs_left = self.climbs_left(task, attempt.harness_decision(), rung, step);
        let ladder = &self.config.routing.ladder;
        let allowance = RungAllowance {
            rung: name.to_string(),
            turns: ladder.try_turns(name, climbs_left, tier_turns),
            usd: ladder.try_usd(name),
        };
        (allowance.turns.is_some() || allowance.usd.is_some()).then_some(allowance)
    }

    /// Count `settled`, an attempt of `task`, toward the task's standing on
    /// the ladder. A pass clears the standing. The agent's failure on the rung
    /// the ladder put it on counts, and the second such failure moves the
    /// task one runnable rung up, unless it already climbed
    /// [`MAX_ESCALATIONS`] rungs or stands on its top rung. Other failures,
    /// pinned attempts, and attempts a failover substitute ran in place of
    /// the rung's model (backlog 1118), change nothing. Returns whether the
    /// attempt exhausted the ladder, which it reports.
    pub(super) fn note_ladder_outcome(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
    ) -> bool {
        if settled.succeeded() {
            self.gate_retry_context
                .clear_ladder(&spec.plan_id, &task.id);
            return false;
        }
        let verdict = &settled.verdict;
        let Some(ladder) = verdict.ladder.as_ref() else {
            return false;
        };
        if verdict.blame != Blame::Agent || ladder.reason == LadderReason::Pinned {
            return false;
        }
        if !verdict.executed.failover_chain.is_empty() {
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = ladder.rung.as_deref().unwrap_or("-"),
                substitute = verdict.executed.model_dispatched.as_deref().unwrap_or("-"),
                "a substitute ran; the rung's standing is unchanged"
            );
            return false;
        }
        let Some(routing) = self.routing_ladder() else {
            return false;
        };
        let role = task.role.as_deref().unwrap_or("implementer");
        let index = ladder.index.and_then(|index| usize::try_from(index).ok());
        // The cap of the θ the attempt ran stops the climb as the top rung
        // does (8124); a task already above a cap M1 lowered keeps its rung.
        let bounds = self.tier_bounds(task, settled.harness.as_deref());
        let mut standing = self
            .gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id);
        standing.failures_on_rung = standing.failures_on_rung.saturating_add(1);
        // M3 (6131): when an active self-model started the chain, its re-forecast may climb
        // after one failure; every other chain keeps the two-failure rule.
        let two_failures = standing.failures_on_rung >= FAILURES_PER_RUNG;
        let step = match index {
            Some(index) if !two_failures => {
                self.self_model_step(spec, task, verdict, index, standing.escalations)
            }
            _ => None,
        };
        let (early, rungs) = match step {
            Some(StepAction::Climb { .. }) => (true, 1),
            Some(StepAction::Skip { .. }) => (true, 2),
            _ => (false, 1),
        };
        // A climb of `rungs` lands at most on the cap: the rung below its
        // target is under the cap (8124), for a one-rung climb and a skip.
        let can_climb = index.is_some_and(|index| {
            !bounds.caps(index + rungs - 1) && routing.runnable_above(role, index) >= rungs
        });
        let within = standing.escalations.saturating_add(rungs as u32) <= MAX_ESCALATIONS;
        if (two_failures || early) && within && can_climb {
            standing = LadderStanding {
                escalations: standing.escalations.saturating_add(rungs as u32),
                failures_on_rung: 0,
            };
            self.note_self_model_climb(&verdict.identity.chain_key, early);
            let reason = if early { "low_forecast" } else { "gate_fail" };
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                rung = ladder.rung.as_deref().unwrap_or("-"),
                escalations = standing.escalations,
                reason,
                two_failure_rule = two_failures,
                self_model_step = ?step,
                "the task climbs the ladder; its next attempt runs higher"
            );
        }
        self.gate_retry_context
            .set_ladder_standing(&spec.plan_id, &task.id, standing);
        if ladder.exhausted {
            self.report_ladder_exhausted(spec, task, verdict, ladder);
        }
        ladder.exhausted
    }

    /// Log that `task` exhausted the ladder on `ladder`'s rung, and tell the
    /// dashboard to split or replan it.
    fn report_ladder_exhausted(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        verdict: &AttemptVerdictRecord,
        ladder: &AttemptLadder,
    ) {
        let rung = ladder.rung.as_deref().unwrap_or("-");
        let model = verdict.executed.model_requested.as_deref().unwrap_or("-");
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            rung,
            model,
            escalations = ladder.step,
            reason = "ladder_exhausted",
            "the task failed on its top ladder rung with no retries left; split or replan it"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.diagnosis(DiagnosisSummary {
                id: format!("ladder_exhausted:{}/{}", spec.plan_id, task.id),
                severity: DiagnosisSeverity::Warn,
                subject: format!("{} exhausted the model ladder", task.id),
                detail: format!(
                    "Task `{}` of plan `{}` failed its checks on rung `{rung}` ({model}), the top \
                     rung it can climb, and has no retries left (ladder_exhausted).",
                    task.id, spec.plan_id
                ),
                suggested_action: Some(SPLIT_OR_REPLAN.into()),
                ..DiagnosisSummary::default()
            });
        }
    }

    /// Tell the hub that `verdict`'s attempt of `task` exhausted the ladder
    /// (gap-22c4b1), after the attempt's `attempt_settled`.
    pub(super) fn publish_ladder_exhausted(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        verdict: &AttemptVerdictRecord,
    ) {
        let (Some(tui), Some(ladder)) = (&self.tui_bridge, verdict.ladder.as_ref()) else {
            return;
        };
        tui.publish_event(roko_core::DashboardEvent::LadderExhausted {
            plan_id: spec.plan_id.clone(),
            task_id: task.id.clone(),
            attempt: verdict.identity.key().attempt,
            rung: ladder.rung.clone(),
            model: verdict.executed.model_requested.clone().unwrap_or_default(),
            escalations: ladder.step,
            suggestion: SPLIT_OR_REPLAN.to_string(),
        });
    }
}

/// `value`'s wire word, as its records spell it (`gate_failed`, `agent`).
fn wire_word(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Most bytes of a failure's first line an `attempt_settled` event carries.
const DETAIL_BYTES: usize = 200;

/// The first line of `reason`, at most [`DETAIL_BYTES`], cut at a character
/// boundary and marked `…` when cut.
fn first_line(reason: &str) -> String {
    let line = reason.lines().next().unwrap_or_default().trim_end();
    if line.len() <= DETAIL_BYTES {
        return line.to_string();
    }
    let mut end = DETAIL_BYTES;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &line[..end])
}

/// The cascade router's shadow pick beside the rung (G56), from the attempt's route row
/// `decision`: its learned proposal, except on a row L-M3 decided, whose learned proposal is
/// the self-model's pick (S03's a^L), not the router's (bug-7dff88). The prediction row and the
/// route row keep the self-model's pick.
fn router_pick(decision: Option<&RoutingDecisionLog>) -> Option<String> {
    let decision = decision?;
    if decision.audit.loop_id.as_deref() == Some(SELF_MODEL_LOOP) {
        return None;
    }
    decision.proposals.learned.clone()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::config::routing::LadderRung;
    use roko_graph::cells::NoopAttemptRecorder;
    use roko_learn::cascade_router::CascadeRouter;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, cli_provider, make_scripted_batch_dispatcher, make_spec, model,
        no_auto_fix, verify_step,
    };
    use crate::state_hub::StateHub;

    const RUN: &str = "ladder-run";
    const CHEAP: &str = "claude-haiku-4-5";
    const TOP: &str = "claude-sonnet-4-6";

    /// Fake Claude CLI that records the model of each call. While
    /// `fail-once` exists beside it, a call removes it and fails as the
    /// provider, before any output.
    const PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
dir=$(dirname -- "$0")
previous=
for arg in "$@"; do
  if [ "$previous" = "--model" ]; then
    printf '%s\n' "$arg" >> "$dir/provider-models"
  fi
  previous=$arg
done
if [ -f "$dir/fail-once" ]; then
  rm "$dir/fail-once"
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"ladder-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-l","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    fn rung(name: &str, model: &str) -> LadderRung {
        LadderRung {
            name: name.to_string(),
            model: model.to_string(),
        }
    }

    /// Batch fixture with a two-rung ladder that starts mechanical tasks on
    /// [`CHEAP`], and an unhinted mechanical task whose verify step always
    /// fails. Helper model calls go to their own provider, so [`PROVIDER`]
    /// records only the attempts.
    async fn ladder_fixture(temp: &tempfile::TempDir) -> (GraphTaskDispatcher, TaskDef) {
        ladder_fixture_with(temp, |_| {}).await
    }

    /// [`ladder_fixture`], with `configure` run last on its config.
    async fn ladder_fixture_with(
        temp: &tempfile::TempDir,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        ladder_fixture_scripted(temp, PROVIDER, configure).await
    }

    /// [`ladder_fixture_with`], with `provider` as the fake Claude CLI.
    async fn ladder_fixture_scripted(
        temp: &tempfile::TempDir,
        provider: &str,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        let helper = temp.path().join("helper.sh");
        std::fs::write(&helper, VERIFY_PROVIDER).expect("write helper provider");
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755))
            .expect("make helper executable");
        let (dispatcher, mut task) = make_scripted_batch_dispatcher(temp, provider, |config| {
            no_auto_fix(config);
            config
                .models
                .insert("cheap-model".to_string(), model("batch-cli", CHEAP, None));
            config.routing.ladder.rungs =
                vec![rung("cheap", "cheap-model"), rung("top", "batch-model")];
            config.providers.insert(
                "helper-cli".to_string(),
                cli_provider(&helper.display().to_string()),
            );
            config.models.insert(
                "helper-model".to_string(),
                model("helper-cli", "helper-slug", None),
            );
            config.routing.fast_task_model = "helper-model".to_string();
            configure(config);
        })
        .await;
        task.model_hint = None;
        task.tier = "mechanical".to_string();
        task.verify = vec![verify_step("test", "false")];
        (dispatcher, task)
    }

    /// The model of each call [`PROVIDER`] recorded, in order.
    fn called_models(temp: &tempfile::TempDir) -> Vec<String> {
        std::fs::read_to_string(temp.path().join("provider-models"))
            .expect("the provider recorded its calls")
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Blame, rung, ladder reason and exhaustion of a verdict line.
    fn place(verdict: &serde_json::Value) -> (&str, &str, &str, bool) {
        let ladder = &verdict["ladder"];
        (
            verdict["blame"].as_str().unwrap_or_default(),
            ladder["rung"].as_str().unwrap_or_default(),
            ladder["reason"].as_str().unwrap_or_default(),
            ladder["exhausted"].as_bool().unwrap_or_default(),
        )
    }

    /// Two agent-blamed failures on a rung move the task one rung up, and a
    /// provider failure does not count. On its top rung the task stays, and
    /// its last failed attempt there exhausts the ladder once. A pinned task
    /// never moves, and a `rung` hint picks the start rung.
    #[tokio::test]
    async fn two_failed_attempts_escalate_one_rung() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let hub = StateHub::new(64);
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(runs_dir.clone()),
                ..GraphFeedbackContext::default()
            })
            .with_tui_bridge(TuiBridge::new(hub.sender()));
        assert_eq!(dispatcher.ladder_min_retries(), 5);
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        let mut pinned = task.clone();
        pinned.id = "T-PIN".to_string();
        pinned.model_hint = Some("cheap-model".to_string());
        let pinned_spec = make_spec(&pinned);
        let mut hinted = task.clone();
        hinted.id = "T-HINT".to_string();
        hinted.hints.rung = Some("top".to_string());
        let mut hinted_spec = make_spec(&hinted);
        hinted_spec.max_retries = 1;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        std::fs::write(temp.path().join("fail-once"), "").expect("fail the first call");
        for _ in 0..=spec.max_retries {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails");
        }
        for _ in 0..3 {
            dispatcher
                .dispatch(&pinned_spec, Vec::new(), &ctx)
                .await
                .expect_err("every pinned attempt fails");
        }
        dispatcher
            .dispatch(&hinted_spec, Vec::new(), &ctx)
            .await
            .expect_err("the hinted attempt fails");
        assert_eq!(
            called_models(&temp),
            [CHEAP, CHEAP, CHEAP, TOP, TOP, CHEAP, CHEAP, CHEAP, TOP]
        );
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);
        assert_eq!(dispatcher.ladder_step(&pinned_spec, &pinned), 0);

        dispatcher.close_run_attempts(RUN);
        let verdicts: Vec<serde_json::Value> =
            std::fs::read_to_string(runs_dir.join(RUN).join("attempts.jsonl"))
                .expect("the run's attempt log")
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|line| line["schema_version"] == "roko.verdict/1")
                .collect();
        let places: Vec<(&str, &str, &str, bool)> = verdicts.iter().map(place).collect();
        assert_eq!(
            places,
            [
                ("infra", "cheap", "start", false),
                ("agent", "cheap", "start", false),
                ("agent", "cheap", "start", false),
                ("agent", "top", "escalated", false),
                ("agent", "top", "escalated", true),
                ("agent", "", "pinned", false),
                ("agent", "", "pinned", false),
                ("agent", "", "pinned", false),
                ("agent", "top", "hint", false),
            ]
        );
        let diagnoses: Vec<String> = hub
            .current_snapshot()
            .diagnoses
            .iter()
            .map(|diagnosis| diagnosis.id.clone())
            .filter(|id| id.starts_with("ladder_exhausted"))
            .collect();
        assert_eq!(diagnoses, ["ladder_exhausted:stream-plan/T-EXP"]);
    }

    /// A settled turn-cap stop of `task` on the cheap rung. `substitute`
    /// names the routed model failover replaced, when a substitute ran.
    fn turn_cap_on_cheap_rung(
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        substitute: Option<&str>,
    ) -> SettledAttempt {
        use roko_learn::telemetry::{AttemptIdentity, AttemptKey, AttemptOutcome};

        let key = AttemptKey::new(RUN, &spec.plan_id, &task.id, 1);
        let identity = AttemptIdentity::new(&key);
        let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::TurnCap, true);
        verdict.ladder = Some(AttemptLadder {
            rung: Some("cheap".to_string()),
            index: Some(0),
            step: 0,
            reason: LadderReason::Start,
            exhausted: false,
            router_pick: None,
        });
        verdict.executed.failover_chain = substitute.map(str::to_string).into_iter().collect();
        SettledAttempt {
            verdict: Arc::new(verdict),
            failure_reason: None,
            reflex_rule: None,
            live_tool_calls: LiveToolCalls::default(),
            harness: None,
        }
    }

    /// backlog 1118: an agent-blamed failure of a failover substitute says
    /// nothing about the rung whose model it replaced, so it leaves the
    /// task's standing alone; the same failures on the rung's own model
    /// climb it.
    #[tokio::test]
    async fn substitute_failure_does_not_count_against_routed_rung() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let spec = make_spec(&task);
        let substituted = turn_cap_on_cheap_rung(&spec, &task, Some("cheap-model"));
        assert_eq!(substituted.verdict.blame, Blame::Agent);

        for _ in 0..FAILURES_PER_RUNG {
            dispatcher.note_ladder_outcome(&spec, &task, &substituted);
        }
        let standing = dispatcher
            .gate_retry_context
            .ladder_standing(&spec.plan_id, &task.id);
        assert_eq!(standing.failures_on_rung, 0);
        assert_eq!(dispatcher.ladder_step(&spec, &task), 0);

        let own = turn_cap_on_cheap_rung(&spec, &task, None);
        for _ in 0..FAILURES_PER_RUNG {
            dispatcher.note_ladder_outcome(&spec, &task, &own);
        }
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);
    }

    /// The streaming path climbs the same ladder.
    #[tokio::test]
    async fn two_failed_attempts_escalate_one_rung_when_streaming() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "ladder".to_string(),
        };
        for _ in 0..3 {
            let (events, _received) =
                tokio::sync::mpsc::channel(streaming_event_channel_capacity());
            dispatcher
                .dispatch_streaming(
                    &spec,
                    Vec::new(),
                    &CellContext::new(),
                    &lease,
                    events,
                    &NoopAttemptRecorder,
                )
                .await
                .expect_err("every attempt fails verification");
        }
        assert_eq!(called_models(&temp), [CHEAP, CHEAP, TOP]);
    }

    /// bug-7dff88: a route row L-M3 decided carries the self-model's pick as
    /// its learned proposal, so the ladder record's `router_pick`, the
    /// cascade router's shadow pick, leaves it out; L-route's row and a row
    /// with no loop keep theirs.
    #[test]
    fn router_pick_does_not_show_the_self_models_pick_on_l_m3_rows() {
        let row = |loop_id: Option<&str>| {
            let mut row = serde_json::json!({
                "task_id": "T1",
                "selected_model": "glm-4.7",
                "candidates": [],
                "proposals": { "default": "gpt-oss-120b", "learned": "glm-4.7" },
            });
            if let Some(loop_id) = loop_id {
                row["loop_id"] = serde_json::Value::from(loop_id);
            }
            serde_json::from_value::<RoutingDecisionLog>(row).expect("a route row")
        };
        let self_model = row(Some(SELF_MODEL_LOOP));
        assert_eq!(self_model.proposals.learned.as_deref(), Some("glm-4.7"));
        assert_eq!(router_pick(Some(&self_model)), None);
        for routed in [row(Some("L-route")), row(None)] {
            assert_eq!(router_pick(Some(&routed)).as_deref(), Some("glm-4.7"));
        }
        assert_eq!(router_pick(None), None);
    }

    /// While the ladder routes, an attempt's verdict names the cascade
    /// router's shadow pick beside its rung (G56): the router's own pick its
    /// route row records. Without a cascade router the verdict names none.
    #[tokio::test]
    async fn ladder_attempt_records_router_pick() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = || GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (plain, task) = ladder_fixture(&temp).await;
        let config = Arc::clone(&plain.config);
        let models = vec![CHEAP.to_string(), TOP.to_string()];
        let router = Arc::new(CascadeRouter::new(models));
        let factory = SharedAgentFactory::new(Arc::clone(&config), None, Some(router), None).await;
        let routed = GraphTaskDispatcher::new(Arc::new(factory), config, temp.path().to_path_buf())
            .with_feedback(feedback());
        let plain = plain.with_feedback(feedback());
        let spec = make_spec(&task);
        for (dispatcher, run) in [(&routed, "routed"), (&plain, "plain")] {
            let ctx = CellContext::new().with_run_id(run.to_string());
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("the attempt fails its verify step");
            dispatcher.close_run_attempts(run);
        }
        let rows = |run: &str, file: &str| -> Vec<serde_json::Value> {
            std::fs::read_to_string(runs_dir.join(run).join(file))
                .expect("the run's log")
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        };
        let verdict = |run: &str| {
            rows(run, "attempts.jsonl")
                .into_iter()
                .find(|row| row["schema_version"] == "roko.verdict/1")
                .expect("the attempt's verdict")
        };

        let routed_verdict = verdict("routed");
        assert_eq!(routed_verdict["ladder"]["reason"], "start");
        let pick = routed_verdict["ladder"]["router_pick"]
            .as_str()
            .expect("the cascade router's shadow pick");
        let decisions = rows("routed", "decisions.jsonl");
        let route = decisions
            .iter()
            .find(|row| row["decision_point"] == "route")
            .expect("the attempt's route row");
        assert_eq!(route["source"], "ladder");
        assert_eq!(route["proposals"]["learned"], pick);

        let plain_verdict = verdict("plain");
        assert_eq!(plain_verdict["ladder"]["reason"], "start");
        assert!(
            plain_verdict["ladder"].get("router_pick").is_none(),
            "{plain_verdict}"
        );
    }

    /// S06 B1 (8124): M1's tier floor `mid` starts a mechanical task on
    /// `mid`, its tier cap `mid` stops a climb from `cheap` at `mid`, and a
    /// pinned task runs its own model under both, below the floor or above
    /// the cap. The `harness_policy` rows say which attempts a pin routed,
    /// and a lower cap is a cost-reducing move, which M1 couples to audits.
    #[tokio::test]
    async fn tier_floor_raises_start_rung_but_pins_win() {
        use roko_core::config::harness_params::{
            HarnessLadders, HarnessParams, Knob, KnobKind, Step,
        };
        use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
        use roko_learn::homeostasis::catalog::catalog_move;
        use roko_learn::homeostasis::controller::Controller;
        use roko_learn::homeostasis::detect::Baseline;
        use roko_learn::homeostasis::policy::ViabilityPolicy;
        use roko_learn::telemetry::report::RunRecords;

        use crate::runtime_feedback::HomeostasisSink;

        const MID: &str = "claude-opus-4-6";
        const POLICY: &str = "policy_version = 1\nholdout = 0.0\n\
            ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
            ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";

        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let (dispatcher, task) = ladder_fixture_with(&temp, |config| {
            config
                .models
                .insert("mid-model".to_string(), model("batch-cli", MID, None));
            config.routing.ladder.rungs = vec![
                rung("cheap", "cheap-model"),
                rung("mid", "mid-model"),
                rung("top", "batch-model"),
            ];
        })
        .await;
        let theta0 = HarnessParams::baseline(&dispatcher.config);
        let ladders = HarnessLadders::from_config(&dispatcher.config);
        let settings = HomeostasisConfig {
            mode: HomeostasisMode::On,
            ..HomeostasisConfig::default()
        };
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let controller = Controller::new(
            &settings,
            policy,
            theta0.clone(),
            ladders.clone(),
            baseline,
            0,
        );
        let sink = HomeostasisSink::new(temp.path(), Some(controller), None);
        // Every chain runs the controller's θ, whatever the day's draws.
        let sink = Arc::new(sink.with_holdout(0.0));
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            homeostasis: Some(Arc::clone(&sink)),
            ..GraphFeedbackContext::default()
        });
        let floor = Knob::TierFloor(TaskTier::Mechanical);
        let cap = Knob::TierCap(TaskTier::Mechanical);
        let floored = theta0
            .step(floor, Step::Up, &ladders)
            .expect("floor cheap to mid");
        let capped = theta0
            .step(cap, Step::Down, &ladders)
            .expect("cap top to mid");
        let both = floored
            .step(cap, Step::Down, &ladders)
            .expect("cap mid on floor mid");
        let named = |id: &str, model_hint: Option<&str>| TaskDef {
            id: id.to_string(),
            model_hint: model_hint.map(str::to_string),
            ..task.clone()
        };
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        // Floor mid: the task starts on mid, above its tier's start rung.
        assert_eq!(sink.handle().swap(floored, "floor"), 1);
        let raised = named("T-FLOOR", None);
        dispatcher
            .dispatch(&make_spec(&raised), Vec::new(), &ctx)
            .await
            .expect_err("the attempt fails its verify step");

        // Cap mid: two failures on cheap climb to mid, and failures on mid
        // climb no further.
        assert_eq!(sink.handle().swap(capped, "cap"), 2);
        let climbing = named("T-CAP", None);
        let mut climbing_spec = make_spec(&climbing);
        climbing_spec.max_retries = 4;
        for _ in 0..5 {
            dispatcher
                .dispatch(&climbing_spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails its verify step");
        }
        assert_eq!(dispatcher.ladder_step(&climbing_spec, &climbing), 1);

        // Both: a pinned task runs its model, below the floor or above the
        // cap.
        assert_eq!(sink.handle().swap(both, "both"), 3);
        for (id, model_key) in [("T-LOW", "cheap-model"), ("T-HIGH", "batch-model")] {
            let pinned = named(id, Some(model_key));
            dispatcher
                .dispatch(&make_spec(&pinned), Vec::new(), &ctx)
                .await
                .expect_err("the pinned attempt fails its verify step");
        }
        assert_eq!(
            called_models(&temp),
            [MID, CHEAP, CHEAP, MID, MID, MID, CHEAP, TOP]
        );

        dispatcher.close_run_attempts(RUN);
        let run = RunRecords::load(&runs_dir.join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let places: Vec<(&str, Option<&str>, LadderReason)> = run
            .verdicts
            .iter()
            .map(|line| {
                let ladder = line.record.ladder.as_ref().expect("a ladder record");
                let task = line.record.identity.task_id.as_str();
                (task, ladder.rung.as_deref(), ladder.reason)
            })
            .collect();
        let (start, climbed) = (LadderReason::Start, LadderReason::Escalated);
        assert_eq!(
            places,
            [
                ("T-FLOOR", Some("mid"), start),
                ("T-CAP", Some("cheap"), start),
                ("T-CAP", Some("cheap"), start),
                ("T-CAP", Some("mid"), climbed),
                ("T-CAP", Some("mid"), climbed),
                ("T-CAP", Some("mid"), climbed),
                ("T-LOW", None, LadderReason::Pinned),
                ("T-HIGH", None, LadderReason::Pinned),
            ]
        );
        let pins: Vec<(&str, bool)> = run
            .harness_decisions
            .iter()
            .map(|line| (line.record.identity.task_id.as_str(), line.record.pinned))
            .collect();
        assert_eq!(
            pins,
            [
                ("T-FLOOR", false),
                ("T-CAP", false),
                ("T-CAP", false),
                ("T-CAP", false),
                ("T-CAP", false),
                ("T-CAP", false),
                ("T-LOW", true),
                ("T-HIGH", true),
            ]
        );
        // A lower cap saves cost, so M1 doubles the audit rate after it.
        let lower = catalog_move(KnobKind::TierCap, Step::Down);
        assert!(lower.is_some_and(|lower| lower.audit_coupled()));
    }

    /// gap-26c055: a floor raised after a task climbed lifts its climb with
    /// it, and a cap stops that lift as it stops a climb. A mechanical task
    /// that climbed from cheap to mid stays on mid when θ raises its floor to
    /// mid and caps it there, where the floor alone would lift it to top.
    #[tokio::test]
    async fn a_floor_raised_after_a_climb_stops_at_the_cap() {
        use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
        use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
        use roko_learn::homeostasis::controller::Controller;
        use roko_learn::homeostasis::detect::Baseline;
        use roko_learn::homeostasis::policy::ViabilityPolicy;

        use crate::runtime_feedback::HomeostasisSink;

        const MID: &str = "claude-opus-4-6";
        const POLICY: &str = "policy_version = 1\nholdout = 0.0\n\
            ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
            ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = ladder_fixture_with(&temp, |config| {
            config
                .models
                .insert("mid-model".to_string(), model("batch-cli", MID, None));
            config.routing.ladder.rungs = vec![
                rung("cheap", "cheap-model"),
                rung("mid", "mid-model"),
                rung("top", "batch-model"),
            ];
        })
        .await;
        let theta0 = HarnessParams::baseline(&dispatcher.config);
        let ladders = HarnessLadders::from_config(&dispatcher.config);
        let settings = HomeostasisConfig {
            mode: HomeostasisMode::On,
            ..HomeostasisConfig::default()
        };
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let controller = Controller::new(
            &settings,
            policy,
            theta0.clone(),
            ladders.clone(),
            baseline,
            0,
        );
        let sink = HomeostasisSink::new(temp.path(), Some(controller), None);
        // Every chain runs the controller's θ, whatever the day's draws.
        let sink = Arc::new(sink.with_holdout(0.0));
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(temp.path().join(".roko/runs")),
            homeostasis: Some(Arc::clone(&sink)),
            ..GraphFeedbackContext::default()
        });
        let mut spec = make_spec(&task);
        spec.max_retries = 3;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        // Under θ₀ two failures on cheap climb the task to mid.
        for _ in 0..2 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails its verify step");
        }
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);

        // θ raises the tier's floor to mid and caps it there.
        let floor = Knob::TierFloor(TaskTier::Mechanical);
        let cap = Knob::TierCap(TaskTier::Mechanical);
        let theta = theta0
            .step(floor, Step::Up, &ladders)
            .and_then(|theta| theta.step(cap, Step::Down, &ladders))
            .expect("floor mid, cap mid");
        assert_eq!(sink.handle().swap(theta, "floor and cap"), 1);
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("the attempt fails its verify step");
        assert_eq!(called_models(&temp), [CHEAP, CHEAP, MID]);
    }

    /// gap-26c055: the ladder's last chance is the attempt the task's live
    /// retry budget ends on, the one M1's live source last handed out (B2),
    /// not its plan-load budget. With four retries at plan load raised to
    /// five, the fifth attempt on the top rung is not the last, and the
    /// sixth exhausts the ladder.
    #[tokio::test]
    async fn last_chance_reads_the_live_retry_budget() {
        use crate::runtime_feedback::HomeostasisSink;

        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let sink = Arc::new(HomeostasisSink::new(temp.path(), None, None));
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            homeostasis: Some(Arc::clone(&sink)),
            ..GraphFeedbackContext::default()
        });
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        sink.set_retry_limits(&spec.plan_id, [(task.id.clone(), 5)]);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for _ in 0..6 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails");
        }

        dispatcher.close_run_attempts(RUN);
        let verdicts: Vec<serde_json::Value> =
            std::fs::read_to_string(runs_dir.join(RUN).join("attempts.jsonl"))
                .expect("the run's attempt log")
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|line| line["schema_version"] == "roko.verdict/1")
                .collect();
        let places: Vec<(&str, &str, &str, bool)> = verdicts.iter().map(place).collect();
        assert_eq!(
            places,
            [
                ("agent", "cheap", "start", false),
                ("agent", "cheap", "start", false),
                ("agent", "top", "escalated", false),
                ("agent", "top", "escalated", false),
                ("agent", "top", "escalated", false),
                ("agent", "top", "escalated", true),
            ]
        );
    }

    /// Fake Claude CLI that records each call's model, `--max-turns` and
    /// `--max-budget-usd` (`-` when not given) and its prompt, and never
    /// finishes: given a spend limit it stops there, otherwise at its turn
    /// cap.
    const BOUNDED_PROVIDER: &str = r#"#!/bin/sh
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/calls"
cat > "$dir/prompt-$n"
previous=
model=-
turns=-
budget=-
for arg in "$@"; do
  case "$previous" in
    --model) model=$arg ;;
    --max-turns) turns=$arg ;;
    --max-budget-usd) budget=$arg ;;
  esac
  previous=$arg
done
printf '%s %s %s\n' "$model" "$turns" "$budget" >> "$dir/provider-calls"
if [ "$budget" != "-" ]; then
  printf '{"type":"result","subtype":"error_max_budget_usd","is_error":true,"num_turns":3,"total_cost_usd":%s,"errors":["Reached maximum budget"]}\n' "$budget"
else
  printf '{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":%s,"total_cost_usd":0.01}\n' "$turns"
fi
exit 1
"#;

    /// Three tries of a focused task (60 turns) on [`BOUNDED_PROVIDER`],
    /// under `configure`: what each call was given, each prompt, and the
    /// outcome, blame, rung and ladder reason of each verdict.
    async fn three_bounded_tries(
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (Vec<String>, Vec<String>, Vec<[String; 4]>) {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let (dispatcher, mut task) =
            ladder_fixture_scripted(&temp, BOUNDED_PROVIDER, configure).await;
        task.tier = "focused".to_string();
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        });
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for _ in 0..3 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("no try finishes");
        }
        let calls: Vec<String> = std::fs::read_to_string(temp.path().join("provider-calls"))
            .expect("the provider recorded its calls")
            .lines()
            .map(str::to_string)
            .collect();
        let prompts = (1..=3)
            .map(|n| {
                std::fs::read_to_string(temp.path().join(format!("prompt-{n}"))).expect("prompt")
            })
            .collect();
        dispatcher.close_run_attempts(RUN);
        let verdicts = std::fs::read_to_string(runs_dir.join(RUN).join("attempts.jsonl"))
            .expect("the run's attempt log")
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["schema_version"] == "roko.verdict/1")
            .map(|verdict| {
                let field = |value: &serde_json::Value| value.as_str().unwrap_or("-").to_string();
                [
                    field(&verdict["outcome"]),
                    field(&verdict["blame"]),
                    field(&verdict["ladder"]["rung"]),
                    field(&verdict["ladder"]["reason"]),
                ]
            })
            .collect();
        (calls, prompts, verdicts)
    }

    /// gap-3e770b: a try on a rung the task can still climb from gets half
    /// the tier's turns. A try that uses them up fails on the rung, its
    /// retry keeps the same bound and resumes the work, and the second such
    /// failure climbs; the top rung gets the tier's cap.
    #[tokio::test]
    async fn a_turn_bounded_try_climbs_the_ladder() {
        let (calls, prompts, verdicts) = three_bounded_tries(|_| {}).await;
        assert_eq!(
            calls,
            [
                format!("{CHEAP} 30 -"),
                format!("{CHEAP} 30 -"),
                format!("{TOP} 60 -"),
            ]
        );
        let turn_cap =
            |rung: &str, reason: &str| ["turn_cap", "agent", rung, reason].map(str::to_string);
        assert_eq!(
            verdicts,
            [
                turn_cap("cheap", "start"),
                turn_cap("cheap", "start"),
                turn_cap("top", "escalated"),
            ]
        );
        assert!(!prompts[0].contains("Resuming"));
        assert!(
            prompts[1].contains("stopped at its 30-turn cap")
                && prompts[1].contains("This attempt has a 30-turn cap."),
            "{}",
            prompts[1]
        );
        assert!(
            prompts[2].contains("This attempt has a 60-turn cap."),
            "{}",
            prompts[2]
        );
    }

    /// gap-3e770b: `[routing.ladder] max_turns` and `max_usd` set a rung's
    /// try allowance. The Claude CLI is handed the rung's spend as its own
    /// limit; a try that stops there fails on the rung like a turn-cap stop,
    /// the next try is told to resume, and the second such failure climbs to
    /// a rung with no allowance of its own.
    #[tokio::test]
    async fn a_spend_bounded_try_climbs_the_ladder() {
        let (calls, prompts, verdicts) = three_bounded_tries(|config| {
            let ladder = &mut config.routing.ladder;
            ladder.max_turns.insert("cheap".to_string(), 9);
            ladder.max_usd.insert("cheap".to_string(), 0.015);
        })
        .await;
        assert_eq!(
            calls,
            [
                format!("{CHEAP} 9 0.015000"),
                format!("{CHEAP} 9 0.015000"),
                format!("{TOP} 60 -"),
            ]
        );
        let place =
            |rung: &str, reason: &str| ["turn_cap", "agent", rung, reason].map(str::to_string);
        assert_eq!(
            verdicts,
            [
                place("cheap", "start"),
                place("cheap", "start"),
                place("top", "escalated"),
            ]
        );
        for prompt in &prompts[1..] {
            assert!(
                prompt.contains("was stopped: the try spent its allowance on rung `cheap`"),
                "{prompt}"
            );
        }
        assert!(!prompts[0].contains("Resuming"));
    }

    /// The events of the types `kinds` that `events` received so far, as
    /// JSON, in order.
    fn published(
        events: &mut tokio::sync::broadcast::Receiver<
            roko_runtime::event_bus::Envelope<roko_core::DashboardEvent>,
        >,
        kinds: &[&str],
    ) -> Vec<serde_json::Value> {
        let mut found = Vec::new();
        loop {
            match events.try_recv() {
                Ok(envelope) => {
                    let event = serde_json::to_value(&envelope.payload).expect("event json");
                    if kinds.contains(&event["type"].as_str().unwrap_or_default()) {
                        found.push(event);
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => return found,
            }
        }
    }

    /// gap-aaaaae: `route_decided` names the rung each try runs on, the rungs
    /// its task climbed, and why it runs there: the first try `start`, a try
    /// on the rung of the last one `retry`, the first try on a higher rung
    /// `climb`, and a pinned model `pinned`, with no rung.
    #[tokio::test]
    async fn route_decided_names_its_rung_step_and_reason() {
        let temp = tempdir().expect("tempdir");
        let hub = StateHub::new(4096);
        let mut events = hub.subscribe_events();
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher.with_tui_bridge(TuiBridge::new(hub.sender()));
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        let mut pinned = task.clone();
        pinned.id = "T-PIN".to_string();
        pinned.model_hint = Some("cheap-model".to_string());
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for _ in 0..4 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails its verify step");
        }
        dispatcher
            .dispatch(&make_spec(&pinned), Vec::new(), &ctx)
            .await
            .expect_err("the pinned attempt fails its verify step");

        let routes: Vec<(String, u64, Option<String>, u64, String)> =
            published(&mut events, &["route_decided"])
                .iter()
                .map(|route| {
                    (
                        route["task_id"].as_str().unwrap_or_default().to_string(),
                        route["attempt"].as_u64().unwrap_or_default(),
                        route["rung"].as_str().map(str::to_string),
                        route["ladder_step"].as_u64().unwrap_or(99),
                        route["ladder_reason"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                    )
                })
                .collect();
        let route = |task: &str, attempt: u64, rung: Option<&str>, step: u64, reason: &str| {
            (
                task.to_string(),
                attempt,
                rung.map(str::to_string),
                step,
                reason.to_string(),
            )
        };
        let id = task.id.as_str();
        assert_eq!(
            routes,
            [
                route(id, 1, Some("cheap"), 0, "start"),
                route(id, 2, Some("cheap"), 0, "retry"),
                route(id, 3, Some("top"), 1, "climb"),
                route(id, 4, Some("top"), 1, "retry"),
                route("T-PIN", 1, None, 0, "pinned"),
            ]
        );
    }

    /// gap-aaaaae: with the self-model on, an attempt's `route_decided`
    /// carries its forecast: the mode it ran in, its action, the model it
    /// would choose, and each candidate rung's model with its P(verified
    /// success) and median cost, but not the 90th-percentile cost.
    #[tokio::test]
    async fn route_decided_carries_the_self_model_forecast() {
        use roko_core::config::self_model::{SelfModelConfig, SelfModelMode};
        use roko_core::pricing_snapshot::PriceSnapshot;
        use roko_learn::self_model::model::SelfModel;

        use crate::graph_task_dispatch::self_model::SelfModelRuntime;

        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let settings = SelfModelConfig {
            mode: SelfModelMode::Shadow,
            ..SelfModelConfig::default()
        };
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let state = roko.join("learn/self-model/state-v1.json");
        let runtime = Arc::new(SelfModelRuntime::new(
            settings,
            state,
            SelfModel::new(&snapshot),
        ));
        let hub = StateHub::new(4096);
        let mut events = hub.subscribe_events();
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher
            .with_feedback(GraphFeedbackContext {
                runs_dir: Some(roko.join("runs")),
                self_model: Some(runtime),
                ..GraphFeedbackContext::default()
            })
            .with_tui_bridge(TuiBridge::new(hub.sender()));
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the attempt fails its verify step");

        let routes = published(&mut events, &["route_decided"]);
        let forecast = &routes[0]["forecast"];
        assert_eq!(forecast["mode"], "shadow", "{forecast}");
        assert!(forecast["action"].as_str().is_some(), "{forecast}");
        let candidates = forecast["candidates"].as_array().expect("candidates");
        assert_eq!(candidates.len(), 2, "one per rung: {forecast}");
        let models: Vec<&str> = candidates
            .iter()
            .filter_map(|candidate| candidate["model"].as_str())
            .collect();
        if let Some(choice) = forecast["would_choose"].as_str() {
            assert!(models.contains(&choice), "{forecast}");
        }
        for candidate in candidates {
            assert!(candidate["p_vs"].as_f64().is_some(), "{forecast}");
            assert!(candidate["cost_q50"].as_f64().is_some(), "{forecast}");
            assert!(candidate.get("cost_q90").is_none(), "{forecast}");
        }
    }

    /// gap-4e852d: every try publishes `attempt_settled` with its verdict,
    /// blame, the check that failed it, what it used, and the standing it
    /// leaves: two failed checks on the cheap rung climb (the second says
    /// so), and the first try on the top rung starts its count there.
    #[tokio::test]
    async fn each_try_publishes_attempt_settled() {
        let temp = tempdir().expect("tempdir");
        let hub = StateHub::new(4096);
        let mut events = hub.subscribe_events();
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher.with_tui_bridge(TuiBridge::new(hub.sender()));
        let mut spec = make_spec(&task);
        spec.max_retries = 4;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for _ in 0..3 {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("every attempt fails its verify step");
        }

        let settled = published(&mut events, &["attempt_settled"]);
        let tries: Vec<serde_json::Value> = settled
            .iter()
            .map(|event| {
                serde_json::json!([
                    event["attempt"],
                    event["rung"],
                    event["model"],
                    event["escalations"],
                    event["failures_on_rung"],
                    event["climbed"],
                    event["reason"],
                    event["verdict"],
                    event["blame"],
                ])
            })
            .collect();
        assert_eq!(
            tries,
            [
                serde_json::json!([
                    1,
                    "cheap",
                    CHEAP,
                    0,
                    1,
                    false,
                    "start",
                    "gate_failed",
                    "agent"
                ]),
                serde_json::json!([
                    2,
                    "cheap",
                    CHEAP,
                    1,
                    0,
                    true,
                    "retry",
                    "gate_failed",
                    "agent"
                ]),
                serde_json::json!([3, "top", TOP, 1, 1, false, "climb", "gate_failed", "agent"]),
            ]
        );
        for event in &settled {
            assert_eq!(event["task_id"], task.id.as_str());
            assert!(
                event["failed_check"]
                    .as_str()
                    .is_some_and(|check| check.contains("test")),
                "{event}"
            );
            assert!(
                event["detail"]
                    .as_str()
                    .is_some_and(|detail| detail.starts_with("verify:")),
                "{event}"
            );
            assert_eq!(event["usage"]["out"], 10, "{event}");
            assert_eq!(event["cost_usd"], 0.01, "{event}");
            assert!(event["duration_ms"].as_u64().is_some(), "{event}");
            assert_eq!(event["exhausted"], false, "{event}");
        }
    }

    /// gap-22c4b1: a task that fails on its top rung with no retries left
    /// publishes `ladder_exhausted` once, right after that try's
    /// `attempt_settled`; a pinned task never does.
    #[tokio::test]
    async fn ladder_exhausted_is_a_hub_event() {
        let temp = tempdir().expect("tempdir");
        let hub = StateHub::new(4096);
        let mut events = hub.subscribe_events();
        let (dispatcher, task) = ladder_fixture(&temp).await;
        let dispatcher = dispatcher.with_tui_bridge(TuiBridge::new(hub.sender()));
        let mut spec = make_spec(&task);
        spec.max_retries = 2;
        let mut pinned = task.clone();
        pinned.id = "T-PIN".to_string();
        pinned.model_hint = Some("cheap-model".to_string());
        let mut pinned_spec = make_spec(&pinned);
        pinned_spec.max_retries = 2;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for spec in [&spec, &pinned_spec] {
            for _ in 0..3 {
                dispatcher
                    .dispatch(spec, Vec::new(), &ctx)
                    .await
                    .expect_err("every attempt fails its verify step");
            }
        }

        let events = published(&mut events, &["attempt_settled", "ladder_exhausted"]);
        let order: Vec<(String, String, u64)> = events
            .iter()
            .map(|event| {
                (
                    event["type"].as_str().unwrap_or_default().to_string(),
                    event["task_id"].as_str().unwrap_or_default().to_string(),
                    event["attempt"].as_u64().unwrap_or_default(),
                )
            })
            .collect();
        let line =
            |kind: &str, task: &str, attempt: u64| (kind.to_string(), task.to_string(), attempt);
        let id = task.id.as_str();
        assert_eq!(
            order,
            [
                line("attempt_settled", id, 1),
                line("attempt_settled", id, 2),
                line("attempt_settled", id, 3),
                line("ladder_exhausted", id, 3),
                line("attempt_settled", "T-PIN", 1),
                line("attempt_settled", "T-PIN", 2),
                line("attempt_settled", "T-PIN", 3),
            ]
        );
        let exhausted = &events[3];
        assert_eq!(exhausted["rung"], "top", "{exhausted}");
        assert_eq!(exhausted["escalations"], 1, "{exhausted}");
        assert_eq!(exhausted["suggestion"], SPLIT_OR_REPLAN, "{exhausted}");
        assert_eq!(events[2]["exhausted"], true, "{}", events[2]);
    }
}
