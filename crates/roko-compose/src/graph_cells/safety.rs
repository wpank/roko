//! Safety enrichment Cell (`compose.safety@1`).
//!
//! Provides safety and capability context as prompt sections. This is a
//! **required** enrichment provider -- absence or error **fails the
//! aggregate** rather than degrading gracefully.
//!
//! This Cell does NOT duplicate the E34 safety guards (#243). It only
//! provides the safety-related prompt context (capability declarations,
//! restriction notices, corrigibility reminders) for injection into the
//! composed system prompt.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::{CacheLayer, Placement, PromptSection, SectionPriority};

use super::signals::{ComposeRequest, ComposeScope, SafetySections, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for safety context retrieval.
///
/// Unlike optional providers, a missing or erroring safety provider
/// causes the aggregate Cell to fail closed.
pub trait SafetyContextProvider: Send + Sync + 'static {
    /// Query safety-related prompt sections for the given context.
    ///
    /// Should include capability declarations, restriction notices, and
    /// corrigibility reminders appropriate for the role.
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider that returns an empty section list.
#[derive(Debug, Clone, Copy)]
pub struct NoopSafetyContextProvider;

impl SafetyContextProvider for NoopSafetyContextProvider {
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

/// Safety enrichment Cell for the compose graph.
pub struct SafetyCell<P: SafetyContextProvider = NoopSafetyContextProvider> {
    provider: P,
}

impl<P: SafetyContextProvider> SafetyCell<P> {
    /// Create a new safety cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for SafetyCell<NoopSafetyContextProvider> {
    fn default() -> Self {
        Self::new(NoopSafetyContextProvider)
    }
}

#[async_trait]
impl<P: SafetyContextProvider> roko_graph::Cell for SafetyCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::SAFETY
    }

    fn cell_name(&self) -> &str {
        "Compose Safety Provider"
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

        // Safety sections get highest priority and are placed at the start.
        for section in &mut sections {
            if section.priority == SectionPriority::default() {
                section.priority = SectionPriority::Critical;
            }
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Role;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Start;
            }
        }

        let payload = SafetySections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Store(format!("safety cell serialization: {e}"))
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
        "SafetyCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    #[tokio::test]
    async fn noop_produces_empty() {
        let cell = SafetyCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: SafetySections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
    }

    #[tokio::test]
    async fn sections_get_critical_priority_and_start_placement() {
        struct TestProvider;
        impl SafetyContextProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new(
                    "safety_notice",
                    "You must not modify safety-critical files.",
                )]
            }
        }

        let cell = SafetyCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: SafetySections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections[0].priority, SectionPriority::Critical);
        assert_eq!(payload.sections[0].placement, Placement::Start);
        assert_eq!(payload.sections[0].cache_layer, CacheLayer::Role);
    }
}
