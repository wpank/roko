# PAD_03: Wire CascadeRouter consultation before every ModelCallService call

## Task
Ensure `ModelCallService` consults the `CascadeRouter` for model selection on every inference call, not just from dead orchestrate.rs code.

## Runner Context
Runner PAD (Stream Parser Consolidation), batch 3 of 3. Depends on PAD_02.

## Problem
DP-3 anti-pattern: "Router built, never asked." `ModelCallService` has a `cascade_router` field (model_call_service.rs:~70) and a `model_router` field, but the actual model selection logic may not consult the router for every call. The routing context construction may use defaults instead of real task metadata.

## Exact Changes

### Step 1: Verify ModelCallService.call() routing path

Read `model_call_service.rs` around the `call()` or `dispatch()` method. Check:
- Does it construct a `RoutingContext` from the request?
- Does it call `cascade_router.select_model()`?
- Does it fall back to `default_model` only when router is empty?

### Step 2: Ensure RoutingContext populated from request metadata

```rust
// In ModelCallService dispatch:
let ctx = RoutingContext {
    task_category: request.task_category.unwrap_or(TaskCategory::Implementation),
    complexity: request.complexity.unwrap_or(TaskComplexityBand::Medium),
    iteration: request.iteration.unwrap_or(0),
    role: request.role.unwrap_or(AgentRole::Implementer),
    // ... fill from request metadata, not hardcoded defaults
};

let model = if let Some(router) = &self.cascade_router {
    router.select_model(ctx.to_features(), &self.reward_weights)
        .map(|s| s.model.clone())
        .unwrap_or_else(|| self.default_model.clone())
} else {
    self.default_model.clone()
};
```

### Step 3: Verify observation feedback loop

After each call completes, verify that the result feeds back to the router:

```rust
// After call completes:
if let Some(router) = &mut self.cascade_router {
    if let Some(idx) = router.model_index(&model) {
        let quality = if response.success { 1.0 } else { 0.0 };
        router.observe_multi_objective(
            ctx.to_features(),
            idx,
            quality,
            normalized_cost,
            normalized_latency,
            &self.reward_weights,
        );
    }
}
```

## Write Scope
- `crates/roko-agent/src/model_call_service.rs` (verify/fix routing consultation and observation)

## Read-Only Context
- `crates/roko-learn/src/cascade_router.rs` (select_model, observe_multi_objective)
- `crates/roko-learn/src/model_router.rs` (RoutingContext)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Every MCS call consults CascadeRouter when available
- RoutingContext populated from request metadata (not all defaults)
- Observation recorded after each call
- Default model used only as fallback when router unavailable or empty

## Do NOT
- Change the CascadeRouter API
- Add request fields that don't already exist on the MCS request type
- Block calls when router is unavailable (graceful fallback)
