//! Tests for the cascade router.

use super::helpers::*;
use super::persistence::*;
use super::types::*;
use crate::cascade_router::CascadeRouter;
use crate::cfactor::CFactor;
use crate::model_experiment::{ModelExperiment, ModelExperimentStore, ModelVariant};
use crate::model_router::RoutingContext;
use crate::prompt_experiment::ExperimentStatus;
use crate::provider_health::{ErrorClass, ProviderHealthRegistry};
use crate::routing_log::{RoutingDecisionMeta, RoutingLogger};
use async_trait::async_trait;
use chrono::Utc;
use roko_agent::AgentResult;
use roko_agent::gemini::{CodeExecutionResultPart, GroundingMetadata};
use roko_core::agent::AgentRole;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_core::{
    BehavioralState, Body, DaimonPolicy, Kind, OperatingFrequency, Signal, Temperament,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tempfile::tempdir;

fn test_slugs() -> Vec<String> {
    vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
        "claude-opus-4-6".to_string(),
    ]
}

fn default_ctx() -> RoutingContext {
    RoutingContext {
        task_category: TaskCategory::Implementation,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role: AgentRole::Implementer,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy: DaimonPolicy::new(0.5, BehavioralState::Engaged),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

struct StubShadowRunner {
    result: AgentResult,
}

#[async_trait]
impl ShadowModelRunner for StubShadowRunner {
    async fn run_shadow(&self, _prompt: &str, _model_slug: &str) -> AgentResult {
        self.result.clone()
    }
}

fn agent_result(text: &str, success: bool, model: &str, wall_ms: u64) -> AgentResult {
    let output = Signal::builder(Kind::AgentOutput)
        .body(Body::text(text))
        .tag("model", model)
        .build();

    let usage = roko_agent::Usage {
        wall_ms,
        ..Default::default()
    };

    if success {
        AgentResult::ok(output).with_usage(usage)
    } else {
        AgentResult::fail(output).with_usage(usage)
    }
}

// ── Test 1: starts in Static stage ──────────────────────────────────

#[test]
fn starts_in_static_stage() {
    let cascade = CascadeRouter::new(test_slugs());
    assert_eq!(cascade.current_stage(), CascadeStage::Static);
}

#[test]
fn model_slugs_with_availability_marks_configured_and_successful_models() {
    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
        "claude-opus-4-6".to_string(),
    ]);

    assert!(cascade.record_confidence_outcome("claude-haiku-4-5", true));

    let configured = vec!["claude-sonnet-4-5".to_string()];
    let availability = cascade.model_slugs_with_availability(&configured);

    assert_eq!(
        availability,
        vec![
            ("claude-haiku-4-5".to_string(), true),
            ("claude-sonnet-4-5".to_string(), true),
            ("claude-opus-4-6".to_string(), false),
        ]
    );
}

// ── Test 2: static stage uses role table ────────────────────────────

#[test]
fn static_stage_uses_role_table() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let result = cascade.route(&ctx);

    // Implementer has Standard tier -> sonnet
    assert_eq!(result.stage, CascadeStage::Static);
    assert_eq!(result.primary.slug, "claude-sonnet-4-5");
}

// ── Test 3: static stage gives correct fallback ─────────────────────

#[test]
fn static_stage_fallback_chain_for_standard() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let result = cascade.route(&ctx);

    assert_eq!(result.fallback_chain.len(), 2);
    assert_eq!(result.fallback_chain[0].slug, "claude-haiku-4-5");
    assert_eq!(result.fallback_chain[1].slug, "claude-opus-4-6");
    assert_eq!(
        result.context_overflow_fallback.as_ref().unwrap().slug,
        "claude-opus-4-6"
    );
}

#[test]
fn append_routing_log_records_candidates() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let explanation = cascade.explain_route(&ctx, None);
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("routing.jsonl");
    let logger = RoutingLogger::open_creating(&path)
        .expect("logger")
        .with_model_providers(HashMap::from([
            ("claude-haiku-4-5".to_string(), "anthropic".to_string()),
            ("claude-sonnet-4-5".to_string(), "anthropic".to_string()),
            ("claude-opus-4-6".to_string(), "anthropic".to_string()),
        ]));
    let meta = RoutingDecisionMeta {
        trace_id: "trace-123".to_string(),
        task_id: "task-2m14".to_string(),
        requested_model: "claude-sonnet-4-5".to_string(),
        role: "implementer".to_string(),
        task_complexity: "standard".to_string(),
        task_category: "implementation".to_string(),
        routing_stage: explanation.stage.label().to_string(),
        routing_reason: "role_default".to_string(),
    };

    let record = cascade
        .append_routing_log(&logger, &meta, "claude-sonnet-4-5", Some(&explanation))
        .expect("append routing log");

    assert_eq!(record.selected_model, "claude-sonnet-4-5");
    assert!(!record.candidates.is_empty());
    let stored = std::fs::read_to_string(&path).expect("read log");
    let entry: serde_json::Value = serde_json::from_str(stored.trim()).expect("parse json");
    assert_eq!(entry["task_id"], "task-2m14");
}

#[test]
fn experiment_override_for_active_model_experiment() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    let mut store = ModelExperimentStore::default();
    store.register(ModelExperiment {
        experiment_id: "impl-model-ab".into(),
        description: "Override implementer implementation routing".into(),
        role: Some("implementer".into()),
        task_category: Some("implementation".into()),
        variants: vec![ModelVariant {
            id: "override".into(),
            model_key: "override-model".into(),
            slug: "override-model-slug".into(),
            provider: "test-provider".into(),
        }],
        stats: HashMap::new(),
        status: ExperimentStatus::Running,
        winner_id: None,
        min_trials_per_variant: 1,
        min_effect_size: 0.05,
        created_at: "2026-04-11T00:00:00Z".into(),
    });

    let routed = cascade.route_with_experiments(&ctx, &store);

    assert_eq!(routed.primary.slug, "override-model-slug");
    assert!(routed.fallback_chain.is_empty());
    assert_eq!(routed.context_overflow_fallback, None);
    assert_eq!(routed.latency_sla_ms, 30_000);
    assert_eq!(routed.stage, CascadeStage::Static);
}

// ── Test 4: fast tier has no fallback ────────────────────────────────

#[test]
fn fast_tier_no_fallback() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Conductor; // Fast tier

    let result = cascade.route(&ctx);
    assert!(result.fallback_chain.is_empty());
}

// ── Test 5: transitions to Confidence at 50 observations ────────────

#[test]
fn transitions_to_confidence_stage() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    // Feed 50 observations to cross the threshold.
    for _ in 0..50 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);
    let result = cascade.route(&ctx);
    assert_eq!(result.stage, CascadeStage::Confidence);
}

// ── Test 6: transitions to UCB at 200 observations ──────────────────

#[test]
fn transitions_to_ucb_stage() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    // Feed 200 observations.
    for _ in 0..200 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Ucb);
    let result = cascade.route(&ctx);
    assert_eq!(result.stage, CascadeStage::Ucb);
}

#[test]
fn stage_transition_logging() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let before = Utc::now();

    for _ in 0..50 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    }

    let transitions = cascade.stage_transitions();
    assert_eq!(transitions.len(), 1);
    assert_eq!(
        transitions[0],
        StageTransition {
            from: CascadeStage::Static,
            to: CascadeStage::Confidence,
            observations: 50,
            timestamp: transitions[0].timestamp,
        }
    );
    assert!(transitions[0].timestamp >= before);

    for _ in 0..150 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    }

    let transitions = cascade.stage_transitions();
    assert_eq!(transitions.len(), 2);
    assert_eq!(
        transitions[1],
        StageTransition {
            from: CascadeStage::Confidence,
            to: CascadeStage::Ucb,
            observations: 200,
            timestamp: transitions[1].timestamp,
        }
    );
    assert!(transitions[1].timestamp >= transitions[0].timestamp);

    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    cascade.save(&path).unwrap();

    let reloaded = CascadeRouter::load_or_new(&path, test_slugs());
    assert_eq!(reloaded.current_stage(), CascadeStage::Ucb);
    assert_eq!(reloaded.stage_transitions(), transitions);
}

// ── Test 7: confidence stage prefers high-success model ─────────────

