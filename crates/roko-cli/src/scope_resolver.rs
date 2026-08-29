//! Prompt scope classification for `roko do`.
//!
//! Two layers of classification:
//! 1. **Intent**: is this a research question, a plan generation request,
//!    or a direct coding task?
//! 2. **Complexity**: for coding tasks, how complex is the work?

use roko_core::config::schema::RokoConfig;
use roko_gate::PlanComplexity;

/// High-level intent detected from a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptIntent {
    /// A research/analysis question -- route to `roko research topic`.
    Research,
    /// A plan generation request -- route to `roko plan generate`.
    PlanGenerate,
    /// A coding task -- classify complexity and route through `roko do`.
    Task,
}

/// Resolves a user prompt into the existing gate pipeline complexity type.
pub struct ScopeResolver;

impl ScopeResolver {
    /// Classify a prompt with deterministic heuristics.
    pub async fn resolve(prompt: &str, config: &RokoConfig) -> PlanComplexity {
        let _ = config;
        Self::classify_prompt_complexity(prompt)
    }

    /// Classify the high-level intent of a prompt.
    ///
    /// Returns `Research` for investigation/analysis questions,
    /// `PlanGenerate` for explicit plan requests, and `Task` for everything else.
    #[must_use]
    pub fn classify_intent(prompt: &str) -> PromptIntent {
        let lower = prompt.to_ascii_lowercase();

        // Research markers: questions about architecture, analysis, investigation
        let research_markers = [
            "how does",
            "how do",
            "what is",
            "what are",
            "why does",
            "why do",
            "explain",
            "analyze",
            "analyse",
            "compare",
            "research",
            "investigate",
            "deep dive",
            "survey",
            "evaluate options",
            "what do we know",
            "pros and cons",
            "trade-offs",
            "tradeoffs",
        ];

        // Plan generation markers
        let plan_markers = [
            "create a plan",
            "generate a plan",
            "write a plan",
            "plan for",
            "make a plan",
            "break down into tasks",
            "decompose into",
            "task breakdown",
        ];

        // Question mark at the end is a strong research signal when combined
        // with analysis keywords
        let is_question = lower.trim_end().ends_with('?');

        if plan_markers.iter().any(|m| lower.contains(m)) {
            return PromptIntent::PlanGenerate;
        }

        if research_markers.iter().any(|m| lower.contains(m)) {
            return PromptIntent::Research;
        }

        // Pure questions without action verbs are likely research
        if is_question {
            let action_verbs = [
                "fix",
                "add",
                "implement",
                "create",
                "build",
                "write",
                "update",
                "remove",
                "delete",
                "change",
                "modify",
                "refactor",
            ];
            let has_action = action_verbs.iter().any(|v| lower.contains(v));
            if !has_action {
                return PromptIntent::Research;
            }
        }

        PromptIntent::Task
    }

