//! RAG-19: RetrievalCell — wraps [`UnifiedRetrievalContextBidder`] as a graph node.
//!
//! `RetrievalCell` is a plug-in graph Cell that performs the full unified RAG
//! retrieval pass (knowledge + episodes + code-index, optional reranking) and
//! emits a [`KnowledgeSections`] signal so downstream Cells can consume the
//! retrieved context in the same way they consume `compose.knowledge@1` output.
//!
//! # When to use
//!
//! Insert a `"retrieval"` node into a graph to make retrieved context available
//! to a compose or agent cell without going through the full nine-layer
//! enrichment pipeline.  The cell is self-contained: it takes a
//! [`ComposeRequest`] signal as input and produces a [`KnowledgeSections`]
//! signal as output.
//!
//! # Cell type name
//!
//! Register this cell under the type name `"retrieval"` (see
//! [`register_retrieval_cell`]).

use async_trait::async_trait;
use roko_core::config::RetrievalConfig;
use roko_core::error::{Result, RokoError};
use roko_core::{Body, Kind, ProtocolId, Signal};

use crate::context_provider::{
    ContextBidder, ContextProvider, ContextRequest, ContextTier,
};
use crate::graph_cells::signals::{ComposeRequest, KnowledgeSections};
use crate::prompt::{AttentionBidder, CacheLayer, Placement, SectionPriority};
use crate::unified_retrieval_bidder::UnifiedRetrievalContextBidder;

// ── Cell type constant ────────────────────────────────────────────────────────

/// Cell type name used in graph definitions and the default registry.
pub const RETRIEVAL_CELL_TYPE: &str = "retrieval";

// ── RetrievalCell ─────────────────────────────────────────────────────────────

/// Graph node that wraps [`UnifiedRetrievalContextBidder`] (RAG-09/RAG-19).
///
/// Consumes a [`ComposeRequest`] signal and produces a [`KnowledgeSections`]
/// signal containing the unified retrieval results from all configured sources.
///
/// # Sources
///
/// The cell supports three retrieval sources through builder methods:
/// - Knowledge store (`.with_knowledge(...)`)
/// - Episode store (`.with_episodes(...)`)
/// - Code-index (`.with_code_index(...)`)
///
/// Without any sources the cell degrades to an empty section list.
pub struct RetrievalCell {
    bidder: UnifiedRetrievalContextBidder,
    config: RetrievalConfig,
}

impl RetrievalCell {
    /// Create a new retrieval cell with the given config.
    ///
    /// Use the `with_*` builder methods to attach retrieval sources.
    #[must_use]
    pub fn new(config: RetrievalConfig) -> Self {
        let bidder = UnifiedRetrievalContextBidder::new(config.clone());
        Self { bidder, config }
    }

    /// Create a default retrieval cell (no sources, default config).
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(RetrievalConfig::default())
    }

    /// Attach a knowledge provider.
    #[must_use]
    pub fn with_knowledge<P: crate::graph_cells::knowledge::KnowledgeProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.bidder = self.bidder.with_knowledge(provider);
        self
    }

    /// Attach an episode provider.
    #[must_use]
    pub fn with_episodes<P: crate::graph_cells::episodes::EpisodeProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.bidder = self.bidder.with_episodes(provider);
        self
    }

    /// Attach a code-index provider.
    #[must_use]
    pub fn with_code_index<P: crate::graph_cells::code_index::CodeIndexProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.bidder = self.bidder.with_code_index(provider);
        self
    }

    /// Attach a reranker.
    #[must_use]
    pub fn with_reranker(mut self, reranker: Box<dyn crate::reranker::Reranker>) -> Self {
        self.bidder = self.bidder.with_reranker(reranker);
        self
    }
}

impl Default for RetrievalCell {
    fn default() -> Self {
        Self::default_config()
    }
}

#[async_trait]
impl roko_graph::Cell for RetrievalCell {
    fn cell_id(&self) -> &str {
        RETRIEVAL_CELL_TYPE
    }

    fn cell_name(&self) -> &str {
        "RetrievalCell (RAG-19)"
    }

    fn cell_version(&self) -> roko_graph::CellVersion {
        (1, 0, 0)
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        vec![ProtocolId::Route]
    }

