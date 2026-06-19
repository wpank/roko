# PAK_01: Add unit tests for CascadeRouter core routing logic

## Task
Add tests for `CascadeRouter` covering stage transitions, model selection, tier routing, and persistence — currently 2,195 lines with zero test functions.

## Runner Context
Runner PAK (Testing Gaps), batch 1 of 3. No dependencies.

## Problem
`cascade_router.rs` (2,195 lines) has ZERO test functions. It has 2 `#[cfg(test)]` accessors (lines 1650, 1656) but no actual tests. This is the core model routing system — bugs here silently mis-route every agent dispatch.

## Current Code

**CascadeRouter** — `crates/roko-learn/src/cascade_router.rs`:

Key public functions:
- `new()` (line 151) — router initialization
- `with_role_table()` (line 172) — init with role overrides
- `select()` (line 269) — core routing decision (UCB1 bandit)
- `select_for_frequency()` (line 282) — frequency-aware selection
- `select_tier_with_active_inference()` (line 333) — tier selection
- `strongest_model()` (line 344) — returns highest-quality model
- `cheapest_model()` (line 365) — returns cheapest model
- `check_stage_transition()` (line 229) — adaptive stage progression
- `total_observations()` (line 224) — observation count

Test-only accessors:
- `pareto_frontier_bucket()` (line 1650) — `#[cfg(test)]`
- `pareto_frontier_slugs()` (line 1656) — `#[cfg(test)]`

## Exact Changes

### Step 1: Add test module

At the bottom of `cascade_router.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_router() -> CascadeRouter {
        CascadeRouter::new()
    }
```

### Step 2: Test basic model selection

```rust
    #[test]
    fn select_returns_model() {
        let router = test_router();
        let model = router.select("implementer", "medium");
        assert!(!model.is_empty(), "select() must return a model slug");
    }

    #[test]
    fn strongest_model_returns_opus_tier() {
        let router = test_router();
        let model = router.strongest_model();
        // Should return the highest-quality model available
        assert!(
            model.contains("opus") || model.contains("sonnet"),
            "strongest_model should be opus or sonnet tier, got: {model}"
        );
    }

    #[test]
    fn cheapest_model_returns_haiku_tier() {
        let router = test_router();
        let model = router.cheapest_model();
        assert!(
            model.contains("haiku") || model.contains("flash"),
            "cheapest_model should be haiku/flash tier, got: {model}"
        );
    }
```

### Step 3: Test stage transitions

```rust
    #[test]
    fn stage_transitions_after_observations() {
        let mut router = test_router();
        let initial_stage = router.current_stage();

        // Simulate observations to trigger stage transition
        for i in 0..50 {
            router.observe(ObservationRecord {
                model: "test-model".into(),
                role: "implementer".into(),
                quality: 0.8,
                latency_ms: 500,
                cost_usd: 0.01,
                tokens: 1000,
            });
        }

        // After enough observations, stage should advance (or stay if threshold not met)
        let transitioned = router.check_stage_transition();
        // Just verify it doesn't panic — transition threshold depends on config
        let _ = transitioned;
    }
```

### Step 4: Test persistence round-trip

```rust
    #[test]
    fn save_and_load_preserves_state() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path();

        let mut router = test_router();
        // Add some observations
        router.observe(ObservationRecord {
            model: "test-model".into(),
            role: "implementer".into(),
            quality: 0.9,
            latency_ms: 200,
            cost_usd: 0.005,
            tokens: 500,
        });
        let obs_before = router.total_observations();
        router.save(path).unwrap();

        let loaded = CascadeRouter::load(path).unwrap();
        assert_eq!(loaded.total_observations(), obs_before);
    }
```

### Step 5: Test role-table overrides

```rust
    #[test]
    fn role_table_overrides_default_selection() {
        let mut table = HashMap::new();
        table.insert("researcher".to_string(), "perplexity-sonar".to_string());
        let router = CascadeRouter::with_role_table(table);

        let model = router.select("researcher", "medium");
        assert_eq!(model, "perplexity-sonar",
            "role table override should force specific model");
    }
```

### Step 6: Adapt test code to actual API

The test code above uses plausible method signatures. Before writing, read the actual signatures of:
- `CascadeRouter::new()`, `select()`, `observe()`, `save()`, `load()`
- `ObservationRecord` fields

Adjust the test code to match the real API. The test patterns are correct — only the exact field names/signatures may differ.

## Write Scope
- `crates/roko-learn/src/cascade_router.rs` (add `#[cfg(test)] mod tests` with 5+ tests)

## Read-Only Context
- `crates/roko-learn/src/cascade_router.rs:151-365` (public API)
- `crates/roko-learn/src/cascade_router.rs:1650-1656` (existing test accessors)

## Verify
```bash
cargo test -p roko-learn -- cascade_router 2>&1 | tail -30
```

## Acceptance Criteria
- At least 5 test functions covering: basic selection, strongest/cheapest, stage transition, persistence, role overrides
- All tests pass with `cargo test -p roko-learn`
- Tests use real CascadeRouter API (not mocks)
- Tests are deterministic (no flaky random selection)

## Do NOT
- Change the CascadeRouter implementation
- Add new public API surface
- Mock the bandit algorithm (test real behavior)
- Add integration tests that need a running server
