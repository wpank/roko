//! Run-wide cap on concurrently executing plan tasks (`[conductor] max_agents`).
//!
//! Each running task owns one agent. When several plans run at once, their
//! per-plan `max_parallel` limits add up; this dispatcher bounds the total
//! for the whole run. A task holds its slot for its agent turn and for the
//! verify steps that settle it.

use std::sync::Arc;

use roko_core::Signal;
use roko_core::error::{Result, RokoError};
use roko_graph::CellContext;
use roko_graph::cells::{
    AttemptReconciliation, GraphTaskEvent, TaskDispatchOutcome, TaskDispatchRequest,
    TaskDispatcher, TaskExecutionSpec,
};
use tokio::sync::{Semaphore, SemaphorePermit};

/// [`TaskDispatcher`] that runs at most `slots` tasks at a time through `inner`.
pub struct AgentSlotDispatcher {
    inner: Arc<dyn TaskDispatcher>,
    slots: Semaphore,
    limit: usize,
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
        }
    }

    async fn slot(&self, plan_id: &str, task: &str) -> Result<SemaphorePermit<'_>> {
        if self.slots.available_permits() == 0 {
            tracing::info!(
                plan_id,
                task,
                max_agents = self.limit,
                "waiting for an agent slot ([conductor] max_agents)"
            );
        }
        self.slots.acquire().await.map_err(|_| RokoError::Agent {
            backend: "agent-slots".to_string(),
            message: "agent slot pool closed".to_string(),
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
    use std::sync::atomic::{AtomicUsize, Ordering};
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
}
