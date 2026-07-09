# PAC_02: Add rate limiter per-task reset boundary

## Task
Add explicit per-task reset to `RateLimiter` so stale rate-limit windows from one task don't throttle the next task.

## Runner Context
Runner PAC (Safety Completeness), batch 2 of 4. No dependencies.

## Problem
ISS-7 safety gap: `RateLimiter` (rate_limit.rs:1-99) uses a sliding-window counter keyed by `(role, tool)`. Between tasks, stale timestamps expire via the window naturally, but there is NO explicit per-task reset boundary. If a task exhausts the rate limit near the end, the next task inherits the cooldown — even though it's a completely new task context.

## Current Code

**RateLimiter** — `crates/roko-agent/src/safety/rate_limit.rs:1-99`:
```rust
pub struct RateLimiter {
    // Sliding window counter keyed by (role, tool)
    // No reset(), clear(), or drain() method
}
```

Methods: `check()`, `record()`. No `reset()` exists.

## Exact Changes

### Step 1: Add reset_for_task() to RateLimiter

```rust
impl RateLimiter {
    /// Reset all sliding windows for a new task context.
    /// Called at task boundaries to prevent cross-task rate limit bleed.
    pub fn reset_for_task(&mut self) {
        // Clear all window entries — new task gets fresh limits
        self.windows.clear();
    }

    /// Reset windows for a specific role only.
    pub fn reset_for_role(&mut self, role: &str) {
        self.windows.retain(|key, _| key.0 != role);
    }
}
```

### Step 2: Call reset at task boundaries in the event loop

In `event_loop.rs`, before dispatching a new task:

```rust
// Before starting a new task's agent:
if let Some(safety) = &mut agent_config.safety_layer {
    safety.rate_limiter.reset_for_task();
    debug!(task = %task.id, "rate limiter reset for new task");
}
```

### Step 3: Also reset in tool loop at turn boundaries

In `tool_loop/mod.rs`, at the start of each new turn (complements PAC_01's budget reset):

```rust
// At turn start (same place as budget reset):
if let Some(limiter) = &mut safety.rate_limiter {
    // Don't full-reset at turns — just let the window slide naturally
    // Full reset only at task boundaries
}
```

## Write Scope
- `crates/roko-agent/src/safety/rate_limit.rs` (add reset_for_task, reset_for_role)
- `crates/roko-cli/src/runner/event_loop.rs` (call reset before each task dispatch)

## Read-Only Context
- `crates/roko-agent/src/safety/mod.rs` (SafetyLayer — check how rate_limiter is exposed)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- `reset_for_task()` clears all sliding windows
- Called at task boundaries in the event loop
- New tasks start with fresh rate limits
- Existing sliding-window behavior unchanged within a task

## Do NOT
- Reset at every turn boundary (too aggressive — only at task boundaries)
- Change the sliding-window algorithm
- Remove the natural window expiry mechanism
