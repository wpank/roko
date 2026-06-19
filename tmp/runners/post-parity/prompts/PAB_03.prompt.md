# PAB_03: Thread task role into extension hooks instead of hardcoded "implementer"

## Task
Pass the actual task role from `TaskDef` into `pre_inference` and `post_inference` extension hooks.

## Runner Context
Runner PAB, batch 3 of 4. No dependencies.

## Problem
`event_loop.rs:2535`:
```rust
let mut req = InferenceRequest {
    role: "implementer".to_string(),  // always "implementer" regardless of task
    ...
};
```

Tasks with role "researcher", "strategist", "reviewer" etc. all get "implementer" passed to extension hooks, which means plugins can't apply role-specific behavior.

## Exact Changes

### Step 1: Find where the role is available

The task role is available in `TaskDef.role: Option<String>`. The dispatch path that calls `fire_pre_inference_hook` has access to the `TaskDef`.

### Step 2: Thread the role

```rust
// BEFORE
role: "implementer".to_string(),

// AFTER
role: task_def.role.as_deref().unwrap_or("implementer").to_string(),
```

### Step 3: Apply to both hooks

Search for all `fire_pre_inference_hook` and `fire_post_inference_hook` calls. Ensure both receive the actual role.

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs`


## Verify
```bash
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-cli 2>&1 | tail -20
```
## Acceptance Criteria
- Extension hooks receive the actual task role
- Tasks with `role = "researcher"` get "researcher" in the hook
- Default remains "implementer" when role is unspecified

## Do NOT
- Change unrelated code in the same file
- Add features beyond what's specified
- Remove existing tests
