# 83 — Dream Consolidation Hangs Indefinitely (Tokio Deadlock)

**Priority**: P2 — all dream-based learning from successful plan runs is lost; the fix is a one-line change
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli` (path: `src/runner/event_loop.rs`), `crates/roko-dreams` (path: `src/runner.rs`)
**Depends on**: None

---

## Background

After every successful `roko plan run`, the runner attempts a "dream consolidation" pass that reads recent episodes, synthesizes patterns, and writes insights to the knowledge store. This learning step is what allows roko to improve over time. Currently it always fails with:

```
dream consolidation timed out — skipping timeout_secs=600
```

The 600-second timeout fires every time because dream consolidation deadlocks immediately. This is a classic tokio anti-pattern: calling `block_on` inside `spawn_blocking` when the inner async work spawns child tasks. The result is that roko runs plans but never learns from them.

The fix is one line at the call site in `event_loop.rs`.

## Current State

The deadlock chain is:

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 13997: `run_dream_consolidation()` calls `tokio::task::spawn_blocking(move || { ... })` to run dream consolidation off the async executor.

2. Inside that `spawn_blocking` closure, `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs` line 901: `DreamRunner::consolidate_now()` calls `block_on(self.consolidate_async())`.

3. `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs` lines 1492-1505: The `block_on` helper detects a running tokio handle and uses `tokio::task::block_in_place()` to drive the future — `tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))`.

4. Inside `consolidate_async()`, the dream cycle spawns a `ClaudeCliAgent` subprocess. The subprocess I/O needs tokio executor threads to poll stdout/stderr.

The deadlock: `spawn_blocking` holds a blocking thread pool slot. `block_in_place` inside it blocks an executor thread. The `ClaudeCliAgent` subprocess needs executor threads to poll its I/O. If all executor threads are occupied (or if the threadpool accounting doesn't permit additional work), the system deadlocks — the blocking task waits for async I/O that needs threads that are blocked waiting for the blocking task.

The code at `event_loop.rs` lines 13975-14027:

```rust
// Current (deadlocks):
async fn run_dream_consolidation(config: &RunConfig, telemetry: &dyn TelemetryEventSink) {
    // ...setup...
    let join = tokio::task::spawn_blocking(move || {
        let mut dream_runner = roko_dreams::DreamRunner::new(workdir.clone(), dream_config);
        dream_runner.consolidate_now()   // <-- calls block_on internally
    });
    match tokio::time::timeout(timeout, join).await {
        // ...
    }
}
```

The `DreamRunner::consolidate_now()` signature at `crates/roko-dreams/src/runner.rs` line 901:

```rust
pub fn consolidate_now(&mut self) -> Result<DreamReport> {
    block_on(self.consolidate_async())
}
```

And `consolidate_async()` at line 1038 is an `async fn`. The `DreamRunner` type needs to be checked for `Send` bounds before choosing the fix.

## Implementation Plan

### Option A — Make the top-level call directly async (recommended)

Check whether `DreamRunner` is `Send`. If it is (it should be — it holds `PathBuf` and config data), change `run_dream_consolidation()` to spawn a direct async task instead of using `spawn_blocking`:

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

```rust
// After (runs on the async executor directly):
async fn run_dream_consolidation(config: &RunConfig, telemetry: &dyn TelemetryEventSink) {
    let workdir = config.workdir.clone();
    let timeout = config.roko_config.as_deref().map_or_else(
        || roko_core::config::TimeoutConfig::default().dream_consolidation(),
        |cfg| cfg.timeouts.dream_consolidation(),
    );
    let dream_config = roko_dreams::DreamLoopConfig { /* ...same as before... */ };

    // Use tokio::spawn instead of spawn_blocking.
    // consolidate_async() is an async fn and should run on the executor directly.
    let join = tokio::spawn(async move {
        let mut dream_runner = roko_dreams::DreamRunner::new(workdir.clone(), dream_config);
        dream_runner.consolidate_async().await
    });
    match tokio::time::timeout(timeout, join).await {
        Ok(Ok(Ok(report))) => { /* success log */ }
        Ok(Ok(Err(err))) => warn!(error = %err, "dream consolidation failed"),
        Ok(Err(join_err)) => warn!(error = %join_err, "dream consolidation worker aborted"),
        Err(_) => warn!(timeout_secs = duration_secs(timeout), "dream consolidation timed out — skipping"),
    }
}
```

This requires exposing `consolidate_async()` as `pub` in `DreamRunner`. It's currently private (only `consolidate_now()` is `pub` at line 901). Change `async fn consolidate_async` to `pub async fn consolidate_async` in `crates/roko-dreams/src/runner.rs` at line 1038.

### Option B — Separate tokio runtime for dream work (fallback)

If `DreamRunner` is `!Send` due to non-Send fields (check by attempting to compile Option A and observing errors), use a dedicated single-threaded runtime on a spawned OS thread:

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

```rust
let join = std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("dream runtime");
    let mut dream_runner = roko_dreams::DreamRunner::new(workdir.clone(), dream_config);
    rt.block_on(dream_runner.consolidate_async())
});
// Then wrap with timeout:
match tokio::time::timeout(timeout, tokio::task::spawn_blocking(move || join.join())).await {
    // ...
}
```

Option B avoids all runtime contention because the dream work runs on its own independent runtime.

### Option C — Fire-and-forget (minimum change, maximum isolation)

If neither A nor B is feasible quickly, convert to a fire-and-forget task. The plan has already succeeded by the time dream consolidation runs, so it's non-blocking:

```rust
// In run_dream_consolidation_if_enabled(), instead of awaiting run_dream_consolidation():
tokio::spawn(async move {
    run_dream_consolidation(&config_clone, &telemetry_clone).await;
});
// Don't await — let it run in background after the plan completion report
```

This doesn't fix the deadlock but at least doesn't block the CLI from returning to the user after plan completion.

### Recommended order

Try Option A first. If `DreamRunner` is `Send`, it's a two-line change. If it's not, use Option B.

## Acceptance Criteria

1. `roko plan run plans/demo-hello --fresh` completes and dream consolidation does NOT time out.
2. After the plan completes, the log contains `dream consolidation completed` with non-zero `processed_episodes`.
3. `.roko/learn/` contains updated dream artifacts after a successful consolidation.
4. `cargo test -p roko-dreams` passes.
5. `cargo test -p roko-cli` passes.

## Verification Checklist

- [ ] Run a plan to completion: `cargo run -p roko-cli -- plan run plans/ --engine runner-v2`
- [ ] After "Plan complete" message, watch logs — `dream consolidation completed` should appear within the configured timeout
- [ ] Verify `.roko/learn/` has a new or updated dream report file (timestamp newer than the plan start)
- [ ] Verify the dream confidence levels in the knowledge store are advancing over multiple runs (not stuck at 0.10-0.30)
- [ ] Confirm no `spawn_blocking` wraps an async function that itself uses `block_in_place` in the dream path

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Change `run_dream_consolidation()` at line 13997 from `tokio::task::spawn_blocking(|| dream_runner.consolidate_now())` to `tokio::spawn(async move { dream_runner.consolidate_async().await })` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs` | Change `async fn consolidate_async` to `pub async fn consolidate_async` at line 1038 so the caller in `event_loop.rs` can call it directly |
