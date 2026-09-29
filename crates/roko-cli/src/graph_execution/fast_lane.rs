//! FAST lane (`./dev.sh fast`) run deadline for Graph plan runs.
//!
//! With `ROKO_FAST_MODE` truthy, a run gets `ROKO_FAST_PLAN_DEADLINE_SECS`
//! (default 300) of wall-clock time. `dev.sh` sets that deadline to its own
//! hard deadline minus settlement headroom, so the run stops cleanly before
//! the evidence wrapper kills it. At the deadline the run stops through its
//! [`PlanRunInterruptHandle`]: the running graph is cancelled, its checkpoint
//! is finalized as `interrupted`, and the run exits 143, as on SIGTERM.

use std::time::Duration;

use super::plan_runner::{PlanRunInterrupt, PlanRunInterruptHandle};

/// Run deadline when `ROKO_FAST_PLAN_DEADLINE_SECS` is unset or invalid.
const DEFAULT_PLAN_DEADLINE: Duration = Duration::from_secs(300);

/// Stops its run when the FAST deadline elapses; dropping it disarms.
#[derive(Debug)]
pub struct FastPlanDeadline {
    timer: tokio::task::JoinHandle<()>,
}

impl Drop for FastPlanDeadline {
    fn drop(&mut self) {
        self.timer.abort();
    }
}

/// Arm the FAST run deadline for `interrupt`; `None` outside FAST mode.
pub fn arm_plan_deadline(interrupt: &PlanRunInterruptHandle) -> Option<FastPlanDeadline> {
    if !env_flag_enabled("ROKO_FAST_MODE") {
        return None;
    }
    let deadline = plan_deadline(
        std::env::var("ROKO_FAST_PLAN_DEADLINE_SECS")
            .ok()
            .as_deref(),
    );
    let interrupt = interrupt.clone();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(deadline).await;
        if interrupt.request(PlanRunInterrupt::Terminate) {
            tracing::warn!(
                deadline_secs = deadline.as_secs(),
                "FAST run deadline elapsed; stopping the plan run"
            );
        }
    });
    Some(FastPlanDeadline { timer })
}

fn plan_deadline(configured: Option<&str>) -> Duration {
    configured
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map_or(DEFAULT_PLAN_DEADLINE, Duration::from_secs)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_deadline_parses_positive_seconds_only() {
        assert_eq!(plan_deadline(Some("270")), Duration::from_secs(270));
        assert_eq!(plan_deadline(Some(" 45 ")), Duration::from_secs(45));
        assert_eq!(plan_deadline(Some("0")), DEFAULT_PLAN_DEADLINE);
        assert_eq!(plan_deadline(Some("soon")), DEFAULT_PLAN_DEADLINE);
        assert_eq!(plan_deadline(None), DEFAULT_PLAN_DEADLINE);
    }
}