#[test]
fn confidence_stage_prefers_high_success_model() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    // Build up observations: sonnet mostly succeeds, haiku mostly fails.
    for i in 0..80 {
        if i < 25 {
            cascade.record_observation(&ctx, "claude-haiku-4-5", 0.2, false);
        } else if i < 50 {
            cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.9, true);
        } else if i < 65 {
            cascade.record_observation(&ctx, "claude-haiku-4-5", 0.2, false);
        } else {
            cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.9, true);
        }
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);

    let result = cascade.route(&ctx);
    // Sonnet should have higher upper bound than haiku
    // (sonnet: 25/25 = 100%, haiku: 0/40 = 0%)
    assert_eq!(
        result.primary.slug, "claude-sonnet-4-5",
        "confidence stage should prefer the high-pass-rate model"
    );
}

// ── Test 7b: low affect confidence biases toward stronger model ──────

#[test]
fn low_affect_confidence_prefers_opus_over_sonnet() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();

    for _ in 0..30 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.9, true);
    }
    for _ in 0..15 {
        cascade.record_observation(&ctx, "claude-opus-4-6", 0.9, true);
    }
    for _ in 0..5 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.1, false);
    }
    for _ in 0..5 {
        cascade.record_observation(&ctx, "claude-opus-4-6", 0.1, false);
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);

    ctx.daimon_policy.affect_confidence = 0.2;
    let low_confidence = cascade.route(&ctx);
    assert_eq!(
        low_confidence.primary.slug, "claude-opus-4-6",
        "low affect confidence should bias toward the stronger premium model"
    );

    ctx.daimon_policy.affect_confidence = 0.9;
    let high_confidence = cascade.route(&ctx);
    // High confidence allows routing to cheaper models
    assert!(
        ["claude-haiku-4-5", "claude-sonnet-4-5"].contains(&high_confidence.primary.slug.as_str()),
        "high confidence should allow cheaper model, got: {}",
        high_confidence.primary.slug
    );
}

#[test]
fn behavioral_state_biases_static_routing() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();

    ctx.daimon_policy.behavioral_state = BehavioralState::Struggling;
    let struggling = cascade.route(&ctx);
    assert_eq!(struggling.primary.slug, "claude-opus-4-6");

    ctx.daimon_policy.behavioral_state = BehavioralState::Coasting;
    let coasting = cascade.route(&ctx);
    assert_eq!(coasting.primary.slug, "claude-haiku-4-5");
}

#[test]
fn conductor_load_biases_static_routing_toward_cheaper_models() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();

    ctx.conductor_load = 0.75;
    ctx.active_agents = 4;
    ctx.ready_queue_depth = 3;

    let routed = cascade.route(&ctx);
    assert_eq!(routed.primary.slug, "claude-haiku-4-5");
}

#[test]
fn critical_conductor_load_can_drop_two_tiers() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Architect;
    ctx.conductor_load = 0.95;
    ctx.active_agents = 6;
    ctx.ready_queue_depth = 5;
    ctx.max_queue_wait_hours = 2.0;

    let routed = cascade.route(&ctx);
    assert_eq!(routed.primary.slug, "claude-haiku-4-5");
}

#[test]
fn cache_affinity_bonus() {
    let cascade = CascadeRouter::new(vec![
        "claude-sonnet-4-5".to_string(),
        "claude-sonnet-4-6".to_string(),
    ]);
    let mut ctx = default_ctx();

    for _ in 0..80 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    }
    for _ in 0..10 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.2, false);
    }
    for _ in 0..82 {
        cascade.record_observation(&ctx, "claude-sonnet-4-6", 0.8, true);
    }
    for _ in 0..8 {
        cascade.record_observation(&ctx, "claude-sonnet-4-6", 0.2, false);
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);

    let no_affinity = cascade.route(&ctx);
    assert_eq!(no_affinity.primary.slug, "claude-sonnet-4-6");

    ctx.previous_model = Some("claude-sonnet-4-5".to_string());
    let with_affinity = cascade.route(&ctx);
    assert_eq!(with_affinity.primary.slug, "claude-sonnet-4-5");
}

#[test]
fn routing_hysteresis_keeps_incumbent_below_threshold() {
    let candidates = vec![
        ("claude-sonnet-4-5".to_string(), 0.82),
        ("claude-sonnet-4-6".to_string(), 0.91),
    ];

    assert_eq!(
        select_with_hysteresis(&candidates, Some("claude-sonnet-4-5")),
        "claude-sonnet-4-5"
    );
}

#[test]
fn routing_hysteresis_switches_at_threshold() {
    let candidates = vec![
        ("claude-sonnet-4-5".to_string(), 0.82),
        ("claude-sonnet-4-6".to_string(), 0.92),
    ];

    assert_eq!(
        select_with_hysteresis(&candidates, Some("claude-sonnet-4-5")),
        "claude-sonnet-4-6"
    );
}

// ── Test 7c: health-aware routing skips unhealthy providers ─────────

#[test]
fn cascade_health_aware_excludes_unhealthy_provider_models() {
    let cascade = CascadeRouter::new(vec![
        "claude-sonnet-4-5".to_string(),
        "claude-opus-4-6".to_string(),
    ]);
    let ctx = default_ctx();

    // Push the router into UCB so the candidate-aware LinUCB path is exercised.
    for _ in 0..200 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 1.0, true);
    }
    assert_eq!(cascade.current_stage(), CascadeStage::Ucb);
    assert_eq!(cascade.route(&ctx).primary.slug, "claude-sonnet-4-5");

    let health = ProviderHealthRegistry::new();
    for _ in 0..3 {
        health.record_failure("anthropic", ErrorClass::ServerError);
    }

    let mut model_providers = HashMap::new();
    model_providers.insert("claude-sonnet-4-5".to_string(), "anthropic".to_string());
    model_providers.insert("claude-opus-4-6".to_string(), "openai".to_string());

    let routed = cascade.route_with_health(&ctx, &health, &model_providers);
    assert_eq!(
        routed.primary.slug, "claude-opus-4-6",
        "unhealthy providers should be excluded from cascade selection"
    );
}

// ── Test 8: stage labels are correct ────────────────────────────────

#[test]
fn stage_labels() {
    assert_eq!(CascadeStage::Static.label(), "static");
    assert_eq!(CascadeStage::Confidence.label(), "confidence");
    assert_eq!(CascadeStage::Ucb.label(), "ucb");
}

// ── Test 9: frequency routing follows the frequency policy ─────────

#[test]
fn frequency_routing_uses_expected_policy() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    assert_eq!(
        cascade.select_for_frequency(OperatingFrequency::Gamma, Some(&ctx), None, None),
        None
    );

    let theta = cascade
        .select_for_frequency(OperatingFrequency::Theta, Some(&ctx), None, None)
        .expect("theta should route");
    assert_eq!(theta.slug, "claude-sonnet-4-5");

    let delta = cascade
        .select_for_frequency(OperatingFrequency::Delta, Some(&ctx), None, None)
        .expect("delta should route");
    assert_eq!(delta.slug, "claude-opus-4-6");
}

#[test]
fn high_cfactor_prefers_cheapest_model() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let cfactor = CFactor {
        overall: 0.9,
        ..CFactor::default()
    };

    let selected = cascade
        .select_for_frequency(
            OperatingFrequency::Theta,
            Some(&ctx),
            Some(&cfactor),
            Some("Implementer"),
        )
        .expect("theta should route");

    assert_eq!(selected.slug, "claude-haiku-4-5");
}

#[test]
fn low_cfactor_prefers_strongest_model() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let cfactor = CFactor {
        overall: 0.2,
        ..CFactor::default()
    };

    let selected = cascade
        .select_for_frequency(
            OperatingFrequency::Theta,
            Some(&ctx),
            Some(&cfactor),
            Some("Implementer"),
        )
        .expect("theta should route");

    assert_eq!(selected.slug, "claude-opus-4-6");
}

#[test]
fn strongest_model_falls_back_to_best_available_slug() {
    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);

    assert_eq!(cascade.strongest_model().slug, "claude-sonnet-4-5");
}

// ── Test 11: observation count is consistent ────────────────────────

#[test]
fn observation_count_tracks_correctly() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    assert_eq!(cascade.total_observations(), 0);

    cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    cascade.record_observation(&ctx, "claude-haiku-4-5", 0.3, false);
    cascade.record_observation(&ctx, "claude-opus-4-6", 0.9, true);

    assert_eq!(cascade.total_observations(), 3);
}

// ── Test 12: confidence snapshot tracks trials ──────────────────────

