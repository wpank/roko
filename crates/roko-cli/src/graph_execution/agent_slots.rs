//! Run-wide cap on concurrently executing plan tasks (`[conductor] max_agents`).
//!
//! Each running task owns one agent. When several plans run at once, their
//! per-plan `max_parallel` limits add up; this dispatcher bounds the total
//! for the whole run. A task holds its slot for its agent turn and for the
//! verify steps that settle it.
//!
//! With M1 on, its B5 knob, θ's `max_parallel`, sets the live limit, read on
//! each slot request and clamped to `[1, max_agents]` (8134). A larger limit
//! adds permits at once; a smaller one retires permits as their tasks
//! release them, never by cancelling a held lease. Each change of the limit
//! goes to the run's event log as an `agent_slots.resized` entry.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use roko_core::config::harness_params::HarnessParamsHandle;
use roko_core::error::{Result, RokoError};
use roko_core::{DashboardEvent, Signal};
use roko_graph::CellContext;
use roko_graph::cells::{
    AttemptReconciliation, GraphTaskEvent, TaskDispatchOutcome, TaskDispatchRequest,
    TaskDispatcher, TaskExecutionSpec,
};
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::runner::tui_bridge::TuiBridge;

/// [`TaskDispatcher`] that runs at most `slots` tasks at a time through `inner`.
pub struct AgentSlotDispatcher {
    inner: Arc<dyn TaskDispatcher>,
    slots: Semaphore,
    limit: usize,
    /// θ, whose `max_parallel` sets the live limit (M1's B5, 8134).
    theta: Option<HarnessParamsHandle>,
    /// The run's StateHub, whose event log records each limit change.
    events: Option<TuiBridge>,
    /// The limit the pool runs at now.
    effective: AtomicUsize,
    /// Held permits that are retired, not returned, when released.
    retiring: AtomicUsize,
    /// One resize at a time.
    resizing: parking_lot::Mutex<()>,
}

/// A held agent slot. Released, it goes back to the pool, or is retired
/// while the pool shrinks.
struct Slot<'a> {
    permit: Option<SemaphorePermit<'a>>,
    retiring: &'a AtomicUsize,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        let Some(permit) = self.permit.take() else {
            return;
        };
        let retire = self
            .retiring
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_ok();
        if retire {
            permit.forget();
        }
    }
}

impl AgentSlotDispatcher {
    /// Wrap `inner`, allowing `limit` (at least 1) tasks at a time.
    #[must_use]
    pub fn new(inner: Arc<dyn TaskDispatcher>, limit: usize) -> Self {
        let limit = limit.max(1);
        Self {
            inner,
            slots: Semaphore::new(limit),
            limit,
            theta: None,
            events: None,
            effective: AtomicUsize::new(limit),
            retiring: AtomicUsize::new(0),
            resizing: parking_lot::Mutex::new(()),
        }
    }

    /// Size the pool by `theta`'s `max_parallel` on each slot request, within
    /// `[1, limit]`, and log each change on `events` (M1's B5, 8134).
    #[must_use]
    pub fn with_live_limit(mut self, theta: HarnessParamsHandle, events: TuiBridge) -> Self {
        self.theta = Some(theta);
        self.events = Some(events);
        self
    }

    /// Bring the pool to θ's `max_parallel`: add permits to grow, and retire
    /// free permits, then held ones as they are released, to shrink.
    fn resize(&self, plan_id: &str, task: &str) {
        let Some(theta) = &self.theta else {
            return;
        };
        let wanted = usize::try_from(theta.load().params.max_parallel);
        let target = wanted.unwrap_or(usize::MAX).clamp(1, self.limit);
        let _resizing = self.resizing.lock();
        let effective = self.effective.load(Ordering::SeqCst);
        if target == effective {
            return;
        }
        if target > effective {
            let grow = target - effective;
            let retiring = self
                .retiring
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    Some(left.saturating_sub(grow))
                })
                .unwrap_or(0);
            self.slots.add_permits(grow - retiring.min(grow));
        } else {
            let mut shrink = effective - target;
            while shrink > 0 {
                let Ok(permit) = self.slots.try_acquire() else {
                    break;
                };
                permit.forget();
                shrink -= 1;
            }
            self.retiring.fetch_add(shrink, Ordering::SeqCst);
        }
        self.effective.store(target, Ordering::SeqCst);
        tracing::info!(
            plan_id,
            from = effective,
            to = target,
            "agent slots resized to M1's max_parallel"
        );
        if let Some(events) = &self.events {
            let now = chrono::Utc::now().timestamp_millis();
            events.publish_event(DashboardEvent::EventLogEntry {
                timestamp_ms: u64::try_from(now).unwrap_or_default(),
                event_type: "agent_slots.resized".to_string(),
                plan_id: plan_id.to_string(),
                task_id: task.to_string(),
                message: format!("agent slots {effective} -> {target} (M1's max_parallel)"),
            });
        }
    }

    async fn slot(&self, plan_id: &str, task: &str) -> Result<Slot<'_>> {
        self.resize(plan_id, task);
        if self.slots.available_permits() == 0 {
            tracing::info!(
                plan_id,
                task,
                max_agents = self.effective.load(Ordering::SeqCst),
                "waiting for an agent slot ([conductor] max_agents)"
            );
        }
        let permit = self.slots.acquire().await.map_err(|_| RokoError::Agent {
            backend: "agent-slots".to_string(),
            message: "agent slot pool closed".to_string(),
        })?;
        Ok(Slot {
            permit: Some(permit),
            retiring: &self.retiring,
        })
    }
}

