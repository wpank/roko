//! A Graph plan run's time limit, `[timeouts] plan_total_secs` (bug-bfb75a).
//!
//! Every plan run is armed with it unless it is `0`: once the run has gone on
//! that long, it stops through its [`PlanRunInterruptHandle`] as on SIGTERM.
//! The running graph is cancelled, its checkpoint is finalized as
//! `interrupted`, and the run exits 143; `--resume-plan` continues it. Its
//! summary and `--log-file` name the stop [`PlanRunInterrupt::TimeLimit`]
//! (`time-limit`). A FAST run keeps its own, shorter deadline
//! ([`super::fast_lane`]).

use std::time::Duration;

use roko_core::config::RokoConfig;

use super::plan_runner::{PlanRunInterrupt, PlanRunInterruptHandle};
use crate::runner::types::effective_plan_timeout_secs;

/// Stops its run once the run's time limit has passed; dropping it disarms.
#[derive(Debug)]
pub struct RunTimeLimit {
    timer: tokio::task::JoinHandle<()>,
}

impl Drop for RunTimeLimit {
    fn drop(&mut self) {
        self.timer.abort();
    }
}

/// Arm `config`'s time limit for `interrupt`'s run, which has been going for
/// `elapsed`; `None` when the limit is `0`.
#[must_use]
pub fn arm(
    interrupt: &PlanRunInterruptHandle,
    config: &RokoConfig,
    elapsed: Duration,
) -> Option<RunTimeLimit> {
    let secs = effective_plan_timeout_secs(config);
    (secs > 0).then(|| arm_after(interrupt, Duration::from_secs(secs), elapsed))
}

/// Stop `interrupt`'s run as [`PlanRunInterrupt::TimeLimit`] once it has run
/// `limit`, `elapsed` of it gone already.
fn arm_after(
    interrupt: &PlanRunInterruptHandle,
    limit: Duration,
    elapsed: Duration,
) -> RunTimeLimit {
    let interrupt = interrupt.clone();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(limit.saturating_sub(elapsed)).await;
        if interrupt.request(PlanRunInterrupt::TimeLimit) {
            tracing::warn!(
                limit_secs = limit.as_secs(),
                "the run's time limit ([timeouts] plan_total_secs) elapsed; stopping the plan \
                 run, which --resume-plan continues"
            );
        }
    });
    RunTimeLimit { timer }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-bfb75a: a run stops at its time limit, counted from its start,
    /// as a time limit its summary and log tell apart from SIGTERM; a limit
    /// of `0` arms nothing, and dropping the limit disarms it.
    #[tokio::test]
    async fn a_run_stops_at_its_time_limit() {
        let interrupt = PlanRunInterruptHandle::default();
        let _limit = arm_after(
            &interrupt,
            Duration::from_millis(400),
            Duration::from_millis(300),
        );
        for _ in 0..200 {
            if interrupt.requested().is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(interrupt.requested(), Some(PlanRunInterrupt::TimeLimit));

        let disarmed = PlanRunInterruptHandle::default();
        drop(arm_after(
            &disarmed,
            Duration::from_millis(50),
            Duration::ZERO,
        ));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(disarmed.requested(), None);

        let config = |toml: &str| RokoConfig::from_toml(toml).expect("roko.toml");
        let off = config("[timeouts]\nplan_total_secs = 0\n");
        assert!(arm(&PlanRunInterruptHandle::default(), &off, Duration::ZERO).is_none());
        let hour = config("[timeouts]\nplan_total_secs = 3600\n");
        let armed = PlanRunInterruptHandle::default();
        assert!(arm(&armed, &hour, Duration::ZERO).is_some());
        assert_eq!(armed.requested(), None);
    }
}