#[test]
fn confidence_snapshot_accurate() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.5, false);
    cascade.record_observation(&ctx, "claude-haiku-4-5", 0.9, true);

    let snap = cascade.confidence_snapshot();
    assert_eq!(snap.get("claude-sonnet-4-5"), Some(&(2, 1)));
    assert_eq!(snap.get("claude-haiku-4-5"), Some(&(1, 1)));
}

// ── Test 11: latency SLA varies by tier ─────────────────────────────

#[test]
fn latency_sla_by_tier() {
    let cascade = CascadeRouter::new(test_slugs());

    let mut ctx = default_ctx();
    ctx.role = AgentRole::Conductor; // Fast
    let fast = cascade.route(&ctx);

    ctx.role = AgentRole::Implementer; // Standard
    let standard = cascade.route(&ctx);

    ctx.role = AgentRole::Architect; // Premium
    let premium = cascade.route(&ctx);

    assert!(fast.latency_sla_ms < standard.latency_sla_ms);
    assert!(standard.latency_sla_ms < premium.latency_sla_ms);
}

// ── Test 12: stage_for_observations boundaries ──────────────────────

#[test]
fn stage_boundaries() {
    assert_eq!(stage_for_observations(0), CascadeStage::Static);
    assert_eq!(stage_for_observations(49), CascadeStage::Static);
    assert_eq!(stage_for_observations(50), CascadeStage::Confidence);
    assert_eq!(stage_for_observations(199), CascadeStage::Confidence);
    assert_eq!(stage_for_observations(200), CascadeStage::Ucb);
    assert_eq!(stage_for_observations(1000), CascadeStage::Ucb);
}

// ── Test 13: model_stats pass_rate computation ──────────────────────

#[test]
fn model_stats_pass_rate() {
    let mut s = ModelStats::default();
    assert!((s.pass_rate() - 0.0).abs() < f64::EPSILON);

    s.trials = 10;
    s.successes = 7;
    assert!((s.pass_rate() - 0.7).abs() < f64::EPSILON);
}

// ── Test 14: confidence width shrinks with more data ────────────────

#[test]
fn confidence_width_shrinks() {
    let s10 = ModelStats {
        trials: 10,
        successes: 7,
        ..ModelStats::default()
    };
    let s100 = ModelStats {
        trials: 100,
        successes: 70,
        ..ModelStats::default()
    };
    let s1000 = ModelStats {
        trials: 1000,
        successes: 700,
        ..ModelStats::default()
    };

    assert!(s10.confidence_width() > s100.confidence_width());
    assert!(s100.confidence_width() > s1000.confidence_width());
}

// ── Test 15: premium role uses opus in static stage ─────────────────

#[test]
fn premium_role_gets_opus() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Architect; // Premium tier

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "claude-opus-4-6");
    assert_eq!(result.fallback_chain[0].slug, "claude-sonnet-4-5");
    assert_eq!(result.fallback_chain[1].slug, "claude-haiku-4-5");
    assert_eq!(result.context_overflow_fallback, None);
}

#[test]
fn fallback_chain_tries_each_model_in_order() {
    let cascade = CascadeModel {
        primary: roko_core::agent::ModelSpec::from_slug("primary-model"),
        fallback_chain: vec![
            roko_core::agent::ModelSpec::from_slug("fallback-1"),
            roko_core::agent::ModelSpec::from_slug("fallback-2"),
            roko_core::agent::ModelSpec::from_slug("fallback-3"),
        ],
        context_overflow_fallback: Some(roko_core::agent::ModelSpec::from_slug("larger-context")),
        latency_sla_ms: 30_000,
        stage: CascadeStage::Static,
    };

    assert_eq!(cascade.model_for_attempt(0).unwrap().slug, "primary-model");
    assert_eq!(cascade.model_for_attempt(1).unwrap().slug, "fallback-1");
    assert_eq!(cascade.model_for_attempt(2).unwrap().slug, "fallback-2");
    assert_eq!(cascade.model_for_attempt(3).unwrap().slug, "fallback-3");
    assert!(cascade.model_for_attempt(4).is_none());
}

#[test]
fn error_specific_fallback_routes_by_error_type() {
    use roko_agent::provider::ProviderError;

    let cascade = CascadeModel {
        primary: roko_core::agent::ModelSpec::from_slug("gpt-5"),
        fallback_chain: vec![
            roko_core::agent::ModelSpec::from_slug("glm-5.1"),
            roko_core::agent::ModelSpec::from_slug("claude-sonnet-4-5"),
            roko_core::agent::ModelSpec::from_slug("ollama/llama3"),
        ],
        context_overflow_fallback: Some(roko_core::agent::ModelSpec::from_slug("claude-opus-4-6")),
        latency_sla_ms: 30_000,
        stage: CascadeStage::Static,
    };

    assert_eq!(
        cascade
            .fallback_for_error(&ProviderError::ContextOverflow)
            .unwrap()
            .slug,
        "claude-opus-4-6"
    );
    assert_eq!(
        cascade
            .fallback_for_error(&ProviderError::RateLimit {
                retry_after_ms: Some(1_000),
            })
            .unwrap()
            .slug,
        "claude-sonnet-4-5"
    );
    assert_eq!(
        cascade
            .fallback_for_error(&ProviderError::ServerError(503))
            .unwrap()
            .slug,
        "glm-5.1"
    );
}

// ── Test 16: display impl for CascadeStage ──────────────────────────

#[test]
fn cascade_stage_display() {
    assert_eq!(format!("{}", CascadeStage::Static), "static");
    assert_eq!(format!("{}", CascadeStage::Ucb), "ucb");
}

// ── Test 17: custom role table ──────────────────────────────────────

#[test]
fn custom_role_table() {
    let mut table = HashMap::new();
    table.insert(AgentRole::Implementer, "gpt-5".to_string());

    let cascade = CascadeRouter::new(test_slugs()).with_role_table(table);
    let ctx = default_ctx();
    let result = cascade.route(&ctx);

    assert_eq!(result.primary.slug, "gpt-5");
}

#[test]
fn version_change_detection_detects_glm_upgrade() {
    let changes = detect_version_changes(&["glm-5".to_string()], &["glm-5.1".to_string()]);

    assert!(changes.contains(&VersionChange::Upgraded {
        old: "glm-5".to_string(),
        new: "glm-5.1".to_string(),
    }));
}

#[test]
fn version_change_detection_transfers_weighted_stats_on_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let snapshot = CascadeSnapshot {
        model_slugs: vec!["glm-5".to_string()],
        confidence_stats: HashMap::from([(
            "glm-5".to_string(),
            PersistedModelStats {
                trials: 10,
                successes: 6,
                ..Default::default()
            },
        )]),
        total_observations: 10,
        role_table: HashMap::new(),
        stage_transitions: vec![],
        linucb_state: None,
        pareto_frontier: Vec::new(),
        category_stats: HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();

    let loaded = CascadeRouter::load_or_new(&path, vec!["glm-5.1".to_string()]);
    let stats = loaded.confidence_snapshot();

    assert_eq!(stats.get("glm-5.1"), Some(&(5, 3)));
    assert!(!stats.contains_key("glm-5"));
    assert_eq!(loaded.total_observations(), 10);
}

#[test]
fn version_change_detection_remaps_role_table_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let snapshot = CascadeSnapshot {
        model_slugs: vec!["glm-5".to_string()],
        confidence_stats: HashMap::new(),
        total_observations: 0,
        role_table: HashMap::from([(AgentRole::Implementer, "glm-5".to_string())]),
        stage_transitions: vec![],
        linucb_state: None,
        pareto_frontier: Vec::new(),
        category_stats: HashMap::new(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();

    let loaded = CascadeRouter::load_or_new(&path, vec!["glm-5.1".to_string()]);
    let routed = loaded.route(&default_ctx());

    assert_eq!(routed.primary.slug, "glm-5.1");
}

#[test]
fn cascade_router_kimi_selects_kimi_in_static_stage() {
    let cascade = CascadeRouter::new(vec!["kimi-k2.5".to_string()]);
    let ctx = default_ctx();

    let result = cascade.route(&ctx);
    assert_eq!(result.stage, CascadeStage::Static);
    assert_eq!(result.primary.slug, "kimi-k2.5");
}

#[test]
fn cascade_gemini_routes_configured_fast_standard_and_premium_models() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-flash-lite".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-pro".to_string(),
        "gemini-3.1-pro-preview".to_string(),
    ]);
    let mut ctx = default_ctx();

    ctx.role = AgentRole::Conductor;
    let fast = cascade.route(&ctx);
    assert_eq!(fast.primary.slug, "gemini-2.5-flash-lite");
    assert!(fast.fallback_chain.is_empty());

    ctx.role = AgentRole::Implementer;
    let standard = cascade.route(&ctx);
    assert_eq!(standard.primary.slug, "gemini-2.5-flash");
    assert_eq!(
        standard
            .fallback_chain
            .first()
            .expect("standard fallback")
            .slug,
        "gemini-2.5-flash-lite"
    );

    ctx.role = AgentRole::Architect;
    let premium = cascade.route(&ctx);
    assert_eq!(premium.primary.slug, "gemini-3.1-pro-preview");
    assert_eq!(
        premium
            .fallback_chain
            .first()
            .expect("premium fallback")
            .slug,
        "gemini-2.5-flash"
    );
}

