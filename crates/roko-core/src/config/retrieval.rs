//! Retrieval (RAG) configuration section.

use serde::{Deserialize, Serialize};

/// Retrieval strategy for combining keyword and HDC (vector) search results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    /// Combine keyword and HDC vector results (default).
    #[default]
    Hybrid,
    /// Use only keyword/text search.
    KeywordOnly,
    /// Use only HDC vector similarity search.
    HdcOnly,
}

impl std::fmt::Display for RetrievalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Hybrid => "hybrid",
            Self::KeywordOnly => "keyword_only",
            Self::HdcOnly => "hdc_only",
        })
    }
}

const fn default_max_results() -> usize {
    10
}

const fn default_min_score() -> f64 {
    0.1
}

const fn default_token_budget() -> usize {
    4000
}

const fn default_true() -> bool {
    true
}

/// RAG retrieval configuration.
///
/// Controls how the retrieval pipeline selects and budgets context for
/// prompt composition. All fields have safe defaults so existing configs
/// continue to work without a `[retrieval]` section.
///
/// ```toml
/// [retrieval]
/// mode = "hybrid"
/// max_results = 10
/// min_score = 0.1
/// token_budget = 4000
/// enable_code_index = true
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalConfig {
    /// Retrieval strategy. Default: `hybrid`.
    #[serde(default)]
    pub mode: RetrievalMode,
    /// Maximum number of retrieval results to return. Default: 10.
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Minimum relevance score threshold. Results below this are discarded.
    /// Default: 0.1.
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    /// Maximum token budget allocated for retrieved context. Default: 4000.
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    /// Whether the code index is consulted during retrieval. Default: true.
    #[serde(default = "default_true")]
    pub enable_code_index: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            mode: RetrievalMode::default(),
            max_results: default_max_results(),
            min_score: default_min_score(),
            token_budget: default_token_budget(),
            enable_code_index: default_true(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_config_defaults() {
        let config = RetrievalConfig::default();
        assert_eq!(config.mode, RetrievalMode::Hybrid);
        assert_eq!(config.max_results, 10);
        assert!((config.min_score - 0.1).abs() < f64::EPSILON);
        assert_eq!(config.token_budget, 4000);
        assert!(config.enable_code_index);
    }

    #[test]
    fn retrieval_config_from_empty_toml() {
        let config: RetrievalConfig = toml::from_str("").unwrap();
        assert_eq!(config, RetrievalConfig::default());
    }

    #[test]
    fn retrieval_config_roundtrip() {
        let config = RetrievalConfig {
            mode: RetrievalMode::HdcOnly,
            max_results: 20,
            min_score: 0.25,
            token_budget: 8000,
            enable_code_index: false,
        };
        let serialized = toml::to_string(&config).expect("serialize");
        let deserialized: RetrievalConfig = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn retrieval_mode_display() {
        assert_eq!(RetrievalMode::Hybrid.to_string(), "hybrid");
        assert_eq!(RetrievalMode::KeywordOnly.to_string(), "keyword_only");
        assert_eq!(RetrievalMode::HdcOnly.to_string(), "hdc_only");
    }

    #[test]
    fn retrieval_config_partial_toml() {
        let config: RetrievalConfig =
            toml::from_str("mode = \"keyword_only\"\nmax_results = 5").unwrap();
        assert_eq!(config.mode, RetrievalMode::KeywordOnly);
        assert_eq!(config.max_results, 5);
        // Other fields take defaults.
        assert!((config.min_score - 0.1).abs() < f64::EPSILON);
        assert_eq!(config.token_budget, 4000);
        assert!(config.enable_code_index);
    }
}
