//! RAG-09: Unified retrieval context bidder.
//!
//! [`UnifiedRetrievalContextBidder`] combines episode, knowledge, and code-index
//! retrieval under one [`ContextBidder`] implementation.  It fans out to each
//! registered source with a per-source token sub-budget, deduplicates results by
//! section name, applies the optional [`Reranker`] (RAG-22), and filters by the
//! `min_score` from [`RetrievalConfig`].
//!
//! # Design
//!
//! The existing [`ContextBidder`] trait is synchronous, so sources are queried
//! sequentially.  Each source receives `budget_per_source` tokens, which is
//! `total_budget / num_active_sources`, guaranteeing no single source can crowd
//! out the others.  After merging the results are sorted by relevance score
//! (descending) and capped at `max_results` candidates.
//!
//! # Enabling reranking
//!
//! ```toml
//! [retrieval]
//! rerank_enabled = true
//! ```
//!
//! When enabled, the merged candidate list is reranked by the configured
//! [`Reranker`] using the task description as the query string.  Reranked
//! scores replace the original relevance values.
//!
//! # Per-role budgets
//!
//! The effective token budget is resolved by
//! [`RetrievalConfig::effective_token_budget_for_role`] when the request
//! carries a role profile, otherwise the global `token_budget` is used.

use std::collections::HashSet;

use roko_core::config::RetrievalConfig;

use crate::context_provider::{
    ContextBidder, ContextCandidate, ContextProvider, ContextRequest, ContextScope,
};
use crate::context_provider::{ContextPurpose, ContextSection, ContextSource};
use crate::graph_cells::code_index::CodeIndexProvider;
use crate::graph_cells::episodes::EpisodeProvider;
use crate::graph_cells::knowledge::KnowledgeProvider;
use crate::graph_cells::signals::ComposeScope;
use crate::prompt::{AttentionBidder, CacheLayer, Placement};
use crate::reranker::Reranker;

// ── Fraction of the global retrieval budget allocated to each source ──────────

/// Share of the total retrieval budget given to the knowledge source.
const KNOWLEDGE_BUDGET_FRACTION: f32 = 0.40;
/// Share of the total retrieval budget given to the episode source.
const EPISODE_BUDGET_FRACTION: f32 = 0.30;
/// Share of the total retrieval budget given to the code-index source.
const CODE_INDEX_BUDGET_FRACTION: f32 = 0.30;

// ── Per-source default relevance scores (used before reranking) ───────────────

/// Default relevance score for knowledge candidates.
const KNOWLEDGE_DEFAULT_RELEVANCE: f32 = 0.72;
/// Default relevance score for episode candidates.
const EPISODE_DEFAULT_RELEVANCE: f32 = 0.68;
/// Default relevance score for code-index candidates.
const CODE_INDEX_DEFAULT_RELEVANCE: f32 = 0.65;

/// Unified retrieval context bidder (RAG-09).
///
/// Wraps up to three retrieval sources and presents a single, deduplicated,
/// optionally reranked list of [`ContextCandidate`]s to the context-selection
/// pipeline.
///
/// # Construction
///
/// Start with [`UnifiedRetrievalContextBidder::new`] then chain
/// `.with_knowledge(...)`, `.with_episodes(...)`, and `.with_code_index(...)`:
///
/// ```rust,ignore
/// let bidder = UnifiedRetrievalContextBidder::new(config)
///     .with_knowledge(my_knowledge_provider)
///     .with_episodes(my_episode_provider)
///     .with_code_index(my_code_index_provider)
///     .with_reranker(reranker_for_config(&config));
/// ```
pub struct UnifiedRetrievalContextBidder {
    config: RetrievalConfig,
    knowledge: Option<Box<dyn KnowledgeProvider>>,
    episodes: Option<Box<dyn EpisodeProvider>>,
    code_index: Option<Box<dyn CodeIndexProvider>>,
    reranker: Option<Box<dyn Reranker>>,
}

impl UnifiedRetrievalContextBidder {
    /// Create a new unified retrieval bidder backed by `config`.
    ///
    /// No sources are registered until you call `.with_knowledge(...)` etc.
    #[must_use]
    pub fn new(config: RetrievalConfig) -> Self {
        Self {
            config,
            knowledge: None,
            episodes: None,
            code_index: None,
            reranker: None,
        }
    }

