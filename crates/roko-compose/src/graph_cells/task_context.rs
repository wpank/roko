//! Task context enrichment Cell (`compose.task_context@1`).
//!
//! Provides dependency-output and task-context sections. This is a
//! **required** enrichment provider -- absence or error **fails the
//! aggregate** rather than degrading gracefully.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::{AttentionBidder, CacheLayer, Placement, PromptSection, SectionPriority};

use super::signals::{ComposeRequest, ComposeScope, TaskContextSections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for task context retrieval.
///
/// Unlike optional providers, a missing or erroring task context provider
/// causes the aggregate Cell to fail closed.
pub trait TaskContextProvider: Send + Sync + 'static {
    /// Query task context sections for the given task/role.
    ///
    /// Must return at least the task brief section. Returning an empty
    /// list is valid but will trigger a warning in the aggregate.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider that returns an empty section list.
///
/// Using this in production will cause the aggregate to emit a warning
/// about empty required context, but will not fail since sections were
/// successfully produced (just empty).
#[derive(Debug, Clone, Copy)]
pub struct NoopTaskContextProvider;

impl TaskContextProvider for NoopTaskContextProvider {
    fn query_sections(
        &self,
        _scope: &ComposeScope,
        _budget_tokens: Option<usize>,
    ) -> Vec<PromptSection> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Cell implementation
// ---------------------------------------------------------------------------

/// Task context enrichment Cell for the compose graph.
pub struct TaskContextCell<P: TaskContextProvider = NoopTaskContextProvider> {
    provider: P,
}

impl<P: TaskContextProvider> TaskContextCell<P> {
    /// Create a new task context cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for TaskContextCell<NoopTaskContextProvider> {
    fn default() -> Self {
        Self::new(NoopTaskContextProvider)
    }
}

#[async_trait]
impl<P: TaskContextProvider> roko_graph::Cell for TaskContextCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::TASK_CONTEXT
    }

    fn cell_name(&self) -> &str {
        "Compose Task Context Provider"
    }

    fn cell_version(&self) -> roko_graph::CellVersion {
        (1, 0, 0)
    }

    async fn execute(
        &self,
        input: Vec<Signal>,
        _ctx: &roko_graph::CellContext,
    ) -> Result<Vec<Signal>> {
        let request = extract_compose_request(&input)?;
        let scope = request.scope.clone();

        let mut sections = self.provider.query_sections(&scope, request.token_budget);

        for section in &mut sections {
            section.bidder = AttentionBidder::TaskContext;
            if section.priority == SectionPriority::default() {
                section.priority = SectionPriority::Critical;
            }
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Plan;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::End;
            }
        }

        let payload = TaskContextSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Store(format!("task_context cell serialization: {e}"))
        })?;
        let signal = Signal::builder(Kind::Task).body(body).build();
        Ok(vec![signal])
    }
}

fn extract_compose_request(input: &[Signal]) -> Result<ComposeRequest> {
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok(req);
        }
    }
    Err(roko_core::error::RokoError::Store(
        "TaskContextCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    #[tokio::test]
    async fn noop_produces_empty() {
        let cell = TaskContextCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: TaskContextSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
    }

    #[tokio::test]
    async fn sections_get_critical_priority() {
        struct TestProvider;
        impl TaskContextProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new("task_brief", "Implement the widget")]
            }
        }

        let cell = TaskContextCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: TaskContextSections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections[0].priority, SectionPriority::Critical);
        assert_eq!(payload.sections[0].placement, Placement::End);
    }
}
