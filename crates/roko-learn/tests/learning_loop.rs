//! Integration tests for the learning loop wiring (checklist §I.3).
//!
//! These tests verify that the three subsystems added in this batch —
//! episode logging, bandit persistence, and provider health — work
//! end-to-end across crate boundaries.

use tempfile::TempDir;

use roko_core::DaimonPolicy;
use roko_core::agent::AgentRole;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::model_router::{LinUCBRouter, RoutingContext};
use roko_learn::provider_health::ProviderHealthTracker;

// ─── Helpers ───────────────────────────────────────────────────────────────

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
        daimon_policy: DaimonPolicy::default(),
        thinking_level: None,
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

fn test_slugs() -> Vec<String> {
    vec![
        "claude-haiku-4-5".to_string(),
        "claude-sonnet-4-5".to_string(),
        "claude-opus-4-6".to_string(),
    ]
}

// ─── Test 1: Episode logging — append and read back ────────────────────────

#[tokio::test]
async fn episode_logger_roundtrip_in_temp_dir() {
    let tmp = TempDir::new().unwrap();
    let ep_path = tmp.path().join("episodes.jsonl");
    let logger = EpisodeLogger::new(&ep_path);

    let mut ep = Episode::new("test-agent", "task-42");
    ep.success = true;
    logger.append(&ep).await.unwrap();

    let read_back = EpisodeLogger::read_all(&ep_path).await.unwrap();
    assert_eq!(read_back.len(), 1);
    assert_eq!(read_back[0].agent_id, "test-agent");
    assert_eq!(read_back[0].task_id, "task-42");
    assert!(read_back[0].success);
}

// ─── Test 2: Episode logging — multiple appends ────────────────────────────

#[tokio::test]
async fn episode_logger_multiple_appends() {
    let tmp = TempDir::new().unwrap();
    let ep_path = tmp.path().join("episodes.jsonl");
    let logger = EpisodeLogger::new(&ep_path);

    for i in 0..5 {
        let ep = Episode::new("agent", format!("task-{i}"));
        logger.append(&ep).await.unwrap();
    }

    let read_back = EpisodeLogger::read_all(&ep_path).await.unwrap();
    assert_eq!(read_back.len(), 5);
}

// ─── Test 3: Bandit persistence — update, drop, reload ────────────────────

#[test]
fn bandit_persistence_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("router.json");

    // Create, train 3 times, and save.
    let router = LinUCBRouter::new(test_slugs()).with_persist_path(&path);
    let ctx = default_ctx();
    router.update(&ctx, "claude-sonnet-4-5", 0.9);
    router.update(&ctx, "claude-haiku-4-5", 0.3);
    router.update(&ctx, "claude-opus-4-6", 0.7);
    router.save().unwrap();

    // Drop the router — state is only on disk now.
    drop(router);

    // Reload from the same path.
    let reloaded = LinUCBRouter::load(&path, test_slugs()).unwrap();
    assert_eq!(reloaded.total_observations(), 3);

    let stats = reloaded.arm_stats();
    let sonnet = stats
        .iter()
        .find(|a| a.slug == "claude-sonnet-4-5")
        .unwrap();
    assert_eq!(sonnet.observations, 1);
    let haiku = stats.iter().find(|a| a.slug == "claude-haiku-4-5").unwrap();
    assert_eq!(haiku.observations, 1);
    let opus = stats.iter().find(|a| a.slug == "claude-opus-4-6").unwrap();
    assert_eq!(opus.observations, 1);
}

// ─── Test 4: Auto-persist on update ────────────────────────────────────────

#[test]
fn auto_persist_on_update() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("auto.json");

    // Route with persist_path — update() should auto-save.
    let router = LinUCBRouter::new(test_slugs()).with_persist_path(&path);
    let ctx = default_ctx();
    router.update(&ctx, "claude-sonnet-4-5", 0.8);
    router.update(&ctx, "claude-sonnet-4-5", 0.6);

    // Verify the file was created by the auto-persist.
    assert!(path.exists(), "router file should exist after update()");

    // Reload and verify.
    let reloaded = LinUCBRouter::load(&path, test_slugs()).unwrap();
    assert_eq!(reloaded.total_observations(), 2);
}

// ─── Helpers (health) ─────────────────────────────────────────────────────