    /// Classify a prompt into the existing gate-pipeline complexity type.
    #[must_use]
    pub fn classify_prompt_complexity(prompt: &str) -> PlanComplexity {
        let lower = prompt.to_ascii_lowercase();
        let words = lower.split_whitespace().count();
        let chars = lower.chars().count();
        let sentence_count = lower.matches(['.', '!', '?']).count();

        let trivial_markers = [
            "fix typo",
            "typo",
            "spelling",
            "rename",
            "update comment",
            "format",
            "readme",
            "single file",
        ];
        let complex_markers = [
            "architecture",
            "redesign",
            "refactor",
            "migration",
            "migrate",
            "cross-crate",
            "cross crate",
            "multi-agent",
            "multiple components",
            "entire",
            "sharding",
            "concurrency",
            "security",
            "authentication",
            "database schema",
            "streaming",
            "event architecture",
        ];
        let standard_markers = [
            "add ",
            "add feature",
            "implement",
            "new endpoint",
            "api",
            "cli command",
            "tests",
            "integration",
            "multiple files",
            "workflow",
        ];

        if words >= 60
            || sentence_count >= 3
            || complex_markers.iter().any(|marker| lower.contains(marker))
        {
            return PlanComplexity::Complex;
        }

        if chars <= 100
            && words <= 16
            && trivial_markers.iter().any(|marker| lower.contains(marker))
        {
            return PlanComplexity::Trivial;
        }

        if words >= 18 || standard_markers.iter().any(|marker| lower.contains(marker)) {
            return PlanComplexity::Standard;
        }

        PlanComplexity::Simple
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(prompt: &str) -> PlanComplexity {
        ScopeResolver::classify_prompt_complexity(prompt)
    }

    // ── Trivial classification ─────────────────────────────────────

    #[test]
    fn trivial_fix_typo() {
        assert_eq!(classify("Fix typo in README.md"), PlanComplexity::Trivial);
    }

    #[test]
    fn trivial_spelling() {
        assert_eq!(classify("Fix spelling mistake"), PlanComplexity::Trivial);
    }

    #[test]
    fn trivial_rename() {
        assert_eq!(
            classify("Rename variable foo to bar"),
            PlanComplexity::Trivial
        );
    }

    #[test]
    fn trivial_update_comment() {
        assert_eq!(
            classify("Update comment in lib.rs"),
            PlanComplexity::Trivial
        );
    }

    #[test]
    fn trivial_format() {
        assert_eq!(classify("Format the code"), PlanComplexity::Trivial);
    }

    #[test]
    fn trivial_readme() {
        assert_eq!(classify("Fix the readme"), PlanComplexity::Trivial);
    }

    #[test]
    fn trivial_single_file() {
        assert_eq!(classify("Fix single file issue"), PlanComplexity::Trivial);
    }

    #[test]
    fn trivial_requires_short_prompt() {
        // Trivial markers in a long prompt (>100 chars, >16 words) should NOT be trivial.
        let long = "Fix typo in the very long file path that contains many many many many \
                     words exceeding the threshold of what is considered a trivial task prompt";
        assert_ne!(classify(long), PlanComplexity::Trivial);
    }

    // ── Simple classification ──────────────────────────────────────

    #[test]
    fn simple_short_generic_prompt() {
        assert_eq!(classify("Fix the bug"), PlanComplexity::Simple);
    }

    #[test]
    fn simple_brief_description() {
        assert_eq!(classify("Change error message"), PlanComplexity::Simple);
    }

    #[test]
    fn simple_no_markers() {
        assert_eq!(classify("Hello world"), PlanComplexity::Simple);
    }

    // ── Standard classification ────────────────────────────────────

    #[test]
    fn standard_add_feature() {
        assert_eq!(
            classify("Add rate limiting to the API endpoint"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_implement() {
        assert_eq!(
            classify("Implement retry logic for HTTP calls"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_new_endpoint() {
        assert_eq!(
            classify("Create a new endpoint for user profiles"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_cli_command() {
        assert_eq!(
            classify("Build a CLI command for export"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_tests() {
        assert_eq!(
            classify("Write tests for the parser"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_workflow() {
        assert_eq!(
            classify("Create a workflow for CI"),
            PlanComplexity::Standard,
        );
    }

    #[test]
    fn standard_by_word_count() {
        // 18+ words without any marker should be Standard.
        let prompt = "please go ahead and update the configuration file \
                       so that it handles all the edge cases we discussed yesterday";
        assert!(prompt.split_whitespace().count() >= 18);
        assert_eq!(classify(prompt), PlanComplexity::Standard);
    }

    // ── Complex classification ─────────────────────────────────────

    #[test]
    fn complex_refactor() {
        assert_eq!(
            classify("Refactor the database layer"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_architecture() {
        assert_eq!(
            classify("Redesign the event architecture"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_migration() {
        assert_eq!(
            classify("Migrate from SQLite to PostgreSQL"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_sharding() {
        assert_eq!(
            classify("Add sharding support for the database"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_cross_crate() {
        assert_eq!(
            classify("Fix cross-crate dependency issue"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_security() {
        assert_eq!(
            classify("Implement security layer"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_authentication() {
        assert_eq!(
            classify("Add authentication to the API"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_by_sentence_count() {
        // 3+ sentences should be Complex.
        let prompt = "First do this. Then do that. Finally verify it works.";
        assert_eq!(classify(prompt), PlanComplexity::Complex);
    }

    #[test]
    fn complex_by_word_count() {
        // 60+ words should be Complex.
        let words: Vec<&str> = std::iter::repeat("word").take(61).collect();
        let prompt = words.join(" ");
        assert_eq!(classify(&prompt), PlanComplexity::Complex);
    }

    // ── Priority ordering ──────────────────────────────────────────

    #[test]
    fn complex_markers_override_trivial_markers() {
        // "refactor" is complex, even though "format" is trivial.
        assert_eq!(
            classify("Refactor and format the code"),
            PlanComplexity::Complex,
        );
    }

    #[test]
    fn complex_markers_override_standard_markers() {
        // "redesign" is complex, even though "implement" is standard.
        assert_eq!(
            classify("Redesign and implement the new system"),
            PlanComplexity::Complex,
        );
    }

    // ── Case insensitivity ─────────────────────────────────────────

    #[test]
    fn case_insensitive_trivial() {
        assert_eq!(classify("Fix TYPO in file"), PlanComplexity::Trivial);
    }

    #[test]
    fn case_insensitive_complex() {
        assert_eq!(classify("REFACTOR the module"), PlanComplexity::Complex);
    }

    #[test]
    fn case_insensitive_standard() {
        assert_eq!(classify("IMPLEMENT the feature"), PlanComplexity::Standard,);
    }

    // ── resolve() async wrapper ────────────────────────────────────

    #[tokio::test]
    async fn resolve_delegates_to_classify() {
        let config = RokoConfig::default();
        let result = ScopeResolver::resolve("Fix typo in lib.rs", &config).await;
        assert_eq!(result, PlanComplexity::Trivial);
    }

    // ── Intent classification ─────────────────────────────────────

    fn intent(prompt: &str) -> PromptIntent {
        ScopeResolver::classify_intent(prompt)
    }

    #[test]
    fn intent_research_how_does() {
        assert_eq!(
            intent("how does auth work in this codebase?"),
            PromptIntent::Research
        );
    }

    #[test]
    fn intent_research_explain() {
        assert_eq!(intent("explain the gate pipeline"), PromptIntent::Research);
    }

    #[test]
    fn intent_research_analyze() {
        assert_eq!(intent("analyze the error patterns"), PromptIntent::Research);
    }

    #[test]
    fn intent_research_what_is() {
        assert_eq!(
            intent("what is the cascade router?"),
            PromptIntent::Research
        );
    }

    #[test]
    fn intent_research_pure_question() {
        assert_eq!(
            intent("which model is best for code gen?"),
            PromptIntent::Research
        );
    }

    #[test]
    fn intent_plan_create_plan() {
        assert_eq!(
            intent("create a plan for adding auth"),
            PromptIntent::PlanGenerate
        );
    }

    #[test]
    fn intent_plan_break_down() {
        assert_eq!(
            intent("break down into tasks: add user profiles"),
            PromptIntent::PlanGenerate
        );
    }

    #[test]
    fn intent_task_fix_bug() {
        assert_eq!(intent("fix the login bug"), PromptIntent::Task);
    }

    #[test]
    fn intent_task_add_feature() {
        assert_eq!(intent("add rate limiting"), PromptIntent::Task);
    }

    #[test]
    fn intent_task_question_with_action_verb() {
        // "can you fix this?" has an action verb, so it's a task not research
        assert_eq!(intent("can you fix this bug?"), PromptIntent::Task);
    }
}