#[test]
fn cascade_gemini_prefers_opus_for_premium_when_available() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-flash-lite".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-pro".to_string(),
        "gemini-3.1-pro-preview".to_string(),
        "claude-opus-4-6".to_string(),
    ]);
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Architect;

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "claude-opus-4-6");
}

#[test]
fn cascade_gemini_matches_openrouter_slug_families() {
    let cascade = CascadeRouter::new(vec![
        "google/gemini-2.5-flash-lite".to_string(),
        "google/gemini-2.5-flash".to_string(),
        "google/gemini-3.1-pro-preview".to_string(),
    ]);
    let mut ctx = default_ctx();

    ctx.role = AgentRole::Conductor;
    assert_eq!(
        cascade.route(&ctx).primary.slug,
        "google/gemini-2.5-flash-lite"
    );

    ctx.role = AgentRole::Implementer;
    assert_eq!(cascade.route(&ctx).primary.slug, "google/gemini-2.5-flash");

    ctx.role = AgentRole::Architect;
    assert_eq!(
        cascade.route(&ctx).primary.slug,
        "google/gemini-3.1-pro-preview"
    );
}

#[test]
fn routing_context_thinking_high_prefers_thinking_models() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-flash-lite".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-pro".to_string(),
    ]);
    let mut ctx = default_ctx();
    ctx.complexity = TaskComplexityBand::Complex;
    ctx.thinking_level = Some("high".to_string());

    let result = cascade.route(&ctx);
    assert_ne!(result.primary.slug, "gemini-2.5-flash-lite");
    assert!(model_supports_thinking(&result.primary.slug));
}

#[test]
fn routing_context_thinking_minimal_prefers_non_thinking_models() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-flash-lite".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-pro".to_string(),
    ]);
    let mut ctx = default_ctx();
    ctx.thinking_level = Some("minimal".to_string());

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "gemini-2.5-flash-lite");
}

#[test]
fn conservative_temperament_biases_static_routing_toward_stronger_tiers() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx().with_temperament(Temperament::Conservative);

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "claude-opus-4-6");
}

#[test]
fn aggressive_temperament_biases_static_routing_toward_cheaper_tiers() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx().with_temperament(Temperament::Aggressive);

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "claude-haiku-4-5");
}

#[tokio::test]
async fn shadow_evaluate_records_observation_for_passing_free_model() {
    let primary = agent_result(
        "```rust\nfn answer() -> u32 { 42 }\n```",
        true,
        "gemini-2.5-pro",
        900,
    );
    let shadow = agent_result(
        "```rust\nfn answer() -> u32 { 42 }\n```",
        true,
        "gemini-2.5-flash-lite",
        120,
    );
    let mut cascade = CascadeRouter::new(vec![
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash-lite".to_string(),
    ])
    .with_free_tier_shadow_runner(Arc::new(StubShadowRunner { result: shadow }));

    cascade
        .shadow_evaluate(
            "Implement a Rust function that returns 42 and include code.",
            &primary,
            "gemini-2.5-flash-lite",
        )
        .await;

    let stats = cascade.observation_snapshot();
    let flash_lite = stats
        .get("gemini-2.5-flash-lite")
        .expect("flash-lite stats");

    assert_eq!(flash_lite.trials, 1);
    assert_eq!(flash_lite.successes, 1);
    assert_eq!(cascade.total_observations(), 1);
}

#[tokio::test]
async fn shadow_evaluate_records_failed_observation_when_shadow_output_is_weaker() {
    let primary = agent_result(
        "```rust\nfn answer() -> u32 { 42 }\n```\nAdd a unit test.",
        true,
        "gemini-2.5-pro",
        900,
    );
    let weak_shadow = agent_result("done", true, "gemini-2.5-flash-lite", 120);
    let mut cascade = CascadeRouter::new(vec![
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash-lite".to_string(),
    ])
    .with_free_tier_shadow_runner(Arc::new(StubShadowRunner {
        result: weak_shadow,
    }));

    cascade
        .shadow_evaluate(
            "Implement a Rust function and add tests for it.",
            &primary,
            "gemini-2.5-flash-lite",
        )
        .await;

    let stats = cascade.observation_snapshot();
    let flash_lite = stats
        .get("gemini-2.5-flash-lite")
        .expect("flash-lite stats");

    assert_eq!(flash_lite.trials, 1);
    assert_eq!(flash_lite.successes, 0);
}

#[tokio::test]
async fn shadow_evaluate_shifts_router_toward_free_model() {
    let prompt = "Implement a Rust function that parses a config string into a struct.";
    let primary = agent_result(
        "```rust\nstruct Config { enabled: bool }\nfn parse_config(input: &str) -> Config { Config { enabled: input == \"on\" } }\n```",
        true,
        "gemini-2.5-pro",
        900,
    );
    let shadow = agent_result(
        "```rust\nstruct Config { enabled: bool }\nfn parse_config(input: &str) -> Config { Config { enabled: input.trim() == \"on\" } }\n```",
        true,
        "gemini-2.5-flash-lite",
        110,
    );
    let ctx = infer_shadow_routing_context(prompt, &primary);
    let mut route_ctx = ctx.clone();
    route_ctx.previous_model = None;
    let mut cascade = CascadeRouter::new(vec![
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash-lite".to_string(),
    ])
    .with_free_tier_shadow_runner(Arc::new(StubShadowRunner { result: shadow }));

    for _ in 0..34 {
        cascade.record_observation(&ctx, "gemini-2.5-pro", 0.9, true);
    }
    for _ in 0..6 {
        cascade.record_observation(&ctx, "gemini-2.5-pro", 0.0, false);
    }
    for _ in 0..5 {
        cascade.record_observation(&ctx, "gemini-2.5-flash-lite", 0.8, true);
    }
    for _ in 0..5 {
        cascade.record_observation(&ctx, "gemini-2.5-flash-lite", 0.0, false);
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);
    assert_eq!(cascade.route(&route_ctx).primary.slug, "gemini-2.5-pro");

    for _ in 0..40 {
        cascade
            .shadow_evaluate(prompt, &primary, "gemini-2.5-flash-lite")
            .await;
    }

    let stats = cascade.observation_snapshot();
    let flash_lite = stats
        .get("gemini-2.5-flash-lite")
        .expect("flash-lite stats");
    assert_eq!(flash_lite.trials, 50);
    assert_eq!(flash_lite.successes, 45);
    assert_eq!(
        cascade.route(&route_ctx).primary.slug,
        "gemini-2.5-flash-lite"
    );
}

#[test]
fn shadow_routing_context_keeps_affect_bias_neutral() {
    let prompt = "Refactor a parser and add regression tests.";
    let primary = agent_result("done", false, "claude-sonnet-4-5", 800);
    let ctx = infer_shadow_routing_context(prompt, &primary);

    assert_eq!(ctx.daimon_policy.behavioral_state, BehavioralState::Engaged);
    assert!((ctx.daimon_policy.affect_confidence - 0.5).abs() < f64::EPSILON);
    assert!(ctx.has_prior_failure);
}

