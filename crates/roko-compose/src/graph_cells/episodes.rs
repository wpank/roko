//! Episodes enrichment Cell (`compose.episodes@1`).
//!
//! Provides relevant episode summaries and error-pattern context as prompt
//! sections. This is an **optional** enrichment provider -- absence or error
//! degrades to an empty section list with a visible warning.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::{AttentionBidder, CacheLayer, Placement, PromptSection};

use super::signals::{ComposeRequest, ComposeScope, EpisodeSections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for episode retrieval.
pub trait EpisodeProvider: Send + Sync + 'static {
    /// Query relevant episodes for the given task/role context.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider used when no episode store is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopEpisodeProvider;

impl EpisodeProvider for NoopEpisodeProvider {
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

/// Episodes enrichment Cell for the compose graph.
///
/// Consumes a [`ComposeRequest`] and produces an [`EpisodeSections`] signal
/// containing relevant episode summaries and error patterns.
pub struct EpisodesCell<P: EpisodeProvider = NoopEpisodeProvider> {
    provider: P,
}

impl<P: EpisodeProvider> EpisodesCell<P> {
    /// Create a new episodes cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for EpisodesCell<NoopEpisodeProvider> {
    fn default() -> Self {
        Self::new(NoopEpisodeProvider)
    }
}

#[async_trait]
impl<P: EpisodeProvider> roko_graph::Cell for EpisodesCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::EPISODES
    }

    fn cell_name(&self) -> &str {
        "Compose Episodes Provider"
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
            section.bidder = AttentionBidder::IterationMemory;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Plan;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
        }

        let payload = EpisodeSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Store(format!("episodes cell serialization: {e}"))
        })?;
        let signal = Signal::builder(Kind::ContextPack).body(body).build();
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
        "EpisodesCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    #[tokio::test]
    async fn noop_provider_produces_empty_sections() {
        let cell = EpisodesCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        let payload: EpisodeSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
    }

    #[tokio::test]
    async fn custom_provider_sections_bidder() {
        struct TestProvider;
        impl EpisodeProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new("error_pattern", "E0277 fix: add bound")]
            }
        }

        let cell = EpisodesCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: EpisodeSections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections[0].bidder, AttentionBidder::IterationMemory);
    }
}
