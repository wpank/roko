//! Modulation enrichment Cell (`compose.modulation@1`).
//!
//! Provides Daimon/cortical/routing modulation context as prompt sections.
//! This is an **optional** enrichment provider.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::{AttentionBidder, CacheLayer, Placement, PromptSection};

use super::signals::{ComposeRequest, ComposeScope, ModulationSections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for Daimon/cortical modulation retrieval.
pub trait ModulationProvider: Send + Sync + 'static {
    /// Query modulation context for the given task/role.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider used when no modulation source is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopModulationProvider;

impl ModulationProvider for NoopModulationProvider {
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

/// Modulation enrichment Cell for the compose graph.
pub struct ModulationCell<P: ModulationProvider = NoopModulationProvider> {
    provider: P,
}

impl<P: ModulationProvider> ModulationCell<P> {
    /// Create a new modulation cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for ModulationCell<NoopModulationProvider> {
    fn default() -> Self {
        Self::new(NoopModulationProvider)
    }
}

#[async_trait]
impl<P: ModulationProvider> roko_graph::Cell for ModulationCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::MODULATION
    }

    fn cell_name(&self) -> &str {
        "Compose Modulation Provider"
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
            section.bidder = AttentionBidder::Daimon;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Volatile;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::End;
            }
        }

        let payload = ModulationSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Store(format!("modulation cell serialization: {e}"))
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
        "ModulationCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    #[tokio::test]
    async fn noop_produces_empty() {
        let cell = ModulationCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: ModulationSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
    }

    #[tokio::test]
    async fn sections_get_daimon_bidder() {
        struct TestProvider;
        impl ModulationProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new("affect", "Focus on correctness")]
            }
        }

        let cell = ModulationCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: ModulationSections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections[0].bidder, AttentionBidder::Daimon);
        assert_eq!(payload.sections[0].cache_layer, CacheLayer::Volatile);
    }
}
