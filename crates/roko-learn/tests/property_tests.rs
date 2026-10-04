//! Property-based tests for roko-learn core types.

use proptest::prelude::*;
use roko_learn::cascade::types::CascadeStage;
use roko_learn::efficiency::{AgentEfficiencyEvent, PromptSectionMeta, ToolCallMeta};

// ─── CascadeStage serde roundtrip ────────────────────────────────────────────

fn arb_cascade_stage() -> impl Strategy<Value = CascadeStage> {
    prop_oneof![
        Just(CascadeStage::Static),
        Just(CascadeStage::Confidence),
        Just(CascadeStage::Ucb),
    ]
}

proptest! {
    /// CascadeStage serde roundtrip preserves identity.
    #[test]
    fn cascade_stage_serde_roundtrip(stage in arb_cascade_stage()) {
        let json = serde_json::to_string(&stage).expect("serialize");
        let parsed: CascadeStage = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(stage, parsed);
    }

    /// CascadeStage::label() is non-empty and contains no whitespace.
    #[test]
    fn cascade_stage_label_valid(stage in arb_cascade_stage()) {
        let label = stage.label();
        prop_assert!(!label.is_empty(), "label() must not be empty");
        prop_assert!(!label.contains(' '), "label() must not contain spaces: {label:?}");
    }

    /// CascadeStage::Display matches label().
    #[test]
    fn cascade_stage_display_matches_label(stage in arb_cascade_stage()) {
        let displayed = stage.to_string();
        prop_assert_eq!(displayed.as_str(), stage.label());
    }
}

// ─── AgentEfficiencyEvent serde roundtrip ───────────────────────────────────

fn arb_prompt_section() -> impl Strategy<Value = PromptSectionMeta> {
    (
        "[a-z]{1,12}",
        0u64..8192,
        any::<u8>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(name, tokens, priority, was_truncated, was_dropped)| PromptSectionMeta {
                name,
                tokens,
                priority,
                was_truncated,
                was_dropped,
            },
        )
}

fn arb_tool_call() -> impl Strategy<Value = ToolCallMeta> {
    ("[A-Za-z]{1,10}", 0u64..10_000, 0u64..4096, any::<bool>()).prop_map(
        |(tool_name, duration_ms, result_tokens, succeeded)| ToolCallMeta {
            tool_name,
            duration_ms,
            result_tokens,
            succeeded: Some(succeeded),
            advanced_task: false,
            was_redundant: false,
            error_category: None,
        },
    )
}

fn arb_efficiency_event() -> impl Strategy<Value = AgentEfficiencyEvent> {
    (
        "[a-z]{1,8}",
        "[a-z]{1,8}",
        0u64..100_000,
        0u64..100_000,
        0u32..10,
        0u32..10,
        prop::collection::vec(arb_prompt_section(), 0..4),
        prop::collection::vec(arb_tool_call(), 0..4),
    )
        .prop_map(
            |(
                agent_id,
                plan_id,
                input_tokens,
                output_tokens,
                tools_available,
                tools_used,
                prompt_sections,
                tool_calls,
            )| {
                AgentEfficiencyEvent {
                    agent_id,
                    role: "implementer".into(),
                    backend: "anthropic_api".into(),
                    model: "claude-sonnet-4-5".into(),
                    plan_id,
                    task_id: "t1".into(),
                    attempt_id: "a1".into(),
                    input_tokens,
                    output_tokens,
                    reasoning_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_usd: 0.01,
                    cost_usd_without_cache: 0.02,
                    api_equiv_usd: None,
                    price_snapshot_id: None,
                    prompt_sections,
                    total_prompt_tokens: input_tokens,
                    system_prompt_tokens: input_tokens / 2,
                    tools_available,
                    tools_used,
                    tool_calls,
                    wall_time_ms: 5000,
                    duration_ms: 5000,
                    time_to_first_token_ms: 500,
                    was_warm_start: false,
                    iteration: 1,
                    turn_number: 0,
                    is_final_turn: true,
                    gate_passed: Some(true),
                    outcome: "success".into(),
                    gate_errors: Vec::new(),
                    model_used: "claude-sonnet-4-5".into(),
                    frequency: roko_core::OperatingFrequency::Gamma,
                    strategy_attempted: String::new(),
                    timestamp: "2026-01-01T00:00:00Z".into(),
                }
            },
        )
}

proptest! {
    /// AgentEfficiencyEvent serde roundtrip preserves all scalar fields.
    #[test]
    fn efficiency_event_serde_roundtrip(event in arb_efficiency_event()) {
        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: AgentEfficiencyEvent = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(&event.agent_id, &parsed.agent_id);
        prop_assert_eq!(&event.plan_id, &parsed.plan_id);
        prop_assert_eq!(event.input_tokens, parsed.input_tokens);
        prop_assert_eq!(event.output_tokens, parsed.output_tokens);
        prop_assert_eq!(event.tools_available, parsed.tools_available);
        prop_assert_eq!(event.tools_used, parsed.tools_used);
        prop_assert_eq!(event.gate_passed, parsed.gate_passed);
        prop_assert_eq!(event.prompt_sections.len(), parsed.prompt_sections.len());
    }

    /// tool_utilization is always in [0, 1] for any valid tool counts.
    #[test]
    fn tool_utilization_in_unit_range(
        available in 0u32..50,
        used_extra in 0u32..50,
    ) {
        // used must not exceed available.
        let used = used_extra.min(available);
        let mut ev = AgentEfficiencyEvent::default_event();
        ev.tools_available = available;
        ev.tools_used = used;
        let u = ev.tool_utilization();
        prop_assert!(u >= 0.0 && u <= 1.0,
            "tool_utilization = {u} for available={available}, used={used}");
    }

    /// cache_hit_rate is always in [0, 1] for non-zero input tokens.
    #[test]
    fn cache_hit_rate_in_unit_range(
        input_tokens in 1u64..1_000_000,
        cache_fraction_pct in 0u64..=100,
    ) {
        let cache_read = (input_tokens * cache_fraction_pct) / 100;
        let mut ev = AgentEfficiencyEvent::default_event();
        ev.input_tokens = input_tokens;
        ev.cache_read_tokens = cache_read;
        let r = ev.cache_hit_rate();
        prop_assert!(r >= 0.0 && r <= 1.0,
            "cache_hit_rate = {r} for input={input_tokens}, cache_read={cache_read}");
    }

    /// total_tokens is sum of input and output tokens.
    #[test]
    fn total_tokens_is_sum(input in 0u64..500_000, output in 0u64..500_000) {
        let mut ev = AgentEfficiencyEvent::default_event();
        ev.input_tokens = input;
        ev.output_tokens = output;
        prop_assert_eq!(ev.total_tokens(), input + output);
    }
}
