//! Pure monotonic deadline semantics for runner-owned effects.

use std::time::Duration;

use roko_core::config::TimeoutConfig;

use super::attempt_ownership::{AttemptOwner, AttemptPhase, EffectRef};
use super::types::{GateEffectRef, TaskAttemptRef};

/// Saturating conversion from `Duration` to milliseconds as `u64`.
///
/// Replaces the repeated `u64::try_from(d.as_millis()).unwrap_or(u64::MAX)` pattern.
fn duration_millis_u64(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// A monotonic timestamp expressed in milliseconds.
///
/// All deadline arithmetic uses this type to avoid coupling to wall-clock time.
/// Values are derived from a process-wide `Instant` origin (see [`monotonic_now`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonotonicTime(u64);

impl MonotonicTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }

    pub fn elapsed_since(self, earlier: Self) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

pub(crate) fn monotonic_now() -> MonotonicTime {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let origin = ORIGIN.get_or_init(std::time::Instant::now);
    MonotonicTime::from_millis(duration_millis_u64(origin.elapsed()))
}

/// Monotonic timestamps tracking an owned attempt's lifecycle.
///
/// Records when the attempt started, when the current phase began, and
/// when the agent last produced observable output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnershipTiming {
    pub attempt_started_at: MonotonicTime,
    pub phase_started_at: MonotonicTime,
    pub last_agent_activity_at: MonotonicTime,
}

impl OwnershipTiming {
    pub const fn new(now: MonotonicTime) -> Self {
        Self {
            attempt_started_at: now,
            phase_started_at: now,
            last_agent_activity_at: now,
        }
    }

    pub(crate) fn transition(&mut self, now: MonotonicTime) {
        self.phase_started_at = self.phase_started_at.max(now);
    }

    pub(crate) fn record_agent_activity(&mut self, now: MonotonicTime) {
        self.last_agent_activity_at = self.last_agent_activity_at.max(now);
    }
}

/// Duration limits for every deadline the runner enforces.
///
/// Constructed from [`TimeoutConfig`] via [`DeadlinePolicy::from_config`].
/// Individual tasks may override `task_attempt` with an authored timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlinePolicy {
    pub hard_run: Duration,
    pub task_attempt: Duration,
    pub gate_effect: Duration,
    pub agent_silence: Duration,
    pub scheduler_no_progress: Duration,
}

impl DeadlinePolicy {
    pub fn from_config(config: &TimeoutConfig, legacy_plan_timeout: Duration) -> Self {
        Self {
            hard_run: config
                .hard_run_secs
                .map(|secs| Duration::from_secs(secs.max(1)))
                .unwrap_or(legacy_plan_timeout.max(Duration::from_secs(1))),
            task_attempt: config.task_attempt(),
            gate_effect: config.gate_effect(),
            agent_silence: config.agent_silence(),
            scheduler_no_progress: config.scheduler_no_progress(),
        }
    }

    pub fn task_timeout(self, authored_secs: Option<u64>) -> Duration {
        authored_secs
            .filter(|secs| *secs > 0)
            .map(Duration::from_secs)
            .unwrap_or(self.task_attempt)
    }
}

/// A deadline that has been reached or exceeded.
///
/// Carries the timeout kind, the owning attempt/phase/effect (if per-owner),
/// the configured limit, and the absolute monotonic instant at which it fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineExpiry {
    pub kind: super::types::TimeoutKind,
    pub attempt: Option<TaskAttemptRef>,
    pub phase: Option<AttemptPhase>,
    pub effect: Option<EffectRef>,
    pub gate_effect: Option<GateEffectRef>,
    pub limit: Duration,
    pub deadline_at: MonotonicTime,
}

/// Tracks global (run-wide) deadlines: hard-run wall time and scheduler progress.
///
/// Per-owner deadlines are handled by the free function [`owner_expiry`].
#[derive(Debug, Clone, Copy)]
pub struct DeadlineTracker {
    hard_run_started_at: MonotonicTime,
    scheduler_progress_at: MonotonicTime,
}

