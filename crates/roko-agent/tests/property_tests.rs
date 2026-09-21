//! Property-based tests for roko-agent core types.

use proptest::prelude::*;
use roko_core::agent::AgentBackend;
use roko_core::agent::ProviderKind;

// ─── ProviderKind: serde roundtrip ──────────────────────────────────────────

fn arb_provider_kind() -> impl Strategy<Value = ProviderKind> {
    prop_oneof![
        Just(ProviderKind::AnthropicApi),
        Just(ProviderKind::ClaudeCli),
        Just(ProviderKind::OpenAiCompat),
        Just(ProviderKind::CursorAcp),
        Just(ProviderKind::PerplexityApi),
        Just(ProviderKind::GeminiApi),
        Just(ProviderKind::GeminiCli),
        Just(ProviderKind::CerebrasApi),
        Just(ProviderKind::CursorCli),
        Just(ProviderKind::Hermes),
        Just(ProviderKind::OpenClaw),
        Just(ProviderKind::CodexCli),
    ]
}

proptest! {
    /// ProviderKind JSON serde roundtrip.
    #[test]
    fn provider_kind_serde_roundtrip(kind in arb_provider_kind()) {
        let json = serde_json::to_string(&kind).expect("serialize");
        let parsed: ProviderKind = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(kind, parsed);
    }

    /// ProviderKind::label() is non-empty for every variant.
    #[test]
    fn provider_kind_label_non_empty(kind in arb_provider_kind()) {
        let label = kind.label();
        prop_assert!(!label.is_empty());
    }

    /// ProviderKind::label() contains only lowercase ASCII, digits, and underscores.
    #[test]
    fn provider_kind_label_safe_chars(kind in arb_provider_kind()) {
        let label = kind.label();
        prop_assert!(
            label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "label contains unexpected chars: {label:?}"
        );
    }

    /// ProviderKind::to_backend() produces a consistent result for each call.
    #[test]
    fn provider_kind_to_backend_is_stable(kind in arb_provider_kind()) {
        let backend = kind.to_backend();
        let backend2 = kind.to_backend();
        prop_assert_eq!(backend, backend2);
    }

    /// ProviderKind::Display matches its label.
    #[test]
    fn provider_kind_display_matches_label(kind in arb_provider_kind()) {
        let displayed = kind.to_string();
        prop_assert_eq!(displayed.as_str(), kind.label());
    }
}

// ─── AgentBackend: serde roundtrip ───────────────────────────────────────────

fn arb_agent_backend() -> impl Strategy<Value = AgentBackend> {
    prop_oneof![
        Just(AgentBackend::Claude),
        Just(AgentBackend::Codex),
        Just(AgentBackend::Cursor),
        Just(AgentBackend::Ollama),
        Just(AgentBackend::Perplexity),
        Just(AgentBackend::Cerebras),
        Just(AgentBackend::Hermes),
        Just(AgentBackend::OpenClaw),
        Just(AgentBackend::GeminiCli),
    ]
}

proptest! {
    /// AgentBackend serde roundtrip.
    #[test]
    fn agent_backend_serde_roundtrip(backend in arb_agent_backend()) {
        let json = serde_json::to_string(&backend).expect("serialize");
        let parsed: AgentBackend = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(backend, parsed);
    }

    /// AgentBackend -> ProviderKind -> to_backend() produces a well-formed backend.
    ///
    /// The mapping is many-to-one in both directions, so we just verify the
    /// result is serializable rather than asserting an exact roundtrip value.
    #[test]
    fn backend_kind_backend_stable(backend in arb_agent_backend()) {
        let kind = ProviderKind::from(backend);
        let back = kind.to_backend();
        // Just verify the result is well-formed (serializes without panic).
        let _json = serde_json::to_string(&back).expect("serialize roundtrip backend");
    }

    /// ProviderKind derived from AgentBackend has a non-empty label.
    #[test]
    fn backend_to_kind_has_valid_label(backend in arb_agent_backend()) {
        let kind = ProviderKind::from(backend);
        let label = kind.label();
        prop_assert!(!label.is_empty());
        prop_assert!(label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }
}