#[test]
fn gemini_observations_include_quality_and_cost_signals() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-pro".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let ctx = default_ctx();

    assert!(cascade.record_gemini_observation(
        &ctx,
        "gemini-2.5-pro",
        0.92,
        true,
        GeminiObservation {
            input_tokens: 250_000,
            output_tokens: 1_024,
            thinking_tokens: Some(64),
            cached_tokens: Some(512),
            grounding_query_count: 3,
            code_execution_success_count: 2,
            code_execution_failure_count: 1,
            context_tier: GeminiContextTier::Over200k,
        },
    ));

    let stats = cascade.observation_snapshot();
    let gemini = stats.get("gemini-2.5-pro").expect("gemini stats");

    assert_eq!(gemini.trials, 1);
    assert_eq!(gemini.successes, 1);
    assert_eq!(gemini.gemini_requests, 1);
    assert_eq!(gemini.total_gemini_thinking_tokens, 64);
    assert!((gemini.avg_gemini_thinking_tokens_per_response - 64.0).abs() < 1e-9);
    assert_eq!(gemini.total_gemini_cached_tokens, 512);
    assert!((gemini.avg_gemini_cached_tokens_per_response - 512.0).abs() < 1e-9);
    assert_eq!(gemini.total_gemini_grounding_queries, 3);
    assert!((gemini.avg_gemini_grounding_queries_per_response - 3.0).abs() < 1e-9);
    assert_eq!(gemini.gemini_code_execution_successes, 2);
    assert_eq!(gemini.gemini_code_execution_failures, 1);
    assert!((gemini.gemini_code_execution_success_rate - (2.0 / 3.0)).abs() < 1e-9);
    assert_eq!(gemini.gemini_context_window_le_200k_requests, 0);
    assert_eq!(gemini.gemini_context_window_gt_200k_requests, 1);
}

#[test]
fn gemini_observations_from_metadata_extract_router_signals() {
    use roko_agent::gemini::GeminiMetadata;

    let metadata = GeminiMetadata {
        grounding_metadata: Some(GroundingMetadata {
            web_search_queries: Some(vec![
                "Rust cargo metadata".to_string(),
                "Rust cargo workspace".to_string(),
            ]),
            grounding_chunks: None,
            grounding_supports: None,
            search_entry_point: None,
        }),
        code_execution_results: vec![
            CodeExecutionResultPart {
                outcome: "OUTCOME_OK".to_string(),
                output: "passed".to_string(),
            },
            CodeExecutionResultPart {
                outcome: "OUTCOME_ERROR".to_string(),
                output: "failed".to_string(),
            },
        ],
        thinking_tokens: Some(11),
        cached_tokens: Some(80),
        safety_ratings: Vec::new(),
    };

    let observation = GeminiObservation::from_metadata(&metadata, 240_000, 512);

    assert_eq!(observation.thinking_tokens, Some(11));
    assert_eq!(observation.cached_tokens, Some(80));
    assert_eq!(observation.grounding_query_count, 2);
    assert_eq!(observation.code_execution_success_count, 1);
    assert_eq!(observation.code_execution_failure_count, 1);
    assert_eq!(observation.context_tier, GeminiContextTier::Over200k);
}

