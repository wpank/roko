//! Generic cross-cut functors that enrich signal-processing operations.

use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use roko_core::Signal;
use serde::{Deserialize, Serialize};

use crate::ComposeError;

/// Result returned by cross-cut enrichment operations.
pub type CrossCutResult<T> = Result<T, ComposeError>;

/// One phase of the seven-step agent loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoopStep {
    /// Gather observations.
    #[default]
    Sense,
    /// Score observations and select a cognitive tier.
    Assess,
    /// Assemble the next decision or prompt.
    Compose,
    /// Execute the selected action.
    Act,
    /// Verify the action's result.
    Verify,
    /// Persist durable outputs.
    Persist,
    /// Apply feedback to future behavior.
    React,
}

/// Lightweight, operation-independent context supplied to every cross-cut.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossCutContext {
    /// Current loop phase.
    pub step: LoopStep,
    /// Active plan identifier.
    pub plan_id: String,
    /// Active task identifier.
    pub task_id: String,
    /// Role of the active agent.
    pub agent_role: String,
}

/// An endofunctor over signal bundles.
#[async_trait]
pub trait CrossCutFunctor<C = CrossCutContext>: Send + Sync + 'static {
    /// Stable cross-cut identity.
    fn name(&self) -> &str;

    /// Enrich inputs before the inner operation runs.
    async fn pre_enrich(&self, input: Vec<Signal>, ctx: &C) -> CrossCutResult<Vec<Signal>>;

    /// Enrich outputs after the inner operation runs.
    async fn post_enrich(&self, output: Vec<Signal>, ctx: &C) -> CrossCutResult<Vec<Signal>>;

    /// Whether callers may omit this functor as an optimization.
    fn should_short_circuit(&self) -> bool;
}

/// Generic operation wrapper applying cross-cuts as nested functors.
pub struct EnrichedCell<C = CrossCutContext> {
    functors: Vec<Arc<dyn CrossCutFunctor<C>>>,
}

impl<C: 'static> EnrichedCell<C> {
    /// Construct a wrapper. The first functor is the outermost wrapper.
    #[must_use]
    pub fn new(functors: Vec<Arc<dyn CrossCutFunctor<C>>>) -> Self {
        Self { functors }
    }

    /// Run `pre -> operation -> post`; post hooks unwind in reverse order.
    pub async fn execute<F, Fut>(
        &self,
        mut input: Vec<Signal>,
        ctx: &C,
        operation: F,
    ) -> CrossCutResult<Vec<Signal>>
    where
        F: FnOnce(Vec<Signal>) -> Fut,
        Fut: Future<Output = CrossCutResult<Vec<Signal>>>,
    {
        for functor in &self.functors {
            input = functor.pre_enrich(input, ctx).await?;
        }
        let mut output = operation(input).await?;
        for functor in self.functors.iter().rev() {
            output = functor.post_enrich(output, ctx).await?;
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use roko_core::{Body, Kind};

    use super::*;

    struct RecordingFunctor(&'static str, Arc<Mutex<Vec<String>>>);

    #[async_trait]
    impl CrossCutFunctor for RecordingFunctor {
        fn name(&self) -> &str {
            self.0
        }

        async fn pre_enrich(
            &self,
            input: Vec<Signal>,
            _ctx: &CrossCutContext,
        ) -> CrossCutResult<Vec<Signal>> {
            self.1.lock().unwrap().push(format!("{}.pre", self.0));
            Ok(input)
        }

        async fn post_enrich(
            &self,
            output: Vec<Signal>,
            _ctx: &CrossCutContext,
        ) -> CrossCutResult<Vec<Signal>> {
            self.1.lock().unwrap().push(format!("{}.post", self.0));
            Ok(output)
        }

        fn should_short_circuit(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn enriched_cell_preserves_outer_wrapper_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let cell = EnrichedCell::new(vec![
            Arc::new(RecordingFunctor("safety", events.clone())),
            Arc::new(RecordingFunctor("memory", events.clone())),
        ]);
        let input = vec![Signal::builder(Kind::Task).body(Body::text("task")).build()];
        let operation_events = events.clone();

        let result = cell
            .execute(input, &CrossCutContext::default(), |signals| async move {
                operation_events
                    .lock()
                    .unwrap()
                    .push("operation".to_string());
                Ok(signals)
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            *events.lock().unwrap(),
            [
                "safety.pre",
                "memory.pre",
                "operation",
                "memory.post",
                "safety.post"
            ]
        );
    }
}
