//! Knowledge enrichment Cell (`compose.knowledge@1`).
//!
//! Provides scoped memory, knowledge store, and code-index context as prompt
//! sections. This is an **optional** enrichment provider -- absence or error
//! degrades to an empty section list with a visible warning rather than
//! failing the aggregate.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};
use tracing::warn;

use crate::prompt::{
    AttentionBidder, CacheLayer, Placement, PromptSection, SectionPriority,
};

use super::signals::{ComposeRequest, ComposeScope, KnowledgeSections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for knowledge retrieval.
///
/// Implementations are injected from layer 3 (roko-execution) via the
/// registration manifest. `roko-compose` (layer 2) never imports layer-3
/// types directly.
pub trait KnowledgeProvider: Send + Sync + 'static {
    /// Query scoped knowledge for the given task/role context.
    ///
    /// Returns a list of relevant prompt sections. Empty result is valid.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider used when no knowledge store is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopKnowledgeProvider;

impl KnowledgeProvider for NoopKnowledgeProvider {
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

/// Knowledge enrichment Cell for the compose graph.
///
/// Consumes a [`ComposeRequest`] and produces a [`KnowledgeSections`] signal
/// containing scoped knowledge, memory, and code-index context.
pub struct KnowledgeCell<P: KnowledgeProvider = NoopKnowledgeProvider> {
    provider: P,
}

impl<P: KnowledgeProvider> KnowledgeCell<P> {
    /// Create a new knowledge cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for KnowledgeCell<NoopKnowledgeProvider> {
    fn default() -> Self {
        Self::new(NoopKnowledgeProvider)
    }
}

#[async_trait]
impl<P: KnowledgeProvider> roko_graph::Cell for KnowledgeCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::KNOWLEDGE
    }

    fn cell_name(&self) -> &str {
        "Compose Knowledge Provider"
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

        // Tag all sections with knowledge-appropriate metadata.
        for section in &mut sections {
            section.bidder = AttentionBidder::Neuro;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Workspace;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
            if section.priority == SectionPriority::default() {
                section.priority = SectionPriority::Normal;
            }
        }

        let payload = KnowledgeSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Internal(format!(
                "knowledge cell serialization: {e}"
            ))
        })?;
        let signal = Signal::builder(Kind::Context).body(body).build();
        Ok(vec![signal])
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract a [`ComposeRequest`] from the input signals.
fn extract_compose_request(input: &[Signal]) -> Result<ComposeRequest> {
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok(req);
        }
    }
    Err(roko_core::error::RokoError::Internal(
        "KnowledgeCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;

    #[tokio::test]
    async fn noop_provider_produces_empty_sections() {
        let cell = KnowledgeCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::Context).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        let payload: KnowledgeSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
        assert!(payload.warnings.is_empty());
    }

    #[tokio::test]
    async fn custom_provider_sections_are_tagged() {
        struct TestProvider;
        impl KnowledgeProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new("knowledge_fact", "Rust uses ownership")]
            }
        }

        let cell = KnowledgeCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::Context).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: KnowledgeSections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections.len(), 1);
        assert_eq!(payload.sections[0].bidder, AttentionBidder::Neuro);
        assert_eq!(payload.sections[0].cache_layer, CacheLayer::Workspace);
    }

    #[tokio::test]
    async fn missing_request_errors() {
        let cell = KnowledgeCell::default();
        let empty_signal = Signal::builder(Kind::Context)
            .body(Body::text("not a request"))
            .build();
        let result = cell
            .execute(vec![empty_signal], &roko_graph::CellContext::new())
            .await;
        assert!(result.is_err());
    }
}