#[test]
fn gemini_observations_persist_across_save_and_load() {
    let cascade = CascadeRouter::new(vec![
        "gemini-2.5-flash".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let ctx = default_ctx();

    assert!(cascade.record_gemini_observation(
        &ctx,
        "gemini-2.5-flash",
        0.8,
        true,
        GeminiObservation {
            input_tokens: 120_000,
            output_tokens: 600,
            thinking_tokens: Some(21),
            cached_tokens: Some(144),
            grounding_query_count: 1,
            code_execution_success_count: 1,
            code_execution_failure_count: 0,
            context_tier: GeminiContextTier::UpTo200k,
        },
    ));
    assert!(cascade.record_gemini_observation(
        &ctx,
        "gemini-2.5-flash",
        0.0,
        false,
        GeminiObservation {
            input_tokens: 260_000,
            output_tokens: 700,
            thinking_tokens: Some(34),
            cached_tokens: Some(32),
            grounding_query_count: 4,
            code_execution_success_count: 0,
            code_execution_failure_count: 2,
            context_tier: GeminiContextTier::Over200k,
        },
    ));

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cascade-router.json");
    cascade.save(&path).expect("save cascade router");

    let reloaded = CascadeRouter::load_or_new(
        &path,
        vec![
            "gemini-2.5-flash".to_string(),
            "claude-sonnet-4-5".to_string(),
        ],
    );
    let stats = reloaded.observation_snapshot();
    let gemini = stats.get("gemini-2.5-flash").expect("gemini stats");

    assert_eq!(gemini.gemini_requests, 2);
    assert_eq!(gemini.total_gemini_thinking_tokens, 55);
    assert_eq!(gemini.total_gemini_cached_tokens, 176);
    assert_eq!(gemini.total_gemini_grounding_queries, 5);
    assert_eq!(gemini.gemini_code_execution_successes, 1);
    assert_eq!(gemini.gemini_code_execution_failures, 2);
    assert!((gemini.gemini_code_execution_success_rate - (1.0 / 3.0)).abs() < 1e-9);
    assert_eq!(gemini.gemini_context_window_le_200k_requests, 1);
    assert_eq!(gemini.gemini_context_window_gt_200k_requests, 1);
}

// ── Test 18: UCB stage uses linucb selection ────────────────────────

#[test]
fn ucb_stage_uses_trained_linucb() {
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();

    // Train haiku as the best arm with many observations.
    for _ in 0..250 {
        cascade.record_observation(&ctx, "claude-haiku-4-5", 1.0, true);
        // Give some data to other arms too so LinUCB has seen them.
        if cascade.total_observations() % 10 == 0 {
            cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.1, false);
            cascade.record_observation(&ctx, "claude-opus-4-6", 0.1, false);
        }
    }

    assert_eq!(cascade.current_stage(), CascadeStage::Ucb);
    let result = cascade.route(&ctx);
    // LinUCB should prefer the highly-rewarded arm
    assert_eq!(result.primary.slug, "claude-haiku-4-5");
}

#[test]
fn record_confidence_outcome_updates_model_statistics_without_linucb_observations() {
    let cascade = CascadeRouter::new(test_slugs());

    assert!(cascade.record_confidence_outcome("claude-sonnet-4-5", true));
    // record_confidence_outcome only updates confidence stats, NOT the
    // LinUCB bandit, so the LinUCB observation counter stays at 0.
    assert_eq!(cascade.total_observations(), 0);

    let stats = cascade.confidence_snapshot();
    assert_eq!(stats.get("claude-sonnet-4-5"), Some(&(1, 1)));
}

#[test]
fn override_without_context_records_multi_objective() {
    let cascade = CascadeRouter::new(test_slugs());

    assert!(
        roko_agent::model_call_service::ForceBackendOverrideRecorder::record_override_outcome(
            &cascade,
            "claude-sonnet-4-5",
            true,
            0.1,
            30_000,
        )
    );

    // The trait impl now delegates to the full multi-objective path with a
    // default RoutingContext, so LinUCB observations should be recorded.
    assert!(
        cascade.total_observations() >= 1,
        "override via trait should update LinUCB observations"
    );
    assert_eq!(
        cascade.confidence_snapshot().get("claude-sonnet-4-5"),
        Some(&(1, 1))
    );
}

#[test]
fn failed_override_lowers_success_rate() {
    // bug-f68404: a failed override counts as a failed trial and earns no
    // LinUCB reward, and each override carries only the dampened weight of
    // an observation.
    let cascade = CascadeRouter::new(test_slugs());
    let ctx = default_ctx();
    let slug = "claude-sonnet-4-5";
    let arm_of = |cascade: &CascadeRouter| {
        cascade
            .linucb()
            .arm_stats()
            .into_iter()
            .find(|arm| arm.slug == slug)
            .expect("arm for the slug")
    };

    assert!(cascade.record_override_outcome(slug, &ctx, true, 0.1, 30_000, None));
    let after_success = arm_of(&cascade);
    assert!(
        after_success.b_vector[16] > 0.0,
        "a successful override earns a reward"
    );

    assert!(cascade.record_override_outcome(slug, &ctx, false, 0.05, 1_000, None));
    assert_eq!(
        cascade.confidence_snapshot()[slug],
        (2, 1),
        "the failed override lowers the success rate"
    );
    let after_failure = arm_of(&cascade);
    assert_eq!(
        after_failure.b_vector, after_success.b_vector,
        "a failed override earns no reward, however cheap and fast"
    );
    // The bias feature is 1, so each update adds its weight to A[16][16].
    assert!(
        (after_failure.a_matrix[16][16] - 2.0).abs() < 1e-12,
        "each override carries half an observation's weight"
    );
    assert_eq!(cascade.total_observations(), 2);
}

#[test]
fn a_replayed_failure_earns_zero_reward() {
    // bug-3ea1f5: a WAL entry journaled before the reward-0 rule carries a
    // failure's cost and latency reward; replay applies 0 instead.
    let cascade = CascadeRouter::new(test_slugs());
    let slug = "claude-sonnet-4-5";
    let idx = cascade
        .model_index_for_slug(slug)
        .expect("slug in the router");

    cascade.replay_observation(slug, &default_ctx().to_features(), idx, 0.45, false);

    assert_eq!(cascade.confidence_snapshot()[slug], (1, 0));
    let arm = cascade
        .linucb()
        .arm_stats()
        .into_iter()
        .find(|arm| arm.slug == slug)
        .expect("arm for the slug");
    assert_eq!(arm.observations, 1, "the failure reached LinUCB");
    assert!(
        arm.b_vector.iter().all(|b| *b == 0.0),
        "a replayed failure earns no reward"
    );
}

#[test]
fn perplexity_observations_include_citations_latency_and_total_cost() {
    let cascade = CascadeRouter::new(vec![
        "sonar-pro".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Researcher;
    ctx.task_category = TaskCategory::Research;

    assert!(cascade.record_perplexity_observation(
        &ctx,
        "sonar-pro",
        0.95,
        true,
        PerplexityObservation {
            citation_count: 6,
            search_latency_ms: 1_200,
            input_tokens: 1_000,
            output_tokens: 500,
        },
    ));

    let stats = cascade.observation_snapshot();
    let sonar = stats.get("sonar-pro").expect("sonar-pro stats");

    assert_eq!(sonar.trials, 1);
    assert_eq!(sonar.successes, 1);
    assert_eq!(sonar.total_citations, 6);
    assert!((sonar.avg_citations_per_response - 6.0).abs() < 1e-9);
    assert_eq!(sonar.total_search_latency_ms, 1_200);
    assert!((sonar.avg_search_latency_ms - 1_200.0).abs() < 1e-9);
    assert_eq!(sonar.perplexity_requests, 1);
    assert!((sonar.total_cost_usd - 0.0245).abs() < 1e-9);
    assert!((sonar.avg_cost_usd - 0.0245).abs() < 1e-9);
}

#[test]
fn perplexity_observations_persist_across_save_and_load() {
    let cascade = CascadeRouter::new(vec!["sonar".to_string(), "claude-sonnet-4-5".to_string()]);
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Researcher;
    ctx.task_category = TaskCategory::Research;

    assert!(cascade.record_perplexity_observation(
        &ctx,
        "sonar",
        0.9,
        true,
        PerplexityObservation {
            citation_count: 3,
            search_latency_ms: 900,
            input_tokens: 2_000,
            output_tokens: 1_000,
        },
    ));

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cascade-router.json");
    cascade.save(&path).expect("save cascade router");

    let reloaded = CascadeRouter::load_or_new(
        &path,
        vec!["sonar".to_string(), "claude-sonnet-4-5".to_string()],
    );
    let stats = reloaded.observation_snapshot();
    let sonar = stats.get("sonar").expect("sonar stats");

    assert_eq!(sonar.total_citations, 3);
    assert_eq!(sonar.total_search_latency_ms, 900);
    assert_eq!(sonar.perplexity_requests, 1);
    assert!((sonar.total_cost_usd - 0.008).abs() < 1e-9);
}

// ── cascade_perplexity: Researcher routes to sonar-pro ───────────────

#[test]
fn cascade_perplexity_researcher_routes_to_sonar_pro() {
    let slugs = vec![
        "sonar-pro".to_string(),
        "sonar".to_string(),
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ];
    let cascade = CascadeRouter::new(slugs);
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Researcher;
    ctx.task_category = TaskCategory::Research;

    let result = cascade.route(&ctx);
    assert_eq!(result.stage, CascadeStage::Static);
    assert_eq!(result.primary.slug, "sonar-pro");
}

#[test]
fn cascade_perplexity_research_category_biases_any_role() {
    let slugs = vec![
        "sonar-pro".to_string(),
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ];
    let cascade = CascadeRouter::new(slugs);
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Implementer;
    ctx.task_category = TaskCategory::Research;

    let result = cascade.route(&ctx);
    assert_eq!(result.primary.slug, "sonar-pro");
}

#[test]
fn cascade_perplexity_falls_back_to_standard_when_no_sonar() {
    let cascade = CascadeRouter::new(test_slugs()); // no sonar in test_slugs
    let mut ctx = default_ctx();
    ctx.role = AgentRole::Researcher;
    ctx.task_category = TaskCategory::Research;

    let result = cascade.route(&ctx);
    // No sonar available -> standard tier fallback
    assert_ne!(result.primary.slug, "sonar-pro");
    assert_ne!(result.primary.slug, "sonar");
}

#[test]
fn pareto_pruning_reduces_alpha_for_dominated_models() {
    let frontier = vec!["claude-sonnet-4-5".to_string()];
    let base_alpha = 0.8;

    assert!(
        (pareto_adjusted_alpha(base_alpha, "claude-sonnet-4-5", &frontier) - base_alpha).abs()
            < f64::EPSILON
    );
    assert!(
        (pareto_adjusted_alpha(base_alpha, "claude-haiku-4-5", &frontier) - base_alpha * 0.1).abs()
            < f64::EPSILON
    );
}

#[test]
fn pareto_frontier_refreshes_every_50_observations() {
    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let ctx = default_ctx();

    for _ in 0..50 {
        cascade.record_observation(&ctx, "claude-sonnet-4-5", 1.0, true);
    }

    assert_eq!(cascade.pareto_frontier_bucket(), 1);
    let frontier = cascade.pareto_frontier_slugs();
    assert!(frontier.contains(&"claude-haiku-4-5".to_string()));
    assert!(frontier.contains(&"claude-sonnet-4-5".to_string()));

    for _ in 0..50 {
        cascade.record_observation(&ctx, "claude-haiku-4-5", 0.0, false);
    }

    assert_eq!(cascade.pareto_frontier_bucket(), 2);
    let frontier = cascade.pareto_frontier_slugs();
    assert!(frontier.contains(&"claude-sonnet-4-5".to_string()));
    // Haiku remains on the frontier despite 0% pass rate because it has
    // a latency advantage (Fast tier = 10s vs Standard tier = 30s),
    // meaning sonnet does not dominate on all four objectives.
    assert!(frontier.contains(&"claude-haiku-4-5".to_string()));
}

/// bug-9ab6b8: the Graph feedback sinks record through
/// `observe_multi_objective_outcome`, which advances the stage and refreshes
/// the Pareto frontier as every other observation path does, with no reload.
#[test]
fn multi_objective_observations_advance_the_stage_and_the_frontier() {
    use roko_core::config::schema::RewardWeights;

    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let features = default_ctx().to_features();
    let weights = RewardWeights::default();
    for index in 0..50 {
        let success = index % 5 != 0;
        cascade.observe_multi_objective_outcome(
            features.clone(),
            1,
            1.0,
            0.1,
            0.1,
            &weights,
            success,
        );
    }

    assert_eq!(cascade.total_observations(), 50);
    assert_eq!(cascade.current_stage(), CascadeStage::Confidence);
    let transitions = cascade.stage_transitions();
    assert_eq!(transitions.len(), 1, "{transitions:?}");
    assert_eq!(transitions[0].from, CascadeStage::Static);
    assert_eq!(transitions[0].to, CascadeStage::Confidence);
    assert_eq!(cascade.pareto_frontier_bucket(), 1);
    let frontier = cascade.pareto_frontier_slugs();
    assert!(frontier.contains(&"claude-sonnet-4-5".to_string()));
}

#[test]
fn filter_unhealthy_retains_least_unhealthy_candidate() {
    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);
    let health = ProviderHealthRegistry::new();
    for _ in 0..3 {
        health.record_failure("bad-a", ErrorClass::Timeout);
    }
    for _ in 0..5 {
        health.record_failure("bad-b", ErrorClass::Timeout);
    }
    let mut providers = HashMap::new();
    providers.insert("claude-haiku-4-5".to_string(), "bad-a".to_string());
    providers.insert("claude-sonnet-4-5".to_string(), "bad-b".to_string());

    let filtered = cascade.filter_unhealthy(
        &["claude-haiku-4-5".into(), "claude-sonnet-4-5".into()],
        &health,
        &providers,
    );
    assert_eq!(filtered, vec!["claude-haiku-4-5".to_string()]);
}

#[test]
fn apply_cost_pressure_prefers_cheaper_models() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut scores = vec![
        ("claude-opus-4-6".to_string(), 1.0),
        ("claude-sonnet-4-5".to_string(), 1.0),
        ("claude-haiku-4-5".to_string(), 1.0),
    ];

    cascade.apply_cost_pressure(&mut scores, true);

    assert!((scores[0].1 - 0.0).abs() < 1e-9);
    assert!((scores[1].1 - 0.9).abs() < 1e-9);
    assert!((scores[2].1 - 1.2).abs() < 1e-9);
}

#[test]
fn load_static_overrides_updates_role_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("static-overrides.json");
    std::fs::write(
        &path,
        serde_json::json!({
            "implementer": "claude-sonnet-4-6"
        })
        .to_string(),
    )
    .unwrap();

    let cascade = CascadeRouter::new(test_slugs());
    let applied = cascade.load_static_overrides(&path).unwrap();
    assert_eq!(applied, 1);

    let mut ctx = default_ctx();
    ctx.role = AgentRole::Implementer;
    assert_eq!(cascade.route(&ctx).primary.slug, "claude-sonnet-4-6");
}

