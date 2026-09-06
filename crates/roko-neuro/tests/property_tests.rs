//! Property-based tests for roko-neuro core types.

use proptest::prelude::*;
use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};

// ─── KnowledgeTier ordering properties ──────────────────────────────────────

proptest! {
    /// KnowledgeTier multipliers are strictly ordered: Transient < Working < Consolidated < Persistent.
    #[test]
    fn tier_multipliers_are_strictly_ordered(_dummy in 0u32..1) {
        let transient = KnowledgeTier::Transient.multiplier();
        let working = KnowledgeTier::Working.multiplier();
        let consolidated = KnowledgeTier::Consolidated.multiplier();
        let persistent = KnowledgeTier::Persistent.multiplier();
        prop_assert!(transient < working, "Transient {transient} must be < Working {working}");
        prop_assert!(working < consolidated, "Working {working} must be < Consolidated {consolidated}");
        prop_assert!(consolidated < persistent, "Consolidated {consolidated} must be < Persistent {persistent}");
    }

    /// KnowledgeTier multipliers are always positive.
    #[test]
    fn tier_multipliers_positive(_dummy in 0u32..1) {
        for tier in [
            KnowledgeTier::Transient,
            KnowledgeTier::Working,
            KnowledgeTier::Consolidated,
            KnowledgeTier::Persistent,
        ] {
            prop_assert!(tier.multiplier() > 0.0,
                "{tier:?} multiplier {} must be positive", tier.multiplier());
        }
    }
}

// ─── KnowledgeKind properties ────────────────────────────────────────────────

fn arb_kind() -> impl Strategy<Value = KnowledgeKind> {
    prop_oneof![
        Just(KnowledgeKind::Insight),
        Just(KnowledgeKind::Heuristic),
        Just(KnowledgeKind::AntiKnowledge),
        Just(KnowledgeKind::Warning),
        Just(KnowledgeKind::CausalLink),
        Just(KnowledgeKind::StrategyFragment),
    ]
}

proptest! {
    /// KnowledgeKind serde roundtrip preserves identity.
    #[test]
    fn kind_serde_roundtrip(kind in arb_kind()) {
        let json = serde_json::to_string(&kind).expect("serialize");
        let parsed: KnowledgeKind = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(kind, parsed);
    }

    /// KnowledgeKind::as_str() produces a non-empty string for every variant.
    #[test]
    fn kind_as_str_non_empty(kind in arb_kind()) {
        let s = kind.as_str();
        prop_assert!(!s.is_empty(), "{kind:?}.as_str() must be non-empty");
    }

    /// KnowledgeKind default half-life is always positive.
    #[test]
    fn kind_default_half_life_positive(kind in arb_kind()) {
        let hl = kind.default_half_life_days();
        prop_assert!(hl > 0.0, "{kind:?}.default_half_life_days() = {hl} must be > 0");
    }
}

// ─── KnowledgeEntry serde roundtrip ─────────────────────────────────────────

fn arb_entry() -> impl Strategy<Value = KnowledgeEntry> {
    (
        "[a-z]{1,12}",
        "[a-z ]{1,64}",
        arb_kind(),
        0.0f64..=1.0,
        0.0f64..=1.0,
    )
        .prop_map(|(id, content, kind, confidence, balance)| KnowledgeEntry {
            id,
            content,
            kind,
            confidence,
            balance,
            ..KnowledgeEntry::default()
        })
}

proptest! {
    /// KnowledgeEntry serialization roundtrip preserves key scalar fields.
    #[test]
    fn entry_serde_roundtrip(entry in arb_entry()) {
        let json = serde_json::to_string(&entry).expect("serialize");
        let parsed: KnowledgeEntry = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(&entry.id, &parsed.id);
        prop_assert_eq!(&entry.content, &parsed.content);
        prop_assert_eq!(entry.kind, parsed.kind);
        prop_assert!((entry.confidence - parsed.confidence).abs() < 1e-9);
        prop_assert!((entry.balance - parsed.balance).abs() < 1e-9);
    }

    /// A fresh entry's confirmation_count starts at 0.
    #[test]
    fn fresh_entry_zero_confirmations(entry in arb_entry()) {
        prop_assert_eq!(entry.confirmation_count, 0);
    }
}