    async fn execute(
        &self,
        input: Vec<Signal>,
        _ctx: &roko_graph::CellContext,
    ) -> Result<Vec<Signal>> {
        let request = extract_compose_request(&input)?;
        let scope = request.scope.clone();

        // RAG-12: determine adaptive retrieval depth from task description.
        let complexity = RetrievalConfig::complexity_for_text(&scope.task_id);
        let depth = self.config.depth_for_task(complexity);

        // Determine effective token budget for this role.
        let role_label = scope.role.to_string();
        let budget = self
            .config
            .effective_token_budget_for_role(&role_label)
            .min(request.token_budget.unwrap_or(usize::MAX));

        // Build a minimal ContextRequest so the bidder can propose candidates.
        // The ContextProvider argument is not used by UnifiedRetrievalContextBidder;
        // it accesses its own typed source fields directly.
        let ctx_request = ContextRequest {
            tier: ContextTier::Focused,
            budget_tokens: budget,
            plan_id: scope.plan_id.clone(),
            task_id: scope.task_id.clone(),
            task_files: Vec::new(),
            task: None,
            plan_artifacts: None,
            siblings: Vec::new(),
            prior_outputs: Vec::new(),
            role_profile: None,
            prompt_policy: None,
        };

        // Construct a stub ContextProvider with an empty workdir.
        // UnifiedRetrievalContextBidder does not call any ContextProvider methods.
        let provider = ContextProvider::new(std::path::PathBuf::new());
        let mut candidates = self.bidder.propose_context(&provider, &ctx_request);

        // Apply adaptive depth cap (RAG-12).
        candidates.truncate(depth);

        // Convert ContextCandidate -> PromptSection.
        let sections: Vec<_> = candidates
            .into_iter()
            .map(|c| {
                let mut section = c.section.section;
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
                section
            })
            .collect();

        let payload = KnowledgeSections::new(scope, sections);
        let body = Body::from_json(&payload).map_err(|e| {
            RokoError::Store(format!("retrieval cell serialization: {e}"))
        })?;
        let signal = Signal::builder(Kind::ContextPack).body(body).build();
        Ok(vec![signal])
    }
}

// ── Register helper ───────────────────────────────────────────────────────────

/// Register `RetrievalCell` in a [`roko_graph::CellRegistry`] under the type
/// name `"retrieval"`.
///
/// The cell is instantiated with default config and no sources attached.
/// Callers that need non-default configuration should construct a
/// [`RetrievalCell`] directly and register it with
/// [`roko_graph::CellRegistry::register_with_descriptor`].
pub fn register_retrieval_cell(registry: &mut roko_graph::CellRegistry) {
    use roko_graph::registry::CellDescriptor;
    use roko_core::{Kind, TypeSchema};

    let descriptor = CellDescriptor::new(
        RETRIEVAL_CELL_TYPE,
        (1, 0, 0),
        Some(TypeSchema::OfKind(Kind::Task)),
        Some(TypeSchema::OfKind(Kind::ContextPack)),
    )
    .with_protocols(vec![ProtocolId::Route])
    .with_display_name("RetrievalCell");

    registry.register_with_descriptor(RETRIEVAL_CELL_TYPE, descriptor, |_config| {
        Box::new(RetrievalCell::default_config())
    });
}

// ── Signal extraction helper ──────────────────────────────────────────────────

fn extract_compose_request(signals: &[Signal]) -> Result<ComposeRequest> {
    for signal in signals {
        if let Body::Json(ref json) = signal.body
            && let Ok(request) = serde_json::from_value::<ComposeRequest>(json.clone())
        {
            return Ok(request);
        }
    }
    Err(RokoError::Store(
        "RetrievalCell: no ComposeRequest in inputs".to_string(),
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;
    use roko_graph::Cell;

    fn make_request_signal(task_id: &str) -> Signal {
        let request = ComposeRequest::new("run-1", "plan-1", task_id, AgentRole::Implementer);
        let body = Body::from_json(&request).expect("serialize");
        Signal::builder(Kind::Task).body(body).build()
    }

    #[tokio::test]
    async fn empty_cell_returns_knowledge_sections_signal() {
        let cell = RetrievalCell::default_config();
        let ctx = roko_graph::CellContext::new();
        let input = vec![make_request_signal("task-1")];
        let output = cell.execute(input, &ctx).await.expect("execute");
        assert_eq!(output.len(), 1, "should produce one KnowledgeSections signal");
        assert_eq!(output[0].kind, Kind::ContextPack);
    }

    #[tokio::test]
    async fn missing_request_signal_returns_error() {
        let cell = RetrievalCell::default_config();
        let ctx = roko_graph::CellContext::new();
        // Pass a non-matching signal.
        let signal = Signal::builder(Kind::AgentMessage).build();
        let result = cell.execute(vec![signal], &ctx).await;
        assert!(result.is_err(), "should fail without a ComposeRequest signal");
    }

    #[test]
    fn cell_id_matches_constant() {
        let cell = RetrievalCell::default_config();
        assert_eq!(cell.cell_id(), RETRIEVAL_CELL_TYPE);
    }

    #[test]
    fn register_adds_cell_to_registry() {
        let mut registry = roko_graph::CellRegistry::new();
        register_retrieval_cell(&mut registry);
        assert!(registry.contains(RETRIEVAL_CELL_TYPE));
    }

    #[test]
    fn default_cell_has_route_protocol() {
        let cell = RetrievalCell::default_config();
        assert!(cell.protocols().contains(&ProtocolId::Route));
    }
}