#[test]
fn feedback_updates_confidence_stats() {
    let cascade = CascadeRouter::new(test_slugs());

    cascade.feedback("claude-sonnet-4-5", 0.8, true, -0.2);
    cascade.feedback("claude-sonnet-4-5", 0.8, false, 0.8);

    // Confidence stats should be updated.
    let stats = cascade.confidence_snapshot();
    assert_eq!(stats.get("claude-sonnet-4-5"), Some(&(2, 1)));
}

#[test]
fn feedback_from_prediction_computes_residual() {
    let cascade = CascadeRouter::new(test_slugs());

    cascade.feedback_from_prediction("claude-haiku-4-5", 0.9, true);
    cascade.feedback_from_prediction("claude-haiku-4-5", 0.9, false);

    let stats = cascade.confidence_snapshot();
    assert_eq!(stats.get("claude-haiku-4-5"), Some(&(2, 1)));
}

#[test]
fn feedback_with_unknown_model_is_noop() {
    let cascade = CascadeRouter::new(test_slugs());

    // Should not panic, just silently skip.
    cascade.feedback("mystery-model-xyz", 0.5, true, -0.5);

    // No observations should have been recorded in confidence stats.
    let stats = cascade.confidence_snapshot();
    assert!(!stats.contains_key("mystery-model-xyz"));
}

// ── P2-LRN-5: task_category awareness in stages 2-3 ─────────────────────────

/// gap-b95d94: a hindsight relabel moves one credited success to a failure
/// in the category and confidence stats; the trials stay.
#[test]
fn retract_success_moves_a_credited_success_to_a_failure() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();
    ctx.task_category = TaskCategory::Docs;
    cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.9, true);
    let sonnet_docs = ("claude-sonnet-4-5".to_string(), "docs".to_string());
    assert_eq!(
        cascade.category_stats_snapshot().get(&sonnet_docs),
        Some(&(1, 1))
    );

    cascade.retract_success("claude-sonnet-4-5", TaskCategory::Docs);

    assert_eq!(
        cascade.category_stats_snapshot().get(&sonnet_docs),
        Some(&(1, 0))
    );
    let confidence = cascade.confidence_snapshot();
    assert_eq!(confidence.get("claude-sonnet-4-5"), Some(&(1, 0)));
}

/// bug-583e50: a retraction of a success that an earlier run saved
/// survives the save, though it leaves fewer successes than were loaded.
#[test]
fn a_saved_success_retracted_by_a_later_run_stays_retracted() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let earlier = CascadeRouter::new(test_slugs());
    earlier.record_observation(&default_ctx(), "claude-sonnet-4-5", 0.9, true);
    earlier.save(&path).unwrap();

    let later = CascadeRouter::load_or_new(&path, test_slugs());
    later.retract_success("claude-sonnet-4-5", default_ctx().task_category);
    later.save(&path).unwrap();

    let reloaded = CascadeRouter::load_or_new(&path, test_slugs());
    let confidence = reloaded.confidence_snapshot();
    assert_eq!(confidence.get("claude-sonnet-4-5"), Some(&(1, 0)));
}

/// record_observation must populate category_stats so Stage 2
/// confidence_scores can apply the per-category pass-rate delta.
#[test]
fn record_observation_populates_category_stats() {
    let cascade = CascadeRouter::new(test_slugs());
    let mut ctx = default_ctx();
    ctx.task_category = TaskCategory::Docs;

    // One success for sonnet on Docs.
    cascade.record_observation(&ctx, "claude-sonnet-4-5", 0.9, true);
    // One failure for haiku on Docs.
    cascade.record_observation(&ctx, "claude-haiku-4-5", 0.1, false);

    let cat = cascade.category_stats_snapshot();
    assert_eq!(
        cat.get(&("claude-sonnet-4-5".to_string(), "docs".to_string())),
        Some(&(1, 1)),
        "sonnet/docs should have 1 trial + 1 success"
    );
    assert_eq!(
        cat.get(&("claude-haiku-4-5".to_string(), "docs".to_string())),
        Some(&(1, 0)),
        "haiku/docs should have 1 trial + 0 successes"
    );
    // Implementation category must not appear.
    assert!(
        cat.get(&(
            "claude-sonnet-4-5".to_string(),
            "implementation".to_string()
        ))
        .is_none(),
        "implementation entry must not appear for a Docs observation"
    );
}

/// After enough per-category observations Stage 2 should prefer the model
/// with the better category-specific pass rate over the one with the better
/// global pass rate, when the task category matches.
#[test]
fn stage2_confidence_uses_task_category_delta() {
    // sonnet and haiku start with the same global pass rate (50% each):
    // 5 successes out of 10 trials.  For Refactor tasks specifically, haiku
    // has a 100% pass rate while sonnet has 0%.  After enough category
    // observations (>= CATEGORY_MIN_TRIALS = 5), the Confidence stage must
    // prefer haiku for Refactor.
    let cascade = CascadeRouter::new(vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
    ]);

    // Build identical global pass rates by alternating success/failure.
    let mut base_ctx = default_ctx();
    base_ctx.task_category = TaskCategory::Implementation;
    for _ in 0..5 {
        cascade.record_observation(&base_ctx, "claude-haiku-4-5", 0.9, true);
        cascade.record_observation(&base_ctx, "claude-haiku-4-5", 0.1, false);
        cascade.record_observation(&base_ctx, "claude-sonnet-4-5", 0.9, true);
        cascade.record_observation(&base_ctx, "claude-sonnet-4-5", 0.1, false);
    }

    // Now give haiku a perfect Refactor record and sonnet a zero record.
    let mut refactor_ctx = default_ctx();
    refactor_ctx.task_category = TaskCategory::Refactor;
    for _ in 0..8 {
        cascade.record_observation(&refactor_ctx, "claude-haiku-4-5", 0.9, true);
        cascade.record_observation(&refactor_ctx, "claude-sonnet-4-5", 0.1, false);
    }

    // Force into Confidence stage by bumping to 50 total observations.
    for _ in 0..10 {
        cascade.record_observation(&base_ctx, "claude-haiku-4-5", 0.5, true);
        cascade.record_observation(&base_ctx, "claude-sonnet-4-5", 0.5, true);
    }

    assert_eq!(
        cascade.current_stage(),
        CascadeStage::Confidence,
        "router should be in Confidence stage"
    );

    // Route a Refactor task — haiku should win due to the category delta.
    let result = cascade.route(&refactor_ctx);
    assert_eq!(
        result.primary.slug, "claude-haiku-4-5",
        "Stage 2 should prefer the model with the better Refactor pass rate"
    );
}

// ── Persistence across processes (bug-605a8a, bug-9c88ac) ───────────

/// LinUCB updates the arm for `slug` has absorbed: every context vector sets
/// bias feature 16 to 1, so each update adds 1 to `A[16][16]`.
fn arm_updates(router: &CascadeRouter, slug: &str) -> f64 {
    let arm = router
        .linucb()
        .arm_stats()
        .into_iter()
        .find(|arm| arm.slug == slug)
        .expect("arm for the slug");
    arm.a_matrix[16][16] - 1.0
}

