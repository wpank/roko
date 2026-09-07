//! Code index enrichment Cell (`compose.code_index@1`).
//!
//! Provides symbol, file, and structural workspace context from the code
//! intelligence index as typed prompt sections with their own priority and
//! token budget accounting. This is an **optional** enrichment provider --
//! absence or error degrades to an empty section list with a visible warning
//! rather than failing the aggregate.
//!
//! Separating code index context from the generic knowledge Cell ensures that
//! code index results participate in the `PromptComposer`'s section priority
//! and budget system. When the token budget is exceeded, code index sections
//! are dropped before knowledge and playbook sections (lower priority).

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};
use tracing::debug;

use crate::prompt::{AttentionBidder, CacheLayer, Placement, PromptSection, SectionPriority};

use super::signals::{CodeIndexSections, ComposeRequest, ComposeScope, cell_ids};

/// Default per-section hard cap for code index content (tokens).
///
/// Code index results can be large in a multi-crate workspace. This cap
/// prevents a single code index section from consuming the entire budget.
/// The value is intentionally generous -- budget-level dropping handles
/// the case where the total exceeds the token budget.
pub const DEFAULT_CODE_INDEX_HARD_CAP_TOKENS: usize = 4_000;

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for code index retrieval.
///
/// Implementations are injected from layer 3 (roko-execution) via the
/// registration manifest. `roko-compose` (layer 2) never imports layer-3
/// types directly.
pub trait CodeIndexProvider: Send + Sync + 'static {
    /// Query the code index for relevant symbols, files, and structural
    /// context for the given task/role scope.
    ///
    /// Returns a list of prompt sections. Empty result is valid (e.g., no
    /// index available or no relevant results).
    fn query_sections(
        &self,
        scope: &ComposeScope,
        budget_tokens: Option<usize>,
    ) -> Vec<PromptSection>;
}

/// No-op provider used when no code index is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopCodeIndexProvider;

impl CodeIndexProvider for NoopCodeIndexProvider {
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

/// Code index enrichment Cell for the compose graph.
///
/// Consumes a [`ComposeRequest`] and produces a [`CodeIndexSections`] signal
/// containing structural code context (symbols, files, call graphs) from the
/// workspace code intelligence index.
pub struct CodeIndexCell<P: CodeIndexProvider = NoopCodeIndexProvider> {
    provider: P,
}

impl<P: CodeIndexProvider> CodeIndexCell<P> {
    /// Create a new code index cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for CodeIndexCell<NoopCodeIndexProvider> {
    fn default() -> Self {
        Self::new(NoopCodeIndexProvider)
    }
}

#[async_trait]
impl<P: CodeIndexProvider> roko_graph::Cell for CodeIndexCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::CODE_INDEX
    }

    fn cell_name(&self) -> &str {
        "Compose Code Index Provider"
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

        // Tag all sections with code-index-appropriate metadata.
        for section in &mut sections {
            section.bidder = AttentionBidder::CodeIntelligence;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Workspace;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
            if section.priority == SectionPriority::default() {
                // Normal priority: dropped before knowledge (High) and
                // playbook (Normal with PlaybookRules bidder) but after
                // Low-priority sections.
                section.priority = SectionPriority::Normal;
            }
            // Apply per-section hard cap if not already set.
            if section.hard_cap.is_none() {
                section.hard_cap = Some(DEFAULT_CODE_INDEX_HARD_CAP_TOKENS);
            }
        }

        if !sections.is_empty() {
            let total_tokens: usize = sections.iter().map(|s| s.estimated_tokens()).sum();
            debug!(
                section_count = sections.len(),
                total_tokens,
                "code index enrichment produced sections"
            );
        }

        let payload = CodeIndexSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Store(format!("code index cell serialization: {e}"))
        })?;
        let signal = Signal::builder(Kind::ContextPack).body(body).build();
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
    Err(roko_core::error::RokoError::Store(
        "CodeIndexCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    #[tokio::test]
    async fn noop_provider_produces_empty_sections() {
        let cell = CodeIndexCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        let payload: CodeIndexSections = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
        assert!(payload.warnings.is_empty());
    }

    #[tokio::test]
    async fn custom_provider_sections_are_tagged() {
        struct TestProvider;
        impl CodeIndexProvider for TestProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![PromptSection::new(
                    "code_index",
                    "pub fn main() -> Result<()>",
                )]
            }
        }

        let cell = CodeIndexCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: CodeIndexSections = result[0].body.as_json().unwrap();
        assert_eq!(payload.sections.len(), 1);
        assert_eq!(
            payload.sections[0].bidder,
            AttentionBidder::CodeIntelligence
        );
        assert_eq!(payload.sections[0].cache_layer, CacheLayer::Workspace);
        assert_eq!(payload.sections[0].priority, SectionPriority::Normal);
        assert!(
            payload.sections[0].hard_cap.is_some(),
            "hard cap should be set by default"
        );
    }

    #[tokio::test]
    async fn missing_request_errors() {
        let cell = CodeIndexCell::default();
        let empty_signal = Signal::builder(Kind::ContextPack)
            .body(Body::text("not a request"))
            .build();
        let result = cell
            .execute(vec![empty_signal], &roko_graph::CellContext::new())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn preserves_explicit_priority() {
        struct HighPriorityProvider;
        impl CodeIndexProvider for HighPriorityProvider {
            fn query_sections(
                &self,
                _scope: &ComposeScope,
                _budget_tokens: Option<usize>,
            ) -> Vec<PromptSection> {
                vec![
                    PromptSection::new("code_index", "critical symbol")
                        .with_priority(SectionPriority::High),
                ]
            }
        }

        let cell = CodeIndexCell::new(HighPriorityProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::ContextPack).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: CodeIndexSections = result[0].body.as_json().unwrap();
        // Explicit High priority should be preserved (not overwritten to Normal).
        assert_eq!(payload.sections[0].priority, SectionPriority::High);
    }
}