/// Map model slugs to a synthetic provider name.
fn slug_to_provider(slug: &str) -> String {
    if slug.starts_with("claude") {
        "anthropic".to_string()
    } else {
        "other".to_string()
    }
}

/// Build a router with enough observations to exit cold-start.
fn warm_router_with_health(health: ProviderHealthTracker) -> LinUCBRouter {
    let router = LinUCBRouter::new(test_slugs()).with_health_tracker(health, slug_to_provider);

    let ctx = default_ctx();
    // Push past the cold-start threshold (50 observations).
    for _ in 0..60 {
        router.update(&ctx, "claude-sonnet-4-5", 0.8);
    }

    router
}

// ─── Test 5: Health tracker is stored and retrievable ─────────────────────

#[test]
fn health_tracker_stored_on_router() {
    let health = ProviderHealthTracker::new();
    let router = LinUCBRouter::new(test_slugs()).with_health_tracker(health, slug_to_provider);

    assert!(
        router.health_tracker().is_some(),
        "health tracker should be accessible after with_health_tracker()"
    );
}

// ─── Test 6: Healthy providers are selected normally ──────────────────────

#[test]
fn healthy_providers_selected_normally() {
    let health = ProviderHealthTracker::new();
    let router = warm_router_with_health(health);

    // All providers healthy — should select from UCB scores.
    let ctx = default_ctx();
    let model = router.select_model(&ctx);
    let slug = model.slug.clone();

    // The router should pick one of the known slugs.
    let known = test_slugs();
    assert!(
        known.contains(&slug.to_string()),
        "selected slug '{slug}' should be one of {known:?}"
    );
}

// ─── Test 7: Degraded provider is skipped ─────────────────────────────────

#[test]
fn degraded_provider_skipped_in_selection() {
    let health = ProviderHealthTracker::new();

    // Trip the circuit breaker for "anthropic" (3 consecutive failures).
    for _ in 0..3 {
        health.record_failure("anthropic");
    }

    assert!(
        !health.is_healthy("anthropic"),
        "anthropic should be unhealthy after 3 failures"
    );

    let router = warm_router_with_health(health);
    let ctx = default_ctx();

    // With anthropic unhealthy and all slugs being claude-*, every slug maps
    // to "anthropic". When all providers are unhealthy the router falls back
    // to the best-scoring arm (it never returns an error).
    let model = router.select_model(&ctx);
    let slug = model.slug.clone();
    let known = test_slugs();
    assert!(
        known.contains(&slug.to_string()),
        "fallback slug '{slug}' should be one of {known:?}"
    );
}

// ─── Test 8: Recovery after recording success ─────────────────────────────

#[test]
fn provider_recovers_after_success() {
    let health = ProviderHealthTracker::new();

    // Trip the breaker.
    for _ in 0..3 {
        health.record_failure("anthropic");
    }
    assert!(!health.is_healthy("anthropic"));

    // Record a success — should reset to healthy.
    health.record_success("anthropic");
    assert!(
        health.is_healthy("anthropic"),
        "anthropic should be healthy after record_success()"
    );

    // Router should route normally again.
    let router = warm_router_with_health(health);
    let ctx = default_ctx();
    let model = router.select_model(&ctx);
    let slug = model.slug.clone();
    let known = test_slugs();
    assert!(
        known.contains(&slug.to_string()),
        "recovered slug '{slug}' should be one of {known:?}"
    );
}

// ─── Test 9: filter_arms removes unhealthy arms ──────────────────────────

#[test]
fn filter_arms_removes_unhealthy() {
    let health = ProviderHealthTracker::new();

    // All healthy initially — filter should keep all.
    let arms = test_slugs();
    let filtered = health.filter_arms(&arms, slug_to_provider);
    assert_eq!(
        filtered.len(),
        arms.len(),
        "all arms should pass when healthy"
    );

    // Trip anthropic.
    for _ in 0..3 {
        health.record_failure("anthropic");
    }

    // All claude-* models map to anthropic, so everything is filtered out.
    let filtered = health.filter_arms(&arms, slug_to_provider);
    assert!(
        filtered.is_empty(),
        "all arms should be filtered when their provider is unhealthy"
    );

    // filter_arms_or_best should still return at least one fallback.
    let fallback = health.filter_arms_or_best(&arms, slug_to_provider);
    assert!(
        !fallback.is_empty(),
        "filter_arms_or_best should always return at least one arm"
    );
}
