//! Runtime-boundary adapters that implement `roko-gate` oracle traits around
//! `roko-agent` provider clients.
//!
//! `roko-gate` deliberately does not depend on `roko-agent` (avoiding a crate
//! cycle), so it defines minimal oracle traits (`SearchOracle`, `JudgeOracle`).
//! This module lives in `roko-cli` — which depends on both — and bridges the
//! gap by wrapping the concrete agent clients.

use async_trait::async_trait;
use roko_agent::perplexity::search::{PerplexitySearchClient, SearchQuery};
use roko_gate::fact_check::{SearchHit, SearchOracle};
use std::sync::Arc;

/// [`SearchOracle`] adapter backed by `roko-agent`'s [`PerplexitySearchClient`].
///
/// Constructed when the workspace config (or environment) provides a Perplexity
/// API key. The gate crate remains provider-neutral; all Perplexity-specific
/// details are confined to this adapter.
pub struct PerplexitySearchOracle {
    client: Arc<PerplexitySearchClient>,
}

impl PerplexitySearchOracle {
    /// Construct a new oracle wrapping a Perplexity search client built from
    /// the given API key.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Arc::new(PerplexitySearchClient::new(api_key)),
        }
    }
}

#[async_trait]
impl SearchOracle for PerplexitySearchOracle {
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, String> {
        let search_query = SearchQuery {
            query: query.to_string(),
            ..SearchQuery::default()
        };
        let response = self
            .client
            .search_single(&search_query)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response
            .results
            .into_iter()
            .map(|r| SearchHit { content: r.content })
            .collect())
    }
}
