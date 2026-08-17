# PAI_04: Wire event subscription dispatch_loop to StateHub events

## Task
Connect the existing `dispatch_loop` in roko-serve to the runner's StateHub/EventBus so configured subscriptions actually fire when events occur.

## Runner Context
Runner PAI (Config & Infrastructure), batch 4 of 4. No dependencies.

## Problem
CI-4 anti-pattern: "Subscriptions wired but not triggered." The full subscription infrastructure exists:
- `SubscriptionConfig` at `subscriptions.rs:59-87` (config schema)
- `SubscriptionRegistry` at `dispatch.rs:786-791` (runtime registry)
- `dispatch_loop` at `dispatch.rs:1460-1530` (event→subscriber dispatch)
- `dispatch_agent` at `dispatch.rs:1603-1668` (agent template dispatch)
- `dispatch_template` at `dispatch.rs:1670-1750` (template execution)
- CRUD routes at `routes/subscriptions.rs:19-32`

But the `dispatch_loop` is never spawned from the CLI runner's event loop. `roko serve` has the registry (`AppState.subscriptions` at state.rs:369) but the CLI runner path doesn't connect StateHub events to it.

## Current Code

**SubscriptionConfig** — `crates/roko-core/src/config/subscriptions.rs:59-87`:
```rust
pub struct SubscriptionConfig {
    // trigger, filter, action fields
}
```

**SubscriptionTrigger** — `subscriptions.rs:116-138`:
Defines event patterns that trigger a subscription.

**SubscriptionRegistry** — `crates/roko-serve/src/dispatch.rs:786-791`:
```rust
pub struct SubscriptionRegistry {
    subscriptions: Vec<Subscription>,
    // ...
}
```

**dispatch_loop** — `dispatch.rs:1460-1530`:
The main dispatch loop — iterates subscriptions, checks triggers, dispatches actions. Already implemented but never called from the runner.

**AppState.subscriptions** — `crates/roko-serve/src/state.rs:369`:
```rust
pub subscriptions: SubscriptionRegistry,
```

## Exact Changes

### Step 1: Spawn dispatch_loop from serve startup

In `crates/roko-serve/src/` where the server is initialized (where `AppState::new()` is called, around state.rs:526-528), after constructing the AppState, spawn the dispatch loop:

```rust
// After AppState construction:
let registry = app_state.subscriptions.clone();
let event_rx = app_state.event_bus.subscribe();
tokio::spawn(async move {
    if let Err(err) = dispatch_loop(registry, event_rx).await {
        tracing::error!(%err, "subscription dispatch loop exited");
    }
});
```

### Step 2: Connect CLI runner's StateHub to subscriptions

In `crates/roko-cli/src/runner/event_loop.rs`, where StateHub is constructed, add an optional subscription dispatch:

```rust
// Load subscriptions from config
let subscriptions = if !config.subscriptions.is_empty() {
    let registry = SubscriptionRegistry::from_config(&config.subscriptions);
    let event_rx = state_hub.subscribe();
    Some(tokio::spawn(async move {
        dispatch_loop(registry, event_rx).await
    }))
} else {
    None  // No subscriptions configured → no overhead
};
```

### Step 3: Make dispatch_loop accept a broadcast::Receiver

If `dispatch_loop` at dispatch.rs:1460 currently takes a different receiver type than what StateHub provides, add an adapter:

```rust
// dispatch_loop should accept:
pub async fn dispatch_loop(
    registry: SubscriptionRegistry,
    mut rx: broadcast::Receiver<DashboardEvent>,
) -> Result<()> {
    while let Ok(event) = rx.recv().await {
        registry.dispatch(&event).await;
    }
    Ok(())
}
```

Check the existing signature and adjust the caller to match.

### Step 4: Ensure SubscriptionRegistry is accessible from roko-cli

If `SubscriptionRegistry` is only in `roko-serve`, either:
- Re-export it from a shared crate (roko-runtime), or
- Add `roko-serve` as a dependency of `roko-cli` (if not already), or
- Move the subscription dispatch types to `roko-runtime`

Check existing deps:
```bash
grep 'roko-serve' crates/roko-cli/Cargo.toml
```

## Write Scope
- `crates/roko-serve/src/` (spawn dispatch_loop at startup)
- `crates/roko-cli/src/runner/event_loop.rs` (connect StateHub → subscriptions)
- Possibly `crates/roko-runtime/src/` (if subscription types need to move)

## Read-Only Context
- `crates/roko-core/src/config/subscriptions.rs:59-138` (config types)
- `crates/roko-serve/src/dispatch.rs:786-791,1460-1530,1603-1750` (registry + dispatch loop)
- `crates/roko-serve/src/state.rs:345,369` (AppState.subscriptions)
- `crates/roko-serve/src/routes/subscriptions.rs:19-32` (CRUD routes)

## Verify
```bash
cargo build -p roko-serve 2>&1 | head -30
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-serve -- subscription 2>&1 | tail -20
```

## Acceptance Criteria
- `dispatch_loop` spawned at serve startup with StateHub event receiver
- CLI runner also connects StateHub to subscriptions when configured
- Webhook subscriptions POST event data to configured URLs
- Command subscriptions execute with event context as env vars
- Failed subscriptions logged but don't block the pipeline
- No subscriptions configured → no tokio::spawn, no overhead
- `cargo build --workspace` passes

## Do NOT
- Add new subscription types (webhook + command are sufficient)
- Change the subscription config schema
- Make subscriptions synchronous/blocking
- Create a new event bus (use existing StateHub/EventBus broadcast)
