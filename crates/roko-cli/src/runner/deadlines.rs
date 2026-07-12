//! Pure monotonic deadline semantics for runner-owned effects.

use std::time::Duration;

use roko_core::config::TimeoutConfig;

use super::attempt_ownership::{AttemptOwner, AttemptPhase, EffectRef};
use super::types::{GateEffectRef, TaskAttemptRef};

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
    MonotonicTime::from_millis(u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX))
}

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

    pub fn owner_expiry(
        now: MonotonicTime,
        attempt: &TaskAttemptRef,
        owner: &AttemptOwner,
        policy: DeadlinePolicy,
        authored_task_secs: Option<u64>,
        gate_effect: Option<GateEffectRef>,
    ) -> Option<DeadlineExpiry> {
        let timing = owner.timing;
        let task_limit = policy.task_timeout(authored_task_secs);
        let mut expired = Vec::new();
        if now.elapsed_since(timing.attempt_started_at) >= task_limit {
            expired.push((
                timing
                    .attempt_started_at
                    .as_millis()
                    .saturating_add(u64::try_from(task_limit.as_millis()).unwrap_or(u64::MAX)),
                super::types::TimeoutKind::TaskAttempt,
                task_limit,
            ));
        }
        if owner.phase == AttemptPhase::Gate
            && now.elapsed_since(timing.phase_started_at) >= policy.gate_effect
        {
            expired.push((
                timing.phase_started_at.as_millis().saturating_add(
                    u64::try_from(policy.gate_effect.as_millis()).unwrap_or(u64::MAX),
                ),
                super::types::TimeoutKind::GateEffect,
                policy.gate_effect,
            ));
        }
        if owner.phase == AttemptPhase::Agent
            && now.elapsed_since(timing.last_agent_activity_at) >= policy.agent_silence
        {
            expired.push((
                timing.last_agent_activity_at.as_millis().saturating_add(
                    u64::try_from(policy.agent_silence.as_millis()).unwrap_or(u64::MAX),
                ),
                super::types::TimeoutKind::AgentSilence,
                policy.agent_silence,
            ));
        }
        let (deadline_ms, kind, limit) = expired.into_iter().min_by_key(|entry| entry.0)?;
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
                .saturating_add(u64::try_from(limit.as_millis()).unwrap_or(u64::MAX)),
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
            DeadlineTracker::owner_expiry(
                MonotonicTime::from_millis(9),
                &attempt,
                &agent,
                policy(),
                None,
                None,
            )
            .is_none()
        );
        let silence = DeadlineTracker::owner_expiry(
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
            DeadlineTracker::owner_expiry(
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
            DeadlineTracker::owner_expiry(
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
}
