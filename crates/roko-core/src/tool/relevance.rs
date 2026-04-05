//! Tool relevance scoring (§36.j/k, parity items 36.76–36.80).
//!
//! **Why progressive discovery**: Klavis Strata reported 2× success rate
//! on GitHub tasks via progressive tool disclosure; Qwen3-coder
//! format-switches above 5 tools; Vercel cut 80% of tools and got
//! **better** results. Small models need a ranked subset; big models
//! can handle the full registry. The subset comes from a relevance
//! scorer with task-context input.
//!
//! This module defines the scorer trait and a trivial keyword-overlap
//! scorer for day-one use. Embedding-backed scorers live in `roko-std`
//! (which depends on `roko-index` once that lands).

#![allow(clippy::unnecessary_literal_bound)] // trait default impls returning &str literals

use super::def::ToolDef;

// ─── ToolRelevanceScorer trait ────────────────────────────────────────────

/// Score how relevant a tool is to a task description, in `[0, 1]`.
///
/// Scorers are **pure** — given a fixed `(task_description, tool)` pair
/// they return the same score. This makes scoring deterministic for
/// tests and replay. Expensive embedding lookups are cached by the
/// implementor.
pub trait ToolRelevanceScorer: Send + Sync {
    /// Score one tool's relevance to a task description.
    fn score(&self, task_description: &str, tool: &ToolDef) -> f32;

    /// Rank a slice of tools by descending relevance.
    fn rank<'a>(&self, task_description: &str, tools: &'a [ToolDef]) -> Vec<&'a ToolDef> {
        let mut pairs: Vec<(f32, &ToolDef)> = tools
            .iter()
            .map(|t| (self.score(task_description, t), t))
            .collect();
        pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        pairs.into_iter().map(|(_, t)| t).collect()
    }

    /// Return the top-`limit` most relevant tools.
    fn top_n<'a>(
        &self,
        task_description: &str,
        tools: &'a [ToolDef],
        limit: usize,
    ) -> Vec<&'a ToolDef> {
        let mut ranked = self.rank(task_description, tools);
        ranked.truncate(limit);
        ranked
    }

    /// Human-readable name for logs / TUI.
    fn name(&self) -> &str {
        "unnamed_relevance_scorer"
    }
}

// ─── KeywordOverlapScorer ─────────────────────────────────────────────────

/// Trivial keyword-overlap scorer: fraction of task-description word
/// stems present in the tool's `name + description`.
///
/// A reasonable day-one baseline — cheap, interpretable, no dependencies.
/// Embedding-backed scorers replace this in `roko-std` once `roko-index`
/// lands.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeywordOverlapScorer;

impl ToolRelevanceScorer for KeywordOverlapScorer {
    #[allow(clippy::cast_precision_loss)]
    fn score(&self, task_description: &str, tool: &ToolDef) -> f32 {
        let task_terms = normalize_terms(task_description);
        if task_terms.is_empty() {
            return 0.0;
        }
        let haystack = format!("{} {}", tool.name, tool.description).to_lowercase();
        let mut hits = 0_usize;
        for term in &task_terms {
            if haystack.contains(term) {
                hits += 1;
            }
        }
        hits as f32 / task_terms.len() as f32
    }

    fn name(&self) -> &str {
        "keyword_overlap"
    }
}

fn normalize_terms(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 3) // skip stopword-length tokens
        .map(std::string::ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::def::{ToolCategory, ToolPermission};

    fn def(name: &str, desc: &str) -> ToolDef {
        ToolDef::new(name, desc, ToolCategory::Read, ToolPermission::read_only())
    }

    fn all() -> Vec<ToolDef> {
        vec![
            def("read_file", "Read a UTF-8 text file with optional line range"),
            def("write_file", "Write a text file, creating or overwriting"),
            def("grep", "Search file contents using regex"),
            def("bash", "Execute a shell command and capture stdout"),
            def("web_fetch", "Fetch a URL over HTTPS and return the body"),
        ]
    }

    #[test]
    fn score_empty_description_is_zero() {
        let s = KeywordOverlapScorer;
        assert!(s.score("", &def("read_file", "read files")).abs() < f32::EPSILON);
    }

    #[test]
    fn score_high_for_matching_task() {
        let s = KeywordOverlapScorer;
        let tools = all();
        let read_score = s.score("need to read a file to understand the code", &tools[0]);
        let bash_score = s.score("need to read a file to understand the code", &tools[3]);
        assert!(read_score > bash_score);
    }

    #[test]
    fn rank_orders_descending() {
        let s = KeywordOverlapScorer;
        let tools = all();
        let ranked = s.rank("grep for pattern in files", &tools);
        // The grep tool should rank first.
        assert_eq!(ranked[0].name, "grep");
    }

    #[test]
    fn top_n_truncates() {
        let s = KeywordOverlapScorer;
        let tools = all();
        let top = s.top_n("fetch a url from the web", &tools, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "web_fetch");
    }

    #[test]
    fn rank_stable_on_ties() {
        let s = KeywordOverlapScorer;
        let tools = all();
        // Task with no term matches → all scores are 0 → stable order.
        let ranked = s.rank("xyz qwerty", &tools);
        assert_eq!(ranked.len(), tools.len());
    }

    #[test]
    fn normalize_terms_drops_short_and_nonalpha() {
        let terms = normalize_terms("Read-Files, with (foo) and X!!");
        // "read", "files", "with", "foo" kept; "and" kept (3+ chars); "x" dropped.
        assert!(terms.contains(&"read".to_string()));
        assert!(terms.contains(&"files".to_string()));
        assert!(terms.contains(&"with".to_string()));
        assert!(terms.contains(&"foo".to_string()));
        assert!(terms.contains(&"and".to_string()));
        assert!(!terms.contains(&"x".to_string()));
    }

    #[test]
    fn score_in_unit_interval() {
        let s = KeywordOverlapScorer;
        let tools = all();
        for t in &tools {
            let sc = s.score("read grep bash fetch write xyz garbage tokens", t);
            assert!((0.0..=1.0).contains(&sc));
        }
    }

    #[test]
    fn name_is_stable() {
        assert_eq!(KeywordOverlapScorer.name(), "keyword_overlap");
    }
}
