//! The operator's stop of one running task (gap-c002bb).
//!
//! While an attempt's provider call runs, [`GraphTaskDispatcher::run_watched`]
//! registers it here under its plan and task. A TUI skip that names the task
//! reaches [`OperatorStops::stop`], which ends the call the way the stall
//! watchdog does: the call is dropped, which ends the agent, and the attempt
//! fails as cancelled, so the task is not retried. The engine skips the
//! task's dependants; the plan's other tasks run on.
//!
//! [`GraphTaskDispatcher::run_watched`]: super::GraphTaskDispatcher::run_watched

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

/// The running attempts an operator can stop, by plan and task.
#[derive(Debug, Clone, Default)]
pub struct OperatorStops {
    running: Arc<parking_lot::Mutex<BTreeMap<(String, String), RunningAttempt>>>,
    next_id: Arc<AtomicU64>,
}

#[derive(Debug)]
struct RunningAttempt {
    id: u64,
    stop: CancellationToken,
}

impl OperatorStops {
    /// Register the attempt at `plan_id/task_id` whose provider call is about
    /// to run. The operator can stop it until the returned handle drops.
    pub(crate) fn register(&self, plan_id: &str, task_id: &str) -> OperatorStop {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let stop = CancellationToken::new();
        let key = (plan_id.to_string(), task_id.to_string());
        self.running.lock().insert(
            key.clone(),
            RunningAttempt {
                id,
                stop: stop.clone(),
            },
        );
        OperatorStop {
            stops: self.clone(),
            key,
            id,
            stop,
        }
    }

    /// Stop the running attempt at `plan_id/task_id`. Returns whether one
    /// was running.
    pub fn stop(&self, plan_id: &str, task_id: &str) -> bool {
        let key = (plan_id.to_string(), task_id.to_string());
        let running = self.running.lock();
        let Some(attempt) = running.get(&key) else {
            return false;
        };
        attempt.stop.cancel();
        true
    }
}

/// One registered attempt ([`OperatorStops::register`]). It leaves the
/// registry when dropped.
pub(crate) struct OperatorStop {
    stops: OperatorStops,
    key: (String, String),
    id: u64,
    stop: CancellationToken,
}

impl OperatorStop {
    /// Resolves once the operator stops the attempt.
    pub(crate) async fn stopped(&self) {
        self.stop.cancelled().await;
    }

    /// Whether the operator has stopped the attempt.
    #[cfg(test)]
    pub(crate) fn is_stopped(&self) -> bool {
        self.stop.is_cancelled()
    }
}

impl Drop for OperatorStop {
    fn drop(&mut self) {
        let mut running = self.stops.running.lock();
        if running
            .get(&self.key)
            .is_some_and(|attempt| attempt.id == self.id)
        {
            running.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_reaches_only_the_task_it_names_while_it_runs() {
        let stops = OperatorStops::default();
        let first = stops.register("p1", "T1");
        let sibling = stops.register("p1", "T2");

        assert!(stops.stop("p1", "T1"));
        assert!(first.is_stopped());
        assert!(!sibling.is_stopped());
        assert!(!stops.stop("p2", "T1"), "no such task runs");

        drop(first);
        assert!(!stops.stop("p1", "T1"), "an ended attempt is not running");
        let retry = stops.register("p1", "T1");
        assert!(!retry.is_stopped(), "its next attempt starts afresh");
    }
}
