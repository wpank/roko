//! Property-based tests for roko-compose core types.

use proptest::prelude::*;
use roko_compose::system_prompt_builder::{SystemPromptBuilder, normalize_for_caching};
use roko_compose::prompt::{estimate_tokens, CacheLayer, Placement, SectionPriority, PromptSection};

// ─── normalize_for_caching ───────────────────────────────────────────────────

proptest! {
    /// Normalizing the same string twice is idempotent.
    #[test]
    fn normalize_idempotent(s in "\\PC{0,200}") {
        let once = normalize_for_caching(&s);
        let twice = normalize_for_caching(&once);
        prop_assert_eq!(&once, &twice,
            "normalize_for_caching must be idempotent");
    }

    /// Normalized output never contains carriage returns.
    #[test]
    fn normalize_no_carriage_returns(s in "\\PC{0,200}") {
        let normalized = normalize_for_caching(&s);
        prop_assert!(!normalized.contains('\r'),
            "normalized string contains \\r: {:?}", normalized);
    }

    /// Normalized output never contains trailing whitespace on any line.
    #[test]
    fn normalize_no_trailing_whitespace(s in "[a-zA-Z0-9 \t\n\r]{0,100}") {
        let normalized = normalize_for_caching(&s);
        for line in normalized.lines() {
            prop_assert_eq!(
                line,
                line.trim_end(),
                "line has trailing whitespace: {:?}", line
            );
        }
    }
}

// ─── SystemPromptBuilder::build ─────────────────────────────────────────────

proptest! {
    /// build() returns non-empty output when role_identity is non-empty.
    #[test]
    fn build_non_empty_for_non_empty_role(role in "[A-Za-z]{1,40}") {
        let prompt = SystemPromptBuilder::new(&role).build();
        prop_assert!(!prompt.is_empty(),
            "build() with role '{role}' produced empty output");
    }

    /// build() always contains the role identity text somewhere in the output.
    #[test]
    fn build_contains_role_identity(role in "[A-Za-z0-9 ]{1,40}") {
        let prompt = SystemPromptBuilder::new(role.trim()).build();
        let trimmed = role.trim();
        prop_assert!(
            prompt.contains(trimmed),
            "build() output does not contain role identity '{trimmed}'.\nOutput: {prompt}"
        );
    }

    /// A prompt built from non-empty input never contains unfilled {{ }} placeholders.
    #[test]
    fn build_has_no_unfilled_braces(
        role in "[A-Za-z ]{1,30}",
        task in "[A-Za-z ]{1,30}",
    ) {
        let prompt = SystemPromptBuilder::new(role.trim())
            .with_task(task.trim())
            .build();
        prop_assert!(!prompt.contains("{{"),
            "build() output contains '{{{{' placeholder: {prompt}");
        prop_assert!(!prompt.contains("}}"),
            "build() output contains '}}}}' placeholder: {prompt}");
    }
}

// ─── estimate_tokens ────────────────────────────────────────────────────────

proptest! {
    /// Token estimate for an empty string is 0.
    #[test]
    fn empty_string_zero_tokens(_dummy in 0u32..1) {
        prop_assert_eq!(estimate_tokens(""), 0);
    }

    /// Token estimate is monotonically non-decreasing as text grows.
    #[test]
    fn token_estimate_monotone(prefix in "[A-Za-z ]{0,50}", suffix in "[A-Za-z ]{1,20}") {
        let combined = format!("{prefix}{suffix}");
        prop_assert!(estimate_tokens(&combined) >= estimate_tokens(&prefix),
            "adding text must not decrease token estimate");
    }

    /// Token estimate for a single-character string is 1.
    #[test]
    fn single_char_at_least_one_token(c in "[!-~]") {
        prop_assert_eq!(estimate_tokens(&c), 1);
    }
}

// ─── SectionPriority ordering ────────────────────────────────────────────────

proptest! {
    /// SectionPriority ordering: Low < Normal < High < Critical.
    #[test]
    fn section_priority_ordered(_dummy in 0u32..1) {
        prop_assert!(SectionPriority::Low < SectionPriority::Normal);
        prop_assert!(SectionPriority::Normal < SectionPriority::High);
        prop_assert!(SectionPriority::High < SectionPriority::Critical);
    }

    /// CacheLayer ordering: Role < Workspace < Plan < Volatile.
    #[test]
    fn cache_layer_ordered(_dummy in 0u32..1) {
        prop_assert!(CacheLayer::Role < CacheLayer::Workspace);
        prop_assert!(CacheLayer::Workspace < CacheLayer::Plan);
        prop_assert!(CacheLayer::Plan < CacheLayer::Volatile);
    }

    /// PromptSection::new sets sensible defaults.
    #[test]
    fn prompt_section_default_priority(
        name in "[a-z]{1,12}",
        content in "[A-Za-z ]{1,60}",
    ) {
        let section = PromptSection::new(name.clone(), content.clone());
        prop_assert_eq!(section.priority, SectionPriority::Normal,
            "default priority should be Normal");
        prop_assert_eq!(section.cache_layer, CacheLayer::Plan,
            "default cache_layer should be Plan");
        prop_assert_eq!(section.placement, Placement::Middle,
            "default placement should be Middle");
        prop_assert_eq!(&section.name, &name);
        prop_assert_eq!(&section.content, &content);
    }
}
