//! The operator's pause of a Graph run (G10).
//!
//! Pause holds (decision 1206): `roko plan pause`, the TUI and the other
//! control surfaces set the run's pause flag, which the [`CellContext`] of
//! every task carries. While it is set no attempt starts, a retry included,
//! and the plan-set driver starts no plan; the attempts already running
//! finish. The plan's deadline keeps running.

use std::time::Duration;

use roko_core::error::{Result, RokoError};
use roko_graph::cell::CellContext;

/// How often an attempt the pause holds looks for the resume.
const PAUSE_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Hold the attempt of task `task` of plan `plan_id`, which is about to
/// start, while the run is paused, and return once it is resumed.
///
/// # Errors
///
/// A cancellation, when the run stops while the attempt waits.
pub(super) async fn hold_while_paused(ctx: &CellContext, plan_id: &str, task: &str) -> Result<()> {
    if !ctx.is_paused() {
        return Ok(());
    }
    tracing::info!(plan_id, task, "the run is paused: the task waits for resume");
    while ctx.is_paused() {
        if ctx.is_cancelled() {
            return Err(RokoError::cancelled(format!(
                "the run stopped while task `{task}` of plan `{plan_id}` waited for resume"
            )));
        }
        tokio::time::sleep(PAUSE_POLL_INTERVAL).await;
    }
    tracing::info!(plan_id, task, "the run resumed: the task starts");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    /// A task's context with the run's pause flag, `paused` or not, and its
    /// stop flag, unset.
    fn context(paused: bool) -> (CellContext, Arc<AtomicBool>, Arc<AtomicBool>) {
        let pause = Arc::new(AtomicBool::new(paused));
        let stop = Arc::new(AtomicBool::new(false));
        let ctx = CellContext::new()
            .with_pause_flag(Arc::clone(&pause))
            .with_cancel_flag(Arc::clone(&stop));
        (ctx, pause, stop)
    }

    #[tokio::test]
    async fn an_attempt_starts_at_once_when_the_run_is_not_paused() {
        let (ctx, _pause, _stop) = context(false);
        hold_while_paused(&ctx, "plan-a", "T1")
            .await
            .expect("nothing holds the attempt");
    }

    #[tokio::test]
    async fn a_paused_run_holds_the_attempt_until_resume() {
        let (ctx, pause, _stop) = context(true);
        let held = tokio::spawn(async move {
            hold_while_paused(&ctx, "plan-a", "T1").await
        });

        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(
            !held.is_finished(),
            "the attempt started while the run was paused"
        );

        pause.store(false, Ordering::Release);
        tokio::time::timeout(Duration::from_secs(5), held)
            .await
            .expect("the attempt starts soon after resume")
            .expect("the hold ran to its end")
            .expect("resume lets the attempt start");
    }

    #[tokio::test]
    async fn a_run_stopped_while_paused_starts_no_attempt() {
        let (ctx, _pause, stop) = context(true);
        stop.store(true, Ordering::Release);

        let error = hold_while_paused(&ctx, "plan-a", "T1")
            .await
            .expect_err("a stopped run starts no attempt");

        assert!(matches!(error, RokoError::Cancelled(_)), "{error}");
    }
}