#[test]
fn narrower_slug_list_keeps_other_models_state() {
    // bug-605a8a: a router loaded with fewer models, in another order, gets
    // each model's own LinUCB arm, and its save keeps the counters and arm
    // of the model it does not track.
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let ctx = default_ctx();
    let full = CascadeRouter::new(test_slugs());
    for _ in 0..3 {
        full.record_observation(&ctx, "claude-haiku-4-5", 0.9, true);
    }
    full.record_observation(&ctx, "claude-sonnet-4-5", 0.1, false);
    for _ in 0..2 {
        full.record_observation(&ctx, "claude-opus-4-6", 0.6, true);
    }
    full.save(&path).unwrap();

    // Opus first and no haiku: opus now sits where haiku's arm was saved.
    let narrow = CascadeRouter::load_or_new(
        &path,
        vec![
            "claude-opus-4-6".to_string(),
            "claude-sonnet-4-5".to_string(),
        ],
    );
    assert_eq!(
        arm_updates(&narrow, "claude-opus-4-6"),
        2.0,
        "opus gets its own arm, not haiku's"
    );
    assert_eq!(arm_updates(&narrow, "claude-sonnet-4-5"), 1.0);
    let narrow_confidence = narrow.confidence_snapshot();
    assert!(!narrow_confidence.contains_key("claude-haiku-4-5"));
    narrow.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    narrow.save(&path).unwrap();

    let reloaded = CascadeRouter::load_or_new(&path, test_slugs());
    let confidence = reloaded.confidence_snapshot();
    assert_eq!(
        confidence["claude-haiku-4-5"],
        (3, 3),
        "the narrower router's save keeps haiku's counters"
    );
    assert_eq!(confidence["claude-sonnet-4-5"], (2, 1));
    assert_eq!(confidence["claude-opus-4-6"], (2, 2));
    assert_eq!(
        arm_updates(&reloaded, "claude-haiku-4-5"),
        3.0,
        "and haiku's arm"
    );
    assert_eq!(arm_updates(&reloaded, "claude-sonnet-4-5"), 2.0);
    assert_eq!(arm_updates(&reloaded, "claude-opus-4-6"), 2.0);
    assert_eq!(reloaded.total_observations(), 7);
}

#[test]
fn legacy_snapshot_arms_follow_model_slugs() {
    // Snapshots written before arms carried their slugs list the arms in
    // `model_slugs` order; a reordered router still gets each model's arm.
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let ctx = default_ctx();
    let full = CascadeRouter::new(test_slugs());
    for _ in 0..3 {
        full.record_observation(&ctx, "claude-haiku-4-5", 0.9, true);
    }
    full.record_observation(&ctx, "claude-opus-4-6", 0.6, true);
    full.save(&path).unwrap();
    let mut legacy: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    legacy["linucb_state"]
        .as_object_mut()
        .unwrap()
        .remove("slugs");
    std::fs::write(&path, legacy.to_string()).unwrap();

    let reordered = CascadeRouter::load_or_new(
        &path,
        vec![
            "claude-opus-4-6".to_string(),
            "claude-sonnet-4-5".to_string(),
            "claude-haiku-4-5".to_string(),
        ],
    );
    assert_eq!(arm_updates(&reordered, "claude-opus-4-6"), 1.0);
    assert_eq!(arm_updates(&reordered, "claude-sonnet-4-5"), 0.0);
    assert_eq!(arm_updates(&reordered, "claude-haiku-4-5"), 3.0);
}

#[test]
fn concurrent_saves_merge_observations() {
    // bug-9c88ac: processes that load the same snapshot and each save what
    // they learned keep each other's observations; the last writer no
    // longer wins.
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let ctx = default_ctx();
    let seed = CascadeRouter::new(test_slugs());
    seed.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    seed.save(&path).unwrap();

    let serve = CascadeRouter::load_or_new(&path, test_slugs());
    let cli = CascadeRouter::load_or_new(&path, vec!["claude-sonnet-4-5".to_string()]);
    serve.record_observation(&ctx, "claude-sonnet-4-5", 0.8, true);
    serve.record_observation(&ctx, "claude-opus-4-6", 0.9, true);
    cli.record_observation(&ctx, "claude-sonnet-4-5", 0.0, false);
    cli.save(&path).unwrap();
    serve.save(&path).unwrap();
    // Saving again adds nothing: the first save already wrote it all.
    serve.save(&path).unwrap();

    let merged = CascadeRouter::load_or_new(&path, test_slugs());
    let confidence = merged.confidence_snapshot();
    assert_eq!(confidence["claude-sonnet-4-5"], (3, 2));
    assert_eq!(confidence["claude-opus-4-6"], (1, 1));
    assert_eq!(arm_updates(&merged, "claude-sonnet-4-5"), 3.0);
    assert_eq!(arm_updates(&merged, "claude-opus-4-6"), 1.0);
    assert_eq!(merged.total_observations(), 4);

    // Routers racing for the snapshot lock lose nothing either.
    let workers = (0..4)
        .map(|_| {
            let path = path.clone();
            std::thread::spawn(move || {
                let router = CascadeRouter::load_or_new(&path, test_slugs());
                for _ in 0..5 {
                    router.record_observation(&default_ctx(), "claude-haiku-4-5", 0.7, true);
                }
                router.save(&path).unwrap();
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }
    let merged = CascadeRouter::load_or_new(&path, test_slugs());
    assert_eq!(merged.confidence_snapshot()["claude-haiku-4-5"], (20, 20));
    assert_eq!(arm_updates(&merged, "claude-haiku-4-5"), 20.0);
    assert_eq!(merged.total_observations(), 24);
}

#[test]
fn save_replaces_an_unreadable_snapshot() {
    // A corrupt snapshot is backed up and replaced, as loading one is;
    // otherwise the merge could never save again.
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    std::fs::write(&path, "{ not json").unwrap();

    let router = CascadeRouter::load_or_new(&path, test_slugs());
    router.record_observation(&default_ctx(), "claude-sonnet-4-5", 0.8, true);
    router.save(&path).unwrap();

    let reloaded = CascadeRouter::load_or_new(&path, test_slugs());
    assert_eq!(reloaded.confidence_snapshot()["claude-sonnet-4-5"], (1, 1));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("cascade-router.json.corrupted")).unwrap(),
        "{ not json"
    );
}

#[test]
fn category_stats_survive_a_save() {
    // bug-a6a3cd: the per-category counts behind Stage 2's pass-rate delta
    // are saved and loaded, and routers that share the snapshot keep each
    // other's counts.
    let dir = tempdir().unwrap();
    let path = dir.path().join("cascade-router.json");
    let first = CascadeRouter::load_or_new(&path, test_slugs());
    let second = CascadeRouter::load_or_new(&path, test_slugs());
    for success in [true, true, false] {
        first.record_category_outcome("claude-sonnet-4-5", TaskCategory::Research, success);
    }
    second.record_category_outcome("claude-sonnet-4-5", TaskCategory::Research, true);
    second.record_category_outcome("claude-haiku-4-5", TaskCategory::Docs, false);
    first.save(&path).unwrap();
    second.save(&path).unwrap();
    // Saving again adds nothing: the first save already wrote it all.
    first.save(&path).unwrap();

    let loaded = CascadeRouter::load_or_new(&path, test_slugs());
    let stats = loaded.category_stats_snapshot();
    let counts = |slug: &str, category: TaskCategory| {
        stats
            .get(&(slug.to_string(), category.label().to_string()))
            .copied()
    };
    assert_eq!(counts("claude-sonnet-4-5", TaskCategory::Research), Some((4, 3)));
    assert_eq!(counts("claude-haiku-4-5", TaskCategory::Docs), Some((1, 0)));
    assert_eq!(stats.len(), 2, "{stats:?}");
}

#[test]
fn category_stats_merge_keeps_retractions() {
    // bug-a6a3cd: a save adds the trials a router gained since its base and
    // subtracts the successes a relabel retracted, keeping what other
    // writers saved meanwhile.
    let counts = |trials, successes| {
        HashMap::from([(
            "claude-sonnet-4-5".to_string(),
            HashMap::from([(TaskCategory::Research, CategoryModelStats { trials, successes })]),
        )])
    };
    let base = CascadeSnapshot {
        category_stats: counts(4, 3),
        ..CascadeSnapshot::default()
    };
    // One more trial, a failure, and one success retracted.
    let current = CascadeSnapshot {
        category_stats: counts(5, 2),
        ..CascadeSnapshot::default()
    };
    let mut latest = CascadeSnapshot {
        category_stats: counts(6, 5),
        ..CascadeSnapshot::default()
    };
    merge_learning(&mut latest, &current, &base);
    let merged = &latest.category_stats["claude-sonnet-4-5"][&TaskCategory::Research];
    assert_eq!((merged.trials, merged.successes), (7, 4));
}
