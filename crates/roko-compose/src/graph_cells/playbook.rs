//! Playbook enrichment Cell (`compose.playbook@1`).
//!
//! Provides playbook, skill, and Dreams context as prompt sections.
//! This is an **optional** enrichment provider.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::{AttentionBidder, CacheLayer, Placement, PromptSection};

use super::signals::{ComposeRequest, ComposeScope, PlaybookSections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for playbook/skill/Dreams retrieval.
pub trait PlaybookProvider: Send + Sync + 'static {
    /// Query relevant playbooks and skills for the given context.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider used when no playbook store is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopPlaybookProvider;

impl PlaybookProvider for NoopPlaybookProvider {
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

/// Playbook enrichment Cell for the compose graph.
pub struct PlaybookCell<P: PlaybookProvider = NoopPlaybookProvider> {
    provider: P,
}

impl<P: PlaybookProvider> PlaybookCell<P> {
    /// Create a new playbook cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for PlaybookCell<NoopPlaybookProvider> {
    fn default() -> Self {
        Self::new(NoopPlaybookProvider)
    }
}

#[async_trait]
impl<P: PlaybookProvider> roko_graph::Cell for PlaybookCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::PLAYBOOK
    }

    fn cell_name(&self) -> &str {
        "Compose Playbook Provider"
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
            section.bidder = AttentionBidder::PlaybookRules;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Plan;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
        }

        let payload = PlaybookSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Internal(format!(
                "playbook cell serialization: {e}"
            ))
        })?;
        let signal = Signal::builder(Kind::Context).body(body).build();
        Ok(vec![signal])
    }
}

fn extract_compose_request(input: &[Signal]) -> Result<ComposeRequest> {
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok(req);
        }
    }
    Err(roko_core::error::RokoError::Internal(
        "PlaybookCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;

    #[tokio::test]
    async fn noop_produces_empty() {
        let cell = PlaybookCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::Context).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: PlaybookSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
    }
}