    /// Attach a knowledge provider.
    #[must_use]
    pub fn with_knowledge<P: KnowledgeProvider + 'static>(mut self, provider: P) -> Self {
        self.knowledge = Some(Box::new(provider));
        self
    }

    /// Attach an episode provider.
    #[must_use]
    pub fn with_episodes<P: EpisodeProvider + 'static>(mut self, provider: P) -> Self {
        self.episodes = Some(Box::new(provider));
        self
    }

    /// Attach a code-index provider.
    ///
    /// The provider is only consulted when `config.enable_code_index` is `true`.
    #[must_use]
    pub fn with_code_index<P: CodeIndexProvider + 'static>(mut self, provider: P) -> Self {
        self.code_index = Some(Box::new(provider));
        self
    }

    /// Attach a reranker (RAG-22).
    ///
    /// When set, the merged candidate list is reranked using the task description
    /// as the query string.  Reranked scores replace original relevance values.
    #[must_use]
    pub fn with_reranker(mut self, reranker: Box<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Compute a per-source token budget from the total request budget and the
    /// number of active sources (those that are non-None and enabled by config).
    fn per_source_budget(&self, total_budget: usize) -> PerSourceBudget {
        let has_knowledge = self.knowledge.is_some();
        let has_episodes = self.episodes.is_some();
        let has_code_index = self.code_index.is_some() && self.config.enable_code_index;

        let n_active = [has_knowledge, has_episodes, has_code_index]
            .iter()
            .filter(|&&b| b)
            .count();

        if n_active == 0 {
            return PerSourceBudget {
                knowledge: 0,
                episodes: 0,
                code_index: 0,
            };
        }

        // When multiple sources are active, split by fixed fractions so that
        // knowledge (40%), episodes (30%), and code-index (30%) each get their
        // share even when fewer than three sources are present.  The leftover
        // from disabled sources is distributed proportionally by renormalising
        // the fractions of the active ones.
        let mut frac_k = if has_knowledge {
            KNOWLEDGE_BUDGET_FRACTION
        } else {
            0.0
        };
        let mut frac_e = if has_episodes {
            EPISODE_BUDGET_FRACTION
        } else {
            0.0
        };
        let mut frac_c = if has_code_index {
            CODE_INDEX_BUDGET_FRACTION
        } else {
            0.0
        };
        let total_frac = frac_k + frac_e + frac_c;
        if total_frac > 0.0 {
            frac_k /= total_frac;
            frac_e /= total_frac;
            frac_c /= total_frac;
        }

        PerSourceBudget {
            knowledge: (total_budget as f32 * frac_k) as usize,
            episodes: (total_budget as f32 * frac_e) as usize,
            code_index: (total_budget as f32 * frac_c) as usize,
        }
    }

    /// Build a minimal [`ComposeScope`] from the context request.
    fn compose_scope(request: &ContextRequest) -> ComposeScope {
        ComposeScope {
            run_id: String::new(),
            plan_id: request.plan_id.clone(),
            task_id: request.task_id.clone(),
            // Default to Implementer when no role is known; the role is used by
            // providers only as a hint, so this is a safe fallback.
            role: roko_core::AgentRole::Implementer,
        }
    }

    /// Derive the query string for reranking from the context request.
    fn query_string(request: &ContextRequest) -> String {
        // Prefer the task description; fall back to task title, then file list.
        if let Some(task) = &request.task {
            if let Some(desc) = &task.description
                && !desc.trim().is_empty()
            {
                return desc.clone();
            }
            if !task.title.trim().is_empty() {
                return task.title.clone();
            }
        }
        if !request.task_files.is_empty() {
            return request.task_files.join(" ");
        }
        request.task_id.clone()
    }

    /// Apply the configured `min_score` filter and `max_results` cap.
    fn filter_and_cap(&self, mut candidates: Vec<ContextCandidate>) -> Vec<ContextCandidate> {
        let min_score = self.config.min_score as f32;
        candidates.retain(|c| c.relevance >= min_score);
        if candidates.len() > self.config.max_results {
            candidates.truncate(self.config.max_results);
        }
        candidates
    }

    // ── Per-source collection helpers ─────────────────────────────────────────

    fn collect_knowledge(
        &self,
        request: &ContextRequest,
        scope: &ComposeScope,
        budget: usize,
        seen_names: &mut HashSet<String>,
        out: &mut Vec<ContextCandidate>,
    ) {
        let Some(provider) = &self.knowledge else {
            return;
        };
        if budget == 0 {
            return;
        }
        let sections = provider.query_sections(scope, Some(budget));
        for mut section in sections {
            if seen_names.contains(&section.name) {
                continue;
            }
            seen_names.insert(section.name.clone());
            section.bidder = AttentionBidder::Neuro;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Workspace;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
            let context_section = ContextSection::scoped(
                section,
                ContextSource::KnowledgeEntry {
                    entry_id: format!("knowledge:{}:{}", request.plan_id, request.task_id),
                    kind: "knowledge".to_string(),
                    source: None,
                },
                ContextPurpose::TaskGuidance,
                ContextScope::task(&request.plan_id, &request.task_id),
                "unified-retrieval: knowledge source",
            );
            out.push(ContextCandidate {
                section: context_section,
                relevance: KNOWLEDGE_DEFAULT_RELEVANCE,
                bidder: AttentionBidder::Neuro,
            });
        }
    }

    fn collect_episodes(
        &self,
        request: &ContextRequest,
        scope: &ComposeScope,
        budget: usize,
        seen_names: &mut HashSet<String>,
        out: &mut Vec<ContextCandidate>,
    ) {
        let Some(provider) = &self.episodes else {
            return;
        };
        if budget == 0 {
            return;
        }
        let sections = provider.query_sections(scope, Some(budget));
        for mut section in sections {
            if seen_names.contains(&section.name) {
                continue;
            }
            seen_names.insert(section.name.clone());
            section.bidder = AttentionBidder::IterationMemory;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Plan;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
            let context_section = ContextSection::scoped(
                section,
                ContextSource::Episode {
                    episode_id: format!("episode:{}:{}", request.plan_id, request.task_id),
                    plan_id: request.plan_id.clone(),
                    task_id: request.task_id.clone(),
                },
                ContextPurpose::DependencyMemory,
                ContextScope::task(&request.plan_id, &request.task_id),
                "unified-retrieval: episode source",
            );
            out.push(ContextCandidate {
                section: context_section,
                relevance: EPISODE_DEFAULT_RELEVANCE,
                bidder: AttentionBidder::IterationMemory,
            });
        }
    }

    fn collect_code_index(
        &self,
        request: &ContextRequest,
        scope: &ComposeScope,
        budget: usize,
        seen_names: &mut HashSet<String>,
        out: &mut Vec<ContextCandidate>,
    ) {
        let Some(provider) = &self.code_index else {
            return;
        };
        if budget == 0 {
            return;
        }
        let sections = provider.query_sections(scope, Some(budget));
        for mut section in sections {
            if seen_names.contains(&section.name) {
                continue;
            }
            seen_names.insert(section.name.clone());
            section.bidder = AttentionBidder::CodeIntelligence;
            if section.cache_layer == CacheLayer::default() {
                section.cache_layer = CacheLayer::Workspace;
            }
            if section.placement == Placement::default() {
                section.placement = Placement::Middle;
            }
            let context_section = ContextSection::scoped(
                section,
                ContextSource::SymbolSignature {
                    symbol: request.task_id.clone(),
                    file: String::new(),
                },
                ContextPurpose::SourceEvidence,
                ContextScope::task(&request.plan_id, &request.task_id),
                "unified-retrieval: code index source",
            );
            out.push(ContextCandidate {
                section: context_section,
                relevance: CODE_INDEX_DEFAULT_RELEVANCE,
                bidder: AttentionBidder::CodeIntelligence,
            });
        }
    }
}