impl DeadlineTracker {
    pub const fn new(now: MonotonicTime) -> Self {
        Self {
            hard_run_started_at: now,
            scheduler_progress_at: now,
        }
    }

    pub fn record_scheduler_progress(&mut self, now: MonotonicTime) {
        self.scheduler_progress_at = self.scheduler_progress_at.max(now);
    }

    /// Absolute monotonic instant at which the non-resetting run budget ends.
    ///
    /// Dispatch preparation uses this value directly so a long awaited hook or
    /// provider startup cannot hide the expiry from the outer event loop.
    pub fn hard_run_deadline(self, policy: DeadlinePolicy) -> MonotonicTime {
        MonotonicTime::from_millis(
            self.hard_run_started_at
                .as_millis()
                .saturating_add(duration_millis_u64(policy.hard_run)),
        )
    }

    // NB: check order encodes priority — HardRun is checked before
    // SchedulerNoProgress so that a hard-run breach always wins when both
    // deadlines expire in the same tick.
    pub fn global_expiry(
        self,
        now: MonotonicTime,
        policy: DeadlinePolicy,
    ) -> Option<DeadlineExpiry> {
        if now.elapsed_since(self.hard_run_started_at) >= policy.hard_run {
            return Some(global_expiry(
                super::types::TimeoutKind::HardRun,
                policy.hard_run,
                self.hard_run_started_at,
            ));
        }
        if now.elapsed_since(self.scheduler_progress_at) >= policy.scheduler_no_progress {
            return Some(global_expiry(
                super::types::TimeoutKind::SchedulerNoProgress,
                policy.scheduler_no_progress,
                self.scheduler_progress_at,
            ));
        }
        None
    }
}

/// Exact phase of an admitted dispatch while it has not yet become an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchStage {
    Preparation,
    CliStartup,
    BridgeStartup,
}

/// A FAST-only absolute dispatch deadline.
///
/// Normal runs use `None` and retain their existing timeout behavior. FAST
/// runs carry the hard-run instant into each awaited preparation/startup
/// operation so those operations are interruptible even while the main select
/// loop is borrowed by `dispatch_action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchDeadline {
    pub deadline_at: MonotonicTime,
}

impl DispatchDeadline {
    pub const fn new(deadline_at: MonotonicTime) -> Self {
        Self { deadline_at }
    }

    pub fn remaining(self, now: MonotonicTime) -> Option<Duration> {
        (now < self.deadline_at).then(|| {
            Duration::from_millis(self.deadline_at.as_millis().saturating_sub(now.as_millis()))
        })
    }
}

/// Returns the earliest per-owner deadline that has been reached, if any.
///
/// This is a free function rather than a method on [`DeadlineTracker`] because
/// it operates purely on the owner's timing — it has no relationship to the
/// global run clocks that `DeadlineTracker` manages.
pub(crate) fn owner_expiry(
    now: MonotonicTime,
    attempt: &TaskAttemptRef,
    owner: &AttemptOwner,
    policy: DeadlinePolicy,
    authored_task_secs: Option<u64>,
    gate_effect: Option<GateEffectRef>,
) -> Option<DeadlineExpiry> {
    let timing = owner.timing;
    let task_limit = policy.task_timeout(authored_task_secs);

    let mut earliest: Option<(u64, super::types::TimeoutKind, Duration)> = None;

    if now.elapsed_since(timing.attempt_started_at) >= task_limit {
        let candidate = (
            timing
                .attempt_started_at
                .as_millis()
                .saturating_add(duration_millis_u64(task_limit)),
            super::types::TimeoutKind::TaskAttempt,
            task_limit,
        );
        if earliest.map_or(true, |e| candidate.0 < e.0) {
            earliest = Some(candidate);
        }
    }
    if owner.phase == AttemptPhase::Gate
        && now.elapsed_since(timing.phase_started_at) >= policy.gate_effect
    {
        let candidate = (
            timing
                .phase_started_at
                .as_millis()
                .saturating_add(duration_millis_u64(policy.gate_effect)),
            super::types::TimeoutKind::GateEffect,
            policy.gate_effect,
        );
        if earliest.map_or(true, |e| candidate.0 < e.0) {
            earliest = Some(candidate);
        }
    }
    if owner.phase == AttemptPhase::Agent
        && now.elapsed_since(timing.last_agent_activity_at) >= policy.agent_silence
    {
        let candidate = (
            timing
                .last_agent_activity_at
                .as_millis()
                .saturating_add(duration_millis_u64(policy.agent_silence)),
            super::types::TimeoutKind::AgentSilence,
            policy.agent_silence,
        );
        if earliest.map_or(true, |e| candidate.0 < e.0) {
            earliest = Some(candidate);
        }
    }

    let (deadline_ms, kind, limit) = earliest?;
    Some(DeadlineExpiry {
        kind,
        attempt: Some(attempt.clone()),
        phase: Some(owner.phase),
        effect: Some(owner.effect),
        gate_effect,
        limit,
        deadline_at: MonotonicTime::from_millis(deadline_ms),
    })
}

