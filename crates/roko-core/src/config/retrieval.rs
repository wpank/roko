//! Retrieval (RAG) configuration section.

use std::collections::HashMap;

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

/// RAG-12: Task-complexity hint used by the adaptive depth selector.
///
/// Simple tasks (short description, single domain) receive a shallower
/// retrieval depth; complex tasks (multi-domain, long description) receive a
/// deeper one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// Short, single-concern task — retrieval depth = `min_results`.
    Simple,
    /// Default heuristic bucket — retrieval depth = `max_results / 2`.
    #[default]
    Moderate,
    /// Multi-step or cross-domain task — retrieval depth = `max_results`.
    Complex,
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
        })
    }
}

const fn default_max_results() -> usize {
    10
}

const fn default_min_results() -> usize {
    3
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
/// min_results = 3
/// min_score = 0.1
/// token_budget = 4000
/// enable_code_index = true
///
/// [retrieval.role_token_budgets]
/// researcher = 8000
/// implementer = 4000
/// auditor = 3000
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalConfig {
    /// Retrieval strategy. Default: `hybrid`.
    #[serde(default)]
    pub mode: RetrievalMode,
    /// Maximum number of retrieval results to return (used for complex tasks).
    /// Default: 10.
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Minimum number of retrieval results to return (used for simple tasks).
    ///
    /// RAG-12: adaptive depth clamps the result count to `[min_results, max_results]`
    /// depending on detected task complexity. Default: 3.
    #[serde(default = "default_min_results")]
    pub min_results: usize,
    /// Minimum relevance score threshold. Results below this are discarded.
    /// Default: 0.1.
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    /// Global maximum token budget allocated for retrieved context.
    ///
    /// Acts as the fallback cap for any role that does not have an entry in
    /// [`role_token_budgets`](Self::role_token_budgets). Default: 4000.
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    /// Whether the code index is consulted during retrieval. Default: true.
    #[serde(default = "default_true")]
    pub enable_code_index: bool,
    /// Enable dense vector embedding during retrieval.
    ///
    /// When `true` (and `mode` is `"dense"` or `"hybrid"`), the pipeline
    /// will call a `DenseEmbeddingAdapter` to produce float-vector
    /// representations for similarity search.  Default: `false`.
    #[serde(default)]
    pub dense_embedding_enabled: bool,
    /// Enable cross-encoder reranking on the retrieval result set.
    ///
    /// When `true` a `Reranker` implementation post-processes the ranked
    /// candidate list before context is assembled.  Default: `false`.
    #[serde(default)]
    pub rerank_enabled: bool,
    /// Per-role token budget overrides.
    ///
    /// Maps a role name (e.g. `"researcher"`, `"implementer"`) to the maximum
    /// number of tokens that role may use for retrieved context. When a role is
    /// not listed here the global [`token_budget`](Self::token_budget) applies.
    ///
    /// Example:
    /// ```toml
    /// [retrieval.role_token_budgets]
    /// researcher = 8000
    /// implementer = 4000
    /// ```
    #[serde(default)]
    pub role_token_budgets: HashMap<String, usize>,
}

impl RetrievalConfig {
    /// Return the effective retrieval token budget for `role`.
    ///
    /// Looks up `role` in [`role_token_budgets`](Self::role_token_budgets)
    /// (case-insensitive). If no entry is found, falls back to the global
    /// [`token_budget`](Self::token_budget).
    ///
    /// The lookup first tries the exact key, then a lowercase comparison so
    /// that config keys like `"Researcher"` and `"researcher"` both match.
    #[must_use]
    pub fn effective_token_budget_for_role(&self, role: &str) -> usize {
        if let Some(&budget) = self.role_token_budgets.get(role) {
            return budget;
        }
        // Case-insensitive fallback.
        let lower = role.to_lowercase();
        if let Some(&budget) = self.role_token_budgets.get(lower.as_str()) {
            return budget;
        }
        self.token_budget
    }

    /// RAG-12: Return the number of results to retrieve for a given task complexity.
    ///
    /// The depth is chosen from the `[min_results, max_results]` range:
    ///
    /// | Complexity | Result count |
    /// |---|---|
    /// | `Simple`   | `min_results` |
    /// | `Moderate` | midpoint of `[min_results, max_results]` |
    /// | `Complex`  | `max_results` |
    ///
    /// The returned value is always in `[min_results, max_results]`.
    #[must_use]
    pub fn depth_for_task(&self, complexity: TaskComplexity) -> usize {
        let lo = self.min_results.min(self.max_results);
        let hi = self.max_results.max(self.min_results);
        match complexity {
            TaskComplexity::Simple => lo,
            TaskComplexity::Moderate => lo + (hi - lo) / 2,
            TaskComplexity::Complex => hi,
        }
    }

    /// RAG-12: Infer task complexity from a free-text task description.
    ///
    /// This is a lightweight heuristic — not an LLM call — that measures the
    /// description length and counts certain complexity markers (commas,
    /// semicolons, "and", "also", "additionally", "multiple", "several").
    ///
    /// | Score | Classification |
    /// |---|---|
    /// | 0–1 | `Simple` |
    /// | 2–3 | `Moderate` |
    /// | 4+  | `Complex` |
    ///
    /// Callers can override this with an explicit [`TaskComplexity`] when they
    /// have better signal (e.g., a tier label from the task manifest).
    #[must_use]
    pub fn complexity_for_text(text: &str) -> TaskComplexity {
        let lower = text.to_lowercase();
        let words = lower.split_whitespace().count();

        let mut score: u32 = 0;

        // Length heuristic: long descriptions usually indicate more complexity.
        if words > 80 {
            score += 2;
        } else if words > 30 {
            score += 1;
        }

        // Coordination markers that suggest multi-step work.
        let markers = [
            "and also",
            "additionally",
            "furthermore",
            "multiple",
            "several",
            "cross-domain",
            "end-to-end",
            "integration",
        ];
        for marker in &markers {
            if lower.contains(marker) {
                score += 1;
            }
        }

        // Punctuation density (commas / semicolons suggest enumerated steps).
        let punct: u32 = text.chars().filter(|&c| c == ',' || c == ';').count() as u32;
        if punct >= 4 {
            score += 2;
        } else if punct >= 2 {
            score += 1;
        }

        match score {
            0..=1 => TaskComplexity::Simple,
            2..=3 => TaskComplexity::Moderate,
            _ => TaskComplexity::Complex,
        }
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            mode: RetrievalMode::default(),
            max_results: default_max_results(),
            min_results: default_min_results(),
            min_score: default_min_score(),
            token_budget: default_token_budget(),
            enable_code_index: default_true(),
            dense_embedding_enabled: false,
            rerank_enabled: false,
            role_token_budgets: HashMap::new(),
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
        assert!(!config.dense_embedding_enabled);
        assert!(!config.rerank_enabled);
    }

    #[test]
    fn retrieval_config_from_empty_toml() {
        let config: RetrievalConfig = toml::from_str("").unwrap();
        assert_eq!(config, RetrievalConfig::default());
    }

    #[test]
    fn retrieval_config_defaults_include_min_results() {
        let config = RetrievalConfig::default();
        assert_eq!(config.min_results, 3);
    }

    #[test]
    fn retrieval_config_roundtrip() {
        let mut role_budgets = HashMap::new();
        role_budgets.insert("researcher".to_string(), 8000usize);
        let config = RetrievalConfig {
            mode: RetrievalMode::HdcOnly,
            max_results: 20,
            min_results: 5,
            min_score: 0.25,
            token_budget: 8000,
            enable_code_index: false,
            dense_embedding_enabled: true,
            rerank_enabled: true,
            role_token_budgets: role_budgets,
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
        assert!(config.role_token_budgets.is_empty());
    }

    #[test]
    fn effective_token_budget_falls_back_to_global() {
        let config = RetrievalConfig::default();
        assert_eq!(
            config.effective_token_budget_for_role("researcher"),
            4000,
            "no per-role entry => global token_budget"
        );
    }

    #[test]
    fn effective_token_budget_returns_per_role_override() {
        let mut config = RetrievalConfig::default();
        config
            .role_token_budgets
            .insert("researcher".to_string(), 8000);
        config
            .role_token_budgets
            .insert("implementer".to_string(), 3000);

        assert_eq!(config.effective_token_budget_for_role("researcher"), 8000);
        assert_eq!(config.effective_token_budget_for_role("implementer"), 3000);
        // Unknown role falls back to global.
        assert_eq!(config.effective_token_budget_for_role("auditor"), 4000);
    }

    #[test]
    fn effective_token_budget_case_insensitive() {
        let mut config = RetrievalConfig::default();
        config
            .role_token_budgets
            .insert("researcher".to_string(), 7500);
        // Exact key does not match "Researcher" but lowercase fallback should.
        assert_eq!(config.effective_token_budget_for_role("Researcher"), 7500);
    }

    #[test]
    fn retrieval_config_with_role_budgets_from_toml() {
        let toml_str = r#"
token_budget = 5000

[role_token_budgets]
researcher = 10000
implementer = 3500
"#;
        let config: RetrievalConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(config.token_budget, 5000);
        assert_eq!(config.effective_token_budget_for_role("researcher"), 10000);
        assert_eq!(config.effective_token_budget_for_role("implementer"), 3500);
        // Unknown role => global.
        assert_eq!(config.effective_token_budget_for_role("auditor"), 5000);
    }

    // ── RAG-12: adaptive depth tests ────────────────────────────────────────

    #[test]
    fn depth_for_task_simple_returns_min() {
        let config = RetrievalConfig::default(); // min=3, max=10
        assert_eq!(config.depth_for_task(TaskComplexity::Simple), 3);
    }

    #[test]
    fn depth_for_task_complex_returns_max() {
        let config = RetrievalConfig::default();
        assert_eq!(config.depth_for_task(TaskComplexity::Complex), 10);
    }

    #[test]
    fn depth_for_task_moderate_is_midpoint() {
        let config = RetrievalConfig::default(); // min=3, max=10 → mid = 3 + 7/2 = 6
        assert_eq!(config.depth_for_task(TaskComplexity::Moderate), 6);
    }

    #[test]
    fn depth_for_task_custom_bounds() {
        let config = RetrievalConfig {
            min_results: 2,
            max_results: 20,
            ..RetrievalConfig::default()
        };
        assert_eq!(config.depth_for_task(TaskComplexity::Simple), 2);
        assert_eq!(config.depth_for_task(TaskComplexity::Moderate), 11); // 2 + 18/2
        assert_eq!(config.depth_for_task(TaskComplexity::Complex), 20);
    }

    #[test]
    fn complexity_for_text_short_is_simple() {
        let complexity = RetrievalConfig::complexity_for_text("Fix the login bug");
        assert_eq!(complexity, TaskComplexity::Simple);
    }

    #[test]
    fn complexity_for_text_long_is_complex() {
        let text = "Implement end-to-end integration between the authentication service \
                    and the database layer, additionally wire up the event bus, \
                    configure multiple providers, and ensure cross-domain data flow. \
                    The task involves several subsystems: auth, storage, events, and \
                    the CLI. Make sure to handle edge cases, error recovery, and \
                    backward compatibility throughout the implementation.";
        let complexity = RetrievalConfig::complexity_for_text(text);
        assert_eq!(complexity, TaskComplexity::Complex);
    }

    #[test]
    fn complexity_for_text_moderate_text() {
        // Medium-length description, no strong markers.
        let text = "Refactor the cache module to extract the eviction policy into a \
                    separate trait. Update the existing tests to match the new interface \
                    and document the new trait.";
        let complexity = RetrievalConfig::complexity_for_text(text);
        // Should be simple or moderate but definitely not complex.
        assert_ne!(complexity, TaskComplexity::Complex);
    }

    #[test]
    fn task_complexity_display() {
        assert_eq!(TaskComplexity::Simple.to_string(), "simple");
        assert_eq!(TaskComplexity::Moderate.to_string(), "moderate");
        assert_eq!(TaskComplexity::Complex.to_string(), "complex");
    }
}