/// Per-source token budget split.
struct PerSourceBudget {
    knowledge: usize,
    episodes: usize,
    code_index: usize,
}

// ── ContextBidder implementation ─────────────────────────────────────────────

impl ContextBidder for UnifiedRetrievalContextBidder {
    fn bidder_id(&self) -> &'static str {
        "unified-retrieval"
    }

    fn propose_context(
        &self,
        _provider: &ContextProvider,
        request: &ContextRequest,
    ) -> Vec<ContextCandidate> {
        // Resolve effective budget for this role.
        let role_name = request
            .role_profile
            .as_ref()
            .map(|r| r.role_id.clone())
            .unwrap_or_default();
        let total_budget = self
            .config
            .effective_token_budget_for_role(&role_name)
            .min(request.budget_tokens);

        if total_budget == 0 {
            return Vec::new();
        }

        let budgets = self.per_source_budget(total_budget);
        let scope = Self::compose_scope(request);
        let mut all_candidates: Vec<ContextCandidate> = Vec::new();
        let mut seen_names: HashSet<String> = HashSet::new();

        // ── Knowledge source ──────────────────────────────────────────────────
        self.collect_knowledge(
            request,
            &scope,
            budgets.knowledge,
            &mut seen_names,
            &mut all_candidates,
        );

        // ── Episode source ────────────────────────────────────────────────────
        self.collect_episodes(
            request,
            &scope,
            budgets.episodes,
            &mut seen_names,
            &mut all_candidates,
        );

        // ── Code-index source ─────────────────────────────────────────────────
        if self.config.enable_code_index {
            self.collect_code_index(
                request,
                &scope,
                budgets.code_index,
                &mut seen_names,
                &mut all_candidates,
            );
        }

        if all_candidates.is_empty() {
            return Vec::new();
        }

        // ── Optional reranking (RAG-22) ───────────────────────────────────────
        if let Some(reranker) = &self.reranker {
            let query = Self::query_string(request);
            // Collect the text representation of each candidate for reranking.
            let texts: Vec<&str> = all_candidates
                .iter()
                .map(|c| c.section.section.content.as_str())
                .collect();
            let ranked = reranker.rerank(&query, &texts);
            // Apply reranked scores back to the relevance field.
            for (original_idx, score) in &ranked {
                if let Some(candidate) = all_candidates.get_mut(*original_idx) {
                    candidate.relevance = *score as f32;
                }
            }
            // Re-sort by new relevance scores, descending.
            all_candidates.sort_by(|a, b| {
                b.relevance
                    .partial_cmp(&a.relevance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            // No reranker: sort by relevance descending (knowledge first by default).
            all_candidates.sort_by(|a, b| {
                b.relevance
                    .partial_cmp(&a.relevance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        self.filter_and_cap(all_candidates)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_provider::TaskInput;
    use crate::graph_cells::signals::ComposeScope;
    use crate::prompt::PromptSection;

    // ── Stub providers ────────────────────────────────────────────────────────

    struct FixedKnowledge(Vec<(&'static str, &'static str)>);
    impl KnowledgeProvider for FixedKnowledge {
        fn query_sections(
            &self,
            _scope: &ComposeScope,
            _budget: Option<usize>,
        ) -> Vec<PromptSection> {
            self.0
                .iter()
                .map(|(name, content)| PromptSection::new(*name, *content))
                .collect()
        }
    }

    struct FixedEpisodes(Vec<(&'static str, &'static str)>);
    impl EpisodeProvider for FixedEpisodes {
        fn query_sections(
            &self,
            _scope: &ComposeScope,
            _budget: Option<usize>,
        ) -> Vec<PromptSection> {
            self.0
                .iter()
                .map(|(name, content)| PromptSection::new(*name, *content))
                .collect()
        }
    }

    struct FixedCodeIndex(Vec<(&'static str, &'static str)>);
    impl CodeIndexProvider for FixedCodeIndex {
        fn query_sections(
            &self,
            _scope: &ComposeScope,
            _budget: Option<usize>,
        ) -> Vec<PromptSection> {
            self.0
                .iter()
                .map(|(name, content)| PromptSection::new(*name, *content))
                .collect()
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn minimal_request() -> ContextRequest {
        ContextRequest {
            tier: crate::context_provider::ContextTier::Focused,
            budget_tokens: 8000,
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
            task_files: vec![],
            task: None,
            plan_artifacts: None,
            siblings: vec![],
            prior_outputs: vec![],
            role_profile: None,
            prompt_policy: None,
        }
    }

    fn dummy_provider() -> ContextProvider {
        ContextProvider::new(std::path::PathBuf::from("/tmp"))
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn empty_bidder_returns_no_candidates() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config);
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert!(candidates.is_empty());
    }

    #[test]
    fn knowledge_source_tagged_with_neuro_bidder() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![("kn_fact", "Rust uses ownership")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].bidder, AttentionBidder::Neuro);
    }

    #[test]
    fn episode_source_tagged_with_iteration_memory_bidder() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_episodes(FixedEpisodes(vec![("ep_01", "Episode summary")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].bidder, AttentionBidder::IterationMemory);
    }

    #[test]
    fn code_index_source_tagged_with_code_intelligence_bidder() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_code_index(FixedCodeIndex(vec![("fn_signature", "pub fn foo()")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].bidder, AttentionBidder::CodeIntelligence);
    }

    #[test]
    fn duplicate_names_deduplicated_across_sources() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![("shared_name", "from knowledge")]))
            .with_episodes(FixedEpisodes(vec![("shared_name", "from episodes")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        // Only one of the two should survive deduplication.
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn results_from_all_three_sources_merged() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![("kn", "knowledge content")]))
            .with_episodes(FixedEpisodes(vec![("ep", "episode content")]))
            .with_code_index(FixedCodeIndex(vec![("ci", "code index content")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert_eq!(candidates.len(), 3);
        // Sorted by relevance descending: knowledge (0.72) > episodes (0.68) > code index (0.65)
        assert_eq!(candidates[0].bidder, AttentionBidder::Neuro);
        assert_eq!(candidates[1].bidder, AttentionBidder::IterationMemory);
        assert_eq!(candidates[2].bidder, AttentionBidder::CodeIntelligence);
    }

    #[test]
    fn max_results_cap_respected() {
        let mut config = RetrievalConfig::default();
        config.max_results = 2;
        let bidder =
            UnifiedRetrievalContextBidder::new(config).with_knowledge(FixedKnowledge(vec![
                ("kn1", "knowledge 1"),
                ("kn2", "knowledge 2"),
                ("kn3", "knowledge 3"),
            ]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert!(candidates.len() <= 2);
    }

    #[test]
    fn code_index_disabled_by_config() {
        let mut config = RetrievalConfig::default();
        config.enable_code_index = false;
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_code_index(FixedCodeIndex(vec![("ci", "code index content")]));
        let provider = dummy_provider();
        let request = minimal_request();
        let candidates = bidder.propose_context(&provider, &request);
        assert!(
            candidates.is_empty(),
            "code index disabled should produce no candidates"
        );
    }

    #[test]
    fn reranker_applied_when_set() {
        use crate::reranker::HeuristicReranker;

        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![("kn", "rust programming")]))
            .with_episodes(FixedEpisodes(vec![("ep", "unrelated content here")]))
            .with_reranker(Box::new(HeuristicReranker::default()));
        let provider = dummy_provider();
        let mut request = minimal_request();
        // Use a task whose description strongly matches "rust programming".
        request.task = Some(TaskInput {
            id: "t1".to_string(),
            title: "rust programming guide".to_string(),
            description: Some("rust programming guide".to_string()),
            tier: "focused".to_string(),
            files: vec![],
            read_files: vec![],
            symbols: vec![],
            anti_patterns: vec![],
            prior_failures: vec![],
            verify_commands: vec![],
            acceptance: vec![],
            depends_on: vec![],
            max_loc: None,
        });
        let candidates = bidder.propose_context(&provider, &request);
        // Both should still be returned; reranker should rank the matching one first.
        assert_eq!(candidates.len(), 2);
        // The "rust programming" knowledge entry should score higher than "unrelated content".
        assert_eq!(candidates[0].bidder, AttentionBidder::Neuro);
    }

    #[test]
    fn zero_budget_returns_empty() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![("kn", "knowledge")]));
        let provider = dummy_provider();
        let mut request = minimal_request();
        request.budget_tokens = 0;
        let candidates = bidder.propose_context(&provider, &request);
        assert!(candidates.is_empty());
    }

    #[test]
    fn bidder_id_is_stable() {
        let bidder = UnifiedRetrievalContextBidder::new(RetrievalConfig::default());
        assert_eq!(bidder.bidder_id(), "unified-retrieval");
    }

    #[test]
    fn per_source_budget_splits_proportionally_with_all_three() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config)
            .with_knowledge(FixedKnowledge(vec![]))
            .with_episodes(FixedEpisodes(vec![]))
            .with_code_index(FixedCodeIndex(vec![]));
        let b = bidder.per_source_budget(1000);
        // Fractions: knowledge=0.4, episodes=0.3, code=0.3
        // Sum should not exceed the total.
        assert!(b.knowledge + b.episodes + b.code_index <= 1000);
        assert!(b.knowledge > 0);
        assert!(b.episodes > 0);
        assert!(b.code_index > 0);
    }

    #[test]
    fn per_source_budget_with_only_knowledge() {
        let config = RetrievalConfig::default();
        let bidder =
            UnifiedRetrievalContextBidder::new(config).with_knowledge(FixedKnowledge(vec![]));
        let b = bidder.per_source_budget(1000);
        // Knowledge gets 100% when it's the only active source.
        assert_eq!(b.knowledge, 1000);
        assert_eq!(b.episodes, 0);
        assert_eq!(b.code_index, 0);
    }

    #[test]
    fn per_source_budget_no_sources_returns_zeros() {
        let config = RetrievalConfig::default();
        let bidder = UnifiedRetrievalContextBidder::new(config);
        let b = bidder.per_source_budget(1000);
        assert_eq!(b.knowledge, 0);
        assert_eq!(b.episodes, 0);
        assert_eq!(b.code_index, 0);
    }
}