#[async_trait::async_trait]
impl TaskDispatcher for AgentSlotDispatcher {
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>> {
        let _slot = self.slot(&spec.plan_id, &spec.title).await?;
        self.inner.dispatch(spec, input, ctx).await
    }

    async fn dispatch_request(
        &self,
        request: &TaskDispatchRequest,
        ctx: &CellContext,
    ) -> Result<TaskDispatchOutcome> {
        let _slot = self
            .slot(&request.spec.plan_id, &request.spec.title)
            .await?;
        self.inner.dispatch_request(request, ctx).await
    }

    async fn dispatch_stream(
        &self,
        request: &TaskDispatchRequest,
        ctx: &CellContext,
        event_tx: tokio::sync::mpsc::Sender<GraphTaskEvent>,
    ) -> Result<TaskDispatchOutcome> {
        let _slot = self
            .slot(&request.spec.plan_id, &request.spec.title)
            .await?;
        self.inner.dispatch_stream(request, ctx, event_tx).await
    }

    async fn reconcile_attempt(
        &self,
        attempt_id: &str,
        request_fingerprint: &str,
    ) -> AttemptReconciliation {
        self.inner
            .reconcile_attempt(attempt_id, request_fingerprint)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Records how many dispatches overlap.
    #[derive(Default)]
    struct OverlapProbe {
        current: AtomicUsize,
        peak: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for OverlapProbe {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            let now = self.current.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.current.fetch_sub(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    fn spec(plan_id: &str) -> TaskExecutionSpec {
        TaskExecutionSpec {
            plan_id: plan_id.to_string(),
            title: "task".to_string(),
            ..TaskExecutionSpec::default()
        }
    }

    #[tokio::test]
    async fn caps_concurrent_tasks_across_plans() {
        let probe = Arc::new(OverlapProbe::default());
        let dispatcher = Arc::new(AgentSlotDispatcher::new(probe.clone(), 2));
        let tasks = ["plan-a", "plan-a", "plan-b", "plan-b", "plan-c"].map(|plan_id| {
            let dispatcher = Arc::clone(&dispatcher);
            tokio::spawn(async move {
                dispatcher
                    .dispatch(&spec(plan_id), Vec::new(), &CellContext::new())
                    .await
            })
        });

        for task in tasks {
            task.await.expect("join").expect("dispatch");
        }

        assert_eq!(probe.peak.load(Ordering::SeqCst), 2);
    }

    /// Holds each dispatch until the test lets one go.
    struct HeldTasks {
        started: AtomicUsize,
        running: AtomicUsize,
        release: Semaphore,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for HeldTasks {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.running.fetch_add(1, Ordering::SeqCst);
            self.release.acquire().await.expect("the gate").forget();
            self.running.fetch_sub(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    /// Let the spawned tasks run up to their next wait.
    async fn settle() {
        tokio::time::sleep(Duration::from_millis(30)).await;
    }

    /// M1's B5 (8134): with four leases held, a live limit of 1 lets all four
    /// finish and admits the next task only once a slot is free; back at 4,
    /// three more start at once.
    #[tokio::test]
    async fn slots_resize_without_dropping_leases() {
        use roko_core::config::harness_params::HarnessParams;
        use roko_core::config::schema::RokoConfig;

        let mut config = RokoConfig::default();
        config.conductor.max_agents = 4;
        let theta = HarnessParams::baseline(&config);
        assert_eq!(theta.max_parallel, 4);
        let handle = HarnessParamsHandle::new(theta.clone());
        let hub = crate::state_hub::shared_state_hub();
        let mut events = hub.subscribe_events();
        let held = Arc::new(HeldTasks {
            started: AtomicUsize::new(0),
            running: AtomicUsize::new(0),
            release: Semaphore::new(0),
        });
        let slots = AgentSlotDispatcher::new(held.clone(), 4)
            .with_live_limit(handle.clone(), TuiBridge::new(hub.sender()));
        let dispatcher = Arc::new(slots);
        let start = || {
            let dispatcher = Arc::clone(&dispatcher);
            tokio::spawn(async move {
                dispatcher
                    .dispatch(&spec("plan-a"), Vec::new(), &CellContext::new())
                    .await
            })
        };
        let mut tasks: Vec<_> = (0..4).map(|_| start()).collect();
        settle().await;
        assert_eq!(held.running.load(Ordering::SeqCst), 4);

        // One slot: the four leases run on, and the fifth task waits until
        // all four have finished.
        let one = HarnessParams {
            max_parallel: 1,
            ..theta.clone()
        };
        handle.swap(one, "b5");
        tasks.push(start());
        settle().await;
        for finished in 1..=3 {
            held.release.add_permits(1);
            settle().await;
            assert_eq!(held.started.load(Ordering::SeqCst), 4, "{finished} done");
        }
        held.release.add_permits(1);
        settle().await;
        assert_eq!(held.started.load(Ordering::SeqCst), 5);
        assert_eq!(held.running.load(Ordering::SeqCst), 1);

        // Four slots again: three more tasks start at once.
        handle.swap(theta, "b5");
        tasks.extend((0..3).map(|_| start()));
        settle().await;
        assert_eq!(held.running.load(Ordering::SeqCst), 4);
        held.release.add_permits(4);
        for task in tasks {
            task.await.expect("join").expect("dispatch");
        }

        // The run's event log has both changes of the limit.
        let mut resized = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let DashboardEvent::EventLogEntry {
                event_type,
                message,
                ..
            } = envelope.payload
                && event_type == "agent_slots.resized"
            {
                resized.push(message);
            }
        }
        assert_eq!(
            resized,
            [
                "agent slots 4 -> 1 (M1's max_parallel)",
                "agent slots 1 -> 4 (M1's max_parallel)",
            ]
        );
    }
}