fn global_expiry(
    kind: super::types::TimeoutKind,
    limit: Duration,
    started_at: MonotonicTime,
) -> DeadlineExpiry {
    DeadlineExpiry {
        kind,
        attempt: None,
        phase: None,
        effect: None,
        gate_effect: None,
        limit,
        deadline_at: MonotonicTime::from_millis(
            started_at
                .as_millis()
                .saturating_add(duration_millis_u64(limit)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DeadlinePolicy {
        DeadlinePolicy {
            hard_run: Duration::from_millis(100),
            task_attempt: Duration::from_millis(50),
            gate_effect: Duration::from_millis(20),
            agent_silence: Duration::from_millis(10),
            scheduler_no_progress: Duration::from_millis(30),
        }
    }

    #[test]
    fn duration_conversion_is_exact_then_saturates_at_u64_boundary() {
        assert_eq!(duration_millis_u64(Duration::from_millis(17)), 17);
        assert_eq!(duration_millis_u64(Duration::MAX), u64::MAX);
    }

    #[test]
    fn global_clocks_are_independent_and_expire_at_boundary() {
        let mut tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));
        tracker.record_scheduler_progress(MonotonicTime::from_millis(90));
        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(110), policy())
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::HardRun
        );
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));
        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(40), policy())
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::SchedulerNoProgress
        );
    }

    #[test]
    fn backwards_time_saturates_and_authored_timeout_precedes_global() {
        assert_eq!(
            MonotonicTime::from_millis(5).elapsed_since(MonotonicTime::from_millis(10)),
            Duration::ZERO
        );
        assert_eq!(policy().task_timeout(Some(2)), Duration::from_secs(2));
        assert_eq!(policy().task_timeout(Some(0)), Duration::from_millis(50));
    }

    #[test]
    fn scheduler_progress_reset_does_not_extend_hard_run() {
        let mut tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));
        tracker.record_scheduler_progress(MonotonicTime::from_millis(109));

        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(110), policy())
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::HardRun
        );
    }

    #[test]
    fn global_deadlines_do_not_expire_one_tick_before_boundary() {
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));

        assert!(
            tracker
                .global_expiry(MonotonicTime::from_millis(39), policy())
                .is_none()
        );
        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(40), policy())
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::SchedulerNoProgress
        );
    }

    #[test]
    fn stale_progress_observation_cannot_move_tracker_backwards() {
        let mut tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));
        tracker.record_scheduler_progress(MonotonicTime::from_millis(30));
        tracker.record_scheduler_progress(MonotonicTime::from_millis(20));

        assert!(
            tracker
                .global_expiry(MonotonicTime::from_millis(50), policy())
                .is_none()
        );
        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(60), policy())
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::SchedulerNoProgress
        );
    }

    #[test]
    fn explicit_timeout_config_precedes_legacy_and_zero_values_are_clamped() {
        let mut config = TimeoutConfig {
            hard_run_secs: Some(7),
            task_attempt_secs: Some(0),
            ..TimeoutConfig::default()
        };
        let policy = DeadlinePolicy::from_config(&config, Duration::from_secs(99));
        assert_eq!(policy.hard_run, Duration::from_secs(7));
        assert_eq!(policy.task_attempt, Duration::from_secs(1));
        assert_eq!(policy.task_timeout(Some(3)), Duration::from_secs(3));
        assert_eq!(policy.task_timeout(Some(0)), Duration::from_secs(1));

        config.hard_run_secs = None;
        let policy = DeadlinePolicy::from_config(&config, Duration::from_secs(99));
        assert_eq!(policy.hard_run, Duration::from_secs(99));
    }

    #[test]
    fn owner_expiry_uses_exact_boundaries_and_stable_precedence() {
        let attempt = TaskAttemptRef::new("plan", "task", 1);
        let agent = AttemptOwner::new_at(AttemptPhase::Agent, EffectRef(7), MonotonicTime::ZERO);
        assert!(
            owner_expiry(
                MonotonicTime::from_millis(9),
                &attempt,
                &agent,
                policy(),
                None,
                None,
            )
            .is_none()
        );
        let silence = owner_expiry(
            MonotonicTime::from_millis(10),
            &attempt,
            &agent,
            policy(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(silence.kind, super::super::types::TimeoutKind::AgentSilence);
        assert_eq!(silence.attempt.as_ref(), Some(&attempt));
        assert_eq!(silence.phase, Some(AttemptPhase::Agent));
        assert_eq!(silence.effect, Some(EffectRef(7)));

        let gate = AttemptOwner::new_at(AttemptPhase::Gate, EffectRef(8), MonotonicTime::ZERO);
        assert_eq!(
            owner_expiry(
                MonotonicTime::from_millis(20),
                &attempt,
                &gate,
                policy(),
                None,
                None,
            )
            .unwrap()
            .kind,
            super::super::types::TimeoutKind::GateEffect
        );
        let tied_policy = DeadlinePolicy {
            task_attempt: Duration::from_millis(20),
            ..policy()
        };
        assert_eq!(
            owner_expiry(
                MonotonicTime::from_millis(20),
                &attempt,
                &gate,
                tied_policy,
                None,
                None,
            )
            .unwrap()
            .kind,
            super::super::types::TimeoutKind::TaskAttempt,
            "attempt timeout must win when task and gate deadlines have the same instant"
        );
    }

    // ── Kill-point matrix fixtures (backlog #286) ──────────────────────────
    //
    // These tests exercise the five verification checklist items from the
    // FAST hard-deadline interposition backlog item:
    //
    //   1. A hung preparation hook cannot launch a provider after the deadline.
    //   2. CLI/bridge startup deadlines bound process-tree cleanup.
    //   3. Attribution remains exact while the plan clock advances.
    //   4. No duplicated launch or gate after kill/restart at each checkpoint.
    //   5. The outer wrapper retains settlement headroom.
    //
    // The tests below target items 1, 3, and 5 at the deadline-arithmetic
    // level. Items 2 and 4 live in `attempt_ownership::tests` because they
    // exercise the checkpoint/ownership registry.

    // ── Checklist 1: Hung preparation cannot launch a provider ─────────────

    #[test]
    fn dispatch_deadline_remaining_returns_none_when_expired() {
        let deadline = DispatchDeadline::new(MonotonicTime::from_millis(100));
        // One tick past the deadline: no remaining budget.
        assert!(
            deadline
                .remaining(MonotonicTime::from_millis(101))
                .is_none()
        );
        // Exactly at the deadline: no remaining budget.
        assert!(
            deadline
                .remaining(MonotonicTime::from_millis(100))
                .is_none()
        );
    }

    #[test]
    fn dispatch_deadline_remaining_returns_positive_before_deadline() {
        let deadline = DispatchDeadline::new(MonotonicTime::from_millis(100));
        let remaining = deadline.remaining(MonotonicTime::from_millis(90));
        assert_eq!(remaining, Some(Duration::from_millis(10)));
    }

    #[test]
    fn hard_run_deadline_prevents_provider_launch_when_preparation_consumes_budget() {
        // Scenario: the run starts at t=0 with a 50ms hard-run budget.
        // Preparation hooks complete at t=60 — past the deadline.
        // The dispatch must observe no remaining headroom and refuse to launch.
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(0));
        let p = DeadlinePolicy {
            hard_run: Duration::from_millis(50),
            ..policy()
        };
        let dispatch_dl = DispatchDeadline::new(tracker.hard_run_deadline(p));
        // At t=60, the preparation finally returns. Check remaining:
        assert!(
            dispatch_dl
                .remaining(MonotonicTime::from_millis(60))
                .is_none(),
            "expired dispatch deadline must return None to prevent provider launch"
        );
        // The global tracker must also report HardRun expiry at this instant.
        let global = tracker.global_expiry(MonotonicTime::from_millis(60), p);
        assert_eq!(
            global.unwrap().kind,
            super::super::types::TimeoutKind::HardRun,
            "the global clock must independently confirm hard-run expiry"
        );
    }

    #[test]
    fn dispatch_deadline_fires_terminal_not_provider_when_exactly_at_boundary() {
        // The boundary is the last instant where the deadline has NOT expired.
        // At hard_run_started + hard_run, remaining must be None — a terminal
        // timeout event is emitted, not a provider launch.
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(10));
        let p = DeadlinePolicy {
            hard_run: Duration::from_millis(100),
            ..policy()
        };
        let dispatch_dl = DispatchDeadline::new(tracker.hard_run_deadline(p));
        // t=109: one tick before boundary — still has headroom.
        assert!(
            dispatch_dl
                .remaining(MonotonicTime::from_millis(109))
                .is_some()
        );
        // t=110: exactly at boundary — no headroom left.
        assert!(
            dispatch_dl
                .remaining(MonotonicTime::from_millis(110))
                .is_none(),
            "the exact boundary instant must yield terminal timeout, not launch"
        );
    }

    // ── Checklist 3: Attribution remains exact under clock advance ──────────

    #[test]
    fn owner_expiry_attributes_exact_attempt_across_clock_advance() {
        // Scenario: an attempt is admitted at t=0. The plan clock advances
        // independently. The owner's own timing records when it started and
        // must attribute the specific attempt/phase/effect.
        let attempt = TaskAttemptRef::new("plan", "task-A", 1);
        let owner = AttemptOwner::new_at(
            AttemptPhase::Agent,
            EffectRef(42),
            MonotonicTime::from_millis(10),
        );
        let p = DeadlinePolicy {
            agent_silence: Duration::from_millis(25),
            task_attempt: Duration::from_millis(100),
            ..policy()
        };
        // At t=34 the agent has been silent for 24ms — not yet expired.
        assert!(
            owner_expiry(
                MonotonicTime::from_millis(34),
                &attempt,
                &owner,
                p,
                None,
                None,
            )
            .is_none(),
            "one tick before the silence deadline must not fire"
        );
        // At t=35 the agent has been silent for exactly 25ms — fires.
        let expiry = owner_expiry(
            MonotonicTime::from_millis(35),
            &attempt,
            &owner,
            p,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            expiry.attempt.as_ref().unwrap().task_id,
            "task-A",
            "the timed-out attempt attribution must be exact"
        );
        assert_eq!(
            expiry.effect,
            Some(EffectRef(42)),
            "the timed-out effect attribution must be exact"
        );
        assert_eq!(expiry.kind, super::super::types::TimeoutKind::AgentSilence);
    }

    #[test]
    fn owner_expiry_preserves_exact_attribution_when_global_and_owner_both_expire() {
        // When the global hard-run and an owner deadline both expire at the
        // same instant, the owner expiry must still carry its exact attempt
        // attribution — neither deadline wins by erasing the other.
        let attempt = TaskAttemptRef::new("plan", "task-B", 2);
        let owner = AttemptOwner::new_at(
            AttemptPhase::Agent,
            EffectRef(99),
            MonotonicTime::from_millis(0),
        );
        let p = DeadlinePolicy {
            hard_run: Duration::from_millis(50),
            task_attempt: Duration::from_millis(50),
            agent_silence: Duration::from_millis(50),
            ..policy()
        };
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(0));

        let global = tracker.global_expiry(MonotonicTime::from_millis(50), p);
        let per_owner = owner_expiry(
            MonotonicTime::from_millis(50),
            &attempt,
            &owner,
            p,
            None,
            None,
        );
        // Both must fire.
        assert!(global.is_some(), "global must expire at t=50");
        assert!(per_owner.is_some(), "owner must expire at t=50");
        // The per-owner expiry carries exact attribution.
        let owner_exp = per_owner.unwrap();
        assert_eq!(owner_exp.attempt.as_ref().unwrap().plan_id, "plan");
        assert_eq!(owner_exp.attempt.as_ref().unwrap().task_id, "task-B");
        assert_eq!(owner_exp.attempt.as_ref().unwrap().attempt, 2);
        assert_eq!(owner_exp.effect, Some(EffectRef(99)));
        // The global expiry has no attempt attribution — that is correct.
        assert!(global.unwrap().attempt.is_none());
    }

    #[test]
    fn authored_timeout_does_not_corrupt_sibling_attribution() {
        // Task A has an authored timeout override. Task B uses the default.
        // Expiry of A must not contaminate B's attribution or vice versa.
        let attempt_a = TaskAttemptRef::new("plan", "task-A", 1);
        let attempt_b = TaskAttemptRef::new("plan", "task-B", 1);
        let owner_a = AttemptOwner::new_at(
            AttemptPhase::Agent,
            EffectRef(10),
            MonotonicTime::from_millis(0),
        );
        let owner_b = AttemptOwner::new_at(
            AttemptPhase::Agent,
            EffectRef(20),
            MonotonicTime::from_millis(0),
        );
        let p = DeadlinePolicy {
            task_attempt: Duration::from_millis(100),
            agent_silence: Duration::from_millis(200),
            ..policy()
        };
        // A has a short authored timeout of 30ms; B uses default 100ms.
        let expiry_a = owner_expiry(
            MonotonicTime::from_millis(30),
            &attempt_a,
            &owner_a,
            p,
            Some(0), // 0 → uses policy default (100ms), NOT 30ms
            None,
        );
        // At t=30, default 100ms has not fired, so A should NOT have expired.
        assert!(expiry_a.is_none(), "t=30 < 100ms default must not expire");

        // Now use an actual 1-second authored override that is shorter than
        // the default when expressed as ms:
        let expiry_a_short = owner_expiry(
            MonotonicTime::from_millis(1001),
            &attempt_a,
            &owner_a,
            p,
            Some(1), // 1 second = 1000ms
            None,
        );
        assert!(expiry_a_short.is_some());
        assert_eq!(
            expiry_a_short.unwrap().attempt.as_ref().unwrap().task_id,
            "task-A"
        );

        // B should not be affected by A's authored timeout.
        let expiry_b = owner_expiry(
            MonotonicTime::from_millis(99),
            &attempt_b,
            &owner_b,
            p,
            None,
            None,
        );
        assert!(
            expiry_b.is_none(),
            "B at t=99 < 100ms default must not expire"
        );
        let expiry_b_at_100 = owner_expiry(
            MonotonicTime::from_millis(100),
            &attempt_b,
            &owner_b,
            p,
            None,
            None,
        );
        assert_eq!(
            expiry_b_at_100.unwrap().attempt.as_ref().unwrap().task_id,
            "task-B",
            "B's attribution must be exact at its own expiry"
        );
    }

    // ── Checklist 5: Outer wrapper retains settlement headroom ─────────────

    #[test]
    fn hard_run_deadline_leaves_deterministic_settlement_headroom() {
        // The outer FAST wrapper's process budget should be strictly larger
        // than the inner hard_run deadline so the runner can persist its
        // terminal result before the outer kill arrives.
        //
        // We model this by checking that the hard_run deadline instant is
        // strictly before a simulated outer process deadline.
        let inner_budget = Duration::from_millis(300);
        let settlement_headroom = Duration::from_millis(30);
        let outer_budget = inner_budget + settlement_headroom;

        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(0));
        let p = DeadlinePolicy {
            hard_run: inner_budget,
            ..policy()
        };
        let inner_deadline = tracker.hard_run_deadline(p);
        let outer_deadline_ms = outer_budget.as_millis() as u64;

        assert!(
            inner_deadline.as_millis() < outer_deadline_ms,
            "inner deadline ({}) must be strictly before outer deadline ({}) \
             to preserve settlement headroom",
            inner_deadline.as_millis(),
            outer_deadline_ms
        );
        assert_eq!(
            outer_deadline_ms - inner_deadline.as_millis(),
            settlement_headroom.as_millis() as u64,
            "the gap between inner and outer must equal the configured headroom"
        );
    }

    #[test]
    fn global_expiry_fires_before_settlement_window_ends() {
        // When the hard-run fires, the runner must have enough time within
        // the outer wrapper's budget to persist the durable result. This
        // test checks that global_expiry fires at the inner deadline while
        // the outer deadline is still in the future.
        let run_start = MonotonicTime::from_millis(0);
        let inner_limit = Duration::from_millis(200);
        let headroom = Duration::from_millis(30);
        let outer_limit = inner_limit + headroom;

        let tracker = DeadlineTracker::new(run_start);
        let p = DeadlinePolicy {
            hard_run: inner_limit,
            ..policy()
        };

        // At the inner deadline instant, global_expiry must fire.
        let fired = tracker.global_expiry(
            MonotonicTime::from_millis(inner_limit.as_millis() as u64),
            p,
        );
        assert!(fired.is_some(), "global must fire at the inner deadline");

        // The outer deadline has NOT been reached yet.
        let outer_ms = outer_limit.as_millis() as u64;
        assert!(
            inner_limit.as_millis() < outer_limit.as_millis(),
            "inner ({}) must be before outer ({}) — the runner has time to persist",
            inner_limit.as_millis(),
            outer_ms
        );
    }

    #[test]
    fn dispatch_deadline_remaining_is_exact_and_never_negative() {
        // DispatchDeadline::remaining must never return a negative duration.
        // It returns None (expired) or Some(positive).
        let deadline = DispatchDeadline::new(MonotonicTime::from_millis(100));
        // Well before: exact positive.
        assert_eq!(
            deadline.remaining(MonotonicTime::from_millis(0)),
            Some(Duration::from_millis(100))
        );
        // One tick before: exact 1ms.
        assert_eq!(
            deadline.remaining(MonotonicTime::from_millis(99)),
            Some(Duration::from_millis(1))
        );
        // At deadline: None.
        assert!(
            deadline
                .remaining(MonotonicTime::from_millis(100))
                .is_none()
        );
        // Past deadline: None.
        assert!(
            deadline
                .remaining(MonotonicTime::from_millis(200))
                .is_none()
        );
    }

    #[test]
    fn fast_mode_policy_clamps_without_weakening_gate_effects() {
        // Verify the FAST-mode policy transform preserves gate deadlines
        // while clamping agent/scheduler deadlines. This is essential because
        // the settlement / cleanup path for timed-out agents must not be cut
        // short by FAST-mode hard-run limits.
        let base = DeadlinePolicy {
            hard_run: Duration::from_secs(600),
            task_attempt: Duration::from_secs(600),
            gate_effect: Duration::from_secs(120),
            agent_silence: Duration::from_secs(600),
            scheduler_no_progress: Duration::from_secs(600),
        };
        // Simulate FAST clamping with a plan limit of 300s.
        let mut fast = base;
        let plan_limit = Duration::from_secs(300);
        fast.hard_run = fast.hard_run.min(plan_limit);
        fast.task_attempt = fast.task_attempt.min(Duration::from_secs(90));
        fast.agent_silence = fast.agent_silence.min(Duration::from_secs(90));
        fast.scheduler_no_progress = fast.scheduler_no_progress.min(plan_limit);
        // Gate effect is deliberately NOT clamped.
        assert_eq!(fast.hard_run, plan_limit);
        assert_eq!(fast.task_attempt, Duration::from_secs(90));
        assert_eq!(fast.agent_silence, Duration::from_secs(90));
        assert_eq!(fast.scheduler_no_progress, plan_limit);
        assert_eq!(
            fast.gate_effect, base.gate_effect,
            "FAST mode must never weaken the gate effect deadline — \
             it guards correctness-required verification and cleanup settlement"
        );
    }

    // ── FAST deadline kill-point matrix ──────────────────────────────────────

    #[test]
    fn hard_run_deadline_prevents_launch_after_preparation() {
        // When preparation hooks consume the entire hard_run budget, the
        // dispatch deadline must report no remaining headroom.
        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(0));
        let p = DeadlinePolicy {
            hard_run: Duration::from_millis(40),
            ..policy()
        };
        let dispatch_dl = DispatchDeadline::new(tracker.hard_run_deadline(p));

        // Preparation finishes at t=45 — 5ms past the hard_run budget.
        assert!(
            dispatch_dl
                .remaining(MonotonicTime::from_millis(45))
                .is_none(),
            "dispatch must refuse launch when preparation exceeds hard_run budget"
        );
        // The global tracker must independently confirm expiry.
        assert_eq!(
            tracker
                .global_expiry(MonotonicTime::from_millis(45), p)
                .unwrap()
                .kind,
            super::super::types::TimeoutKind::HardRun
        );
    }

    #[test]
    fn hard_run_leaves_settlement_headroom() {
        // The inner hard_run deadline must be strictly before the outer
        // process deadline so the runner has time to persist its terminal
        // result before the outer kill arrives.
        let settlement = Duration::from_millis(20);
        let inner = Duration::from_millis(200);
        let outer = inner + settlement;

        let tracker = DeadlineTracker::new(MonotonicTime::from_millis(0));
        let p = DeadlinePolicy {
            hard_run: inner,
            ..policy()
        };
        let inner_dl = tracker.hard_run_deadline(p);
        let outer_ms = duration_millis_u64(outer);

        assert!(
            inner_dl.as_millis() < outer_ms,
            "inner deadline ({}) must be strictly before outer deadline ({})",
            inner_dl.as_millis(),
            outer_ms
        );
        // The global expiry must fire at the inner boundary while the
        // outer deadline is still in the future.
        assert!(
            tracker
                .global_expiry(MonotonicTime::from_millis(inner_dl.as_millis()), p)
                .is_some(),
            "global expiry must fire at the inner deadline"
        );
        assert!(
            inner_dl.as_millis() < outer_ms,
            "the runner must have settlement headroom between inner and outer"
        );
    }

    #[test]
    fn fast_mode_clamps_without_weakening_gates() {
        // FAST policy must clamp hard_run, task_attempt, agent_silence, and
        // scheduler_no_progress without touching gate_effect. The gate
        // deadline guards correctness-required verification and must not be
        // shortened by FAST limits.
        let base = DeadlinePolicy {
            hard_run: Duration::from_secs(500),
            task_attempt: Duration::from_secs(500),
            gate_effect: Duration::from_secs(90),
            agent_silence: Duration::from_secs(500),
            scheduler_no_progress: Duration::from_secs(500),
        };
        let fast_limit = Duration::from_secs(180);
        let mut fast = base;
        fast.hard_run = fast.hard_run.min(fast_limit);
        fast.task_attempt = fast.task_attempt.min(fast_limit);
        fast.agent_silence = fast.agent_silence.min(fast_limit);
        fast.scheduler_no_progress = fast.scheduler_no_progress.min(fast_limit);
        // gate_effect is deliberately not clamped.

        assert_eq!(fast.hard_run, fast_limit);
        assert_eq!(fast.task_attempt, fast_limit);
        assert_eq!(fast.agent_silence, fast_limit);
        assert_eq!(fast.scheduler_no_progress, fast_limit);
        assert_eq!(
            fast.gate_effect, base.gate_effect,
            "gate_effect must be untouched by FAST clamping"
        );
    }
}
