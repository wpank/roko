# 101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex Across Await)

**Priority**: P1 — latent crash and memory exhaustion risks in production dispatch paths
**Size**: M (2-3 days)
**Crates**: `crates/roko-agent/` (`roko-agent`), `crates/roko-runtime/` (`roko-runtime`), `crates/roko-agent-server/` (`roko-agent-server`)
**Depends on**: None

---

## Background

Tokio, the async runtime used throughout roko, has three well-known async anti-patterns that
cause real failures in production:

**Drop+Spawn**: Spawning tasks inside `Drop` implementations is unsafe. If the tokio runtime
has already shut down when an object is dropped (e.g., during program exit or an in-flight
async context moves to a sync one), `tokio::spawn` panics with "no reactor running." Even when
the runtime is alive, spawned tasks can be silently discarded during shutdown before they
complete, leaving processes un-killed.

**Unbounded channels**: `mpsc::unbounded_channel()` has no backpressure mechanism. A slow
consumer or burst producer can cause unbounded heap growth. In agent dispatch paths this can
exhaust available memory during a long-running plan.

**Mutex held across `.await`**: Holding a `tokio::sync::Mutex` guard while awaiting other
futures starves every other task waiting for that lock. In the worst case this creates a
logical deadlock: task A holds the lock while waiting on task B, and task B needs the lock.
It also causes priority inversion where latency-sensitive tasks queue behind slow I/O.

All three anti-patterns exist in the current codebase in production paths.

## Current State

### Issue 1: `tokio::spawn` inside `Drop` impls

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`, lines 799-809

```rust
impl Drop for CursorCliAgent {
    fn drop(&mut self) {
        // Best-effort kill on drop — spawn a task since drop is sync.
        let conn = self.connection.clone();
        tokio::spawn(async move {
            let mut guard = conn.lock().await;
            if let Some(mut c) = guard.take() {
                c.kill().await;
            }
        });
    }
}
```

If no tokio runtime is active at drop time this panics. If the runtime is shutting down,
the spawned task is silently discarded and the child process is not killed.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/connector_runtime.rs`, lines 162-167

```rust
impl Drop for PreparedEntry {
    fn drop(&mut self) {
        if let Some(entry) = self.0.take() {
            spawn_cleanup(entry);   // calls tokio::spawn internally
        }
    }
}
```

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/connector_runtime.rs`, lines 190-224

```rust
impl Drop for RestartRecovery {
    fn drop(&mut self) {
        if !self.armed { return; }
        let entry = Arc::clone(&self.entry);
        let registry = Arc::clone(&self.registry);
        tokio::spawn(async move {
            // async work to re-register the connector
        });
    }
}
```

### Issue 2: Unbounded channels in production paths

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`, line 290

```rust
let (resp_tx, resp_rx) = mpsc::unbounded_channel::<(u64, Value)>();
```
This is the JSON-RPC response queue for correlating request IDs to responses. At most one
response per in-flight request, so 64 is a safe bound.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`, lines 588-589

```rust
let (event_tx, event_rx) = mpsc::unbounded_channel();
let (turn_done_tx, turn_done_rx) = mpsc::unbounded_channel();
```
These are created in `CursorCliAgent::new()` — the event stream queue and turn-completion
signal queue for each agent run. The event queue has no upper bound on accumulated events
from a verbose model.

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/messaging.rs`, line 171

```rust
let (event_tx, mut event_rx) = mpsc::unbounded_channel();
let stream_task =
    tokio::spawn(async move { dispatcher.dispatch_streaming(request, event_tx).await });
```
A slow WebSocket consumer accumulates streaming chunks in memory without bound.

### Issue 3: Mutex held across `.await` points

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`, lines 642-672

```rust
async fn ensure_connected(&self) -> Result<(), String> {
    let mut conn_guard = self.connection.lock().await;   // lock acquired here
    if conn_guard.is_some() {
        return Ok(());
    }
    let _lock = cursor_startup_lock().lock().await;      // second lock acquired
    let mut conn = CursorConnection::spawn(/* ... */)
        .await?;                                         // subprocess spawn — seconds
    conn.initialize().await?;                            // I/O round trip
    conn.create_session(/* ... */).await?;               // more I/O
    *conn_guard = Some(conn);
    Ok(())
}
```

The connection mutex is held for the entire subprocess spawn + initialization + session
creation sequence, which can take 2-10 seconds. All concurrent callers block for that
entire duration.

**Same file, lines 709-716**: The `run()` method acquires mutex locks to take receivers,
then holds the receivers (not the guards) across a 60+ second timeout loop. The guards are
dropped immediately after `.take()`, which is correct — this is a note that the pattern
should be confirmed acceptable, as the comment above it ("Take the receivers") could mislead
a future reader into thinking the lock is held during the loop.

## Implementation Plan

### Step 1: Add explicit `shutdown()` to `CursorCliAgent`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`:

Add an `async fn shutdown()` method to `CursorCliAgent` (before the `impl Agent for CursorCliAgent` block, around line 673):

```rust
impl CursorCliAgent {
    /// Gracefully shut down the agent: kill the underlying Cursor process.
    /// Call this before dropping the agent from an async context.
    pub async fn shutdown(&mut self) {
        let mut guard = self.connection.lock().await;
        if let Some(mut conn) = guard.take() {
            conn.kill().await;
        }
    }
}
```

Replace the `Drop` impl (lines 799-810) with a warning-only version:

```rust
impl Drop for CursorCliAgent {
    fn drop(&mut self) {
        // Don't spawn here — it's unsafe outside a runtime.
        // Callers must call shutdown() before dropping.
        if let Ok(guard) = self.connection.try_lock() {
            if guard.is_some() {
                tracing::warn!(
                    name = %self.name,
                    "CursorCliAgent dropped without calling shutdown() — child process may be orphaned"
                );
            }
        }
    }
}
```

Find all call sites that drop `CursorCliAgent` and ensure `shutdown()` is called before
drop. Search with: `grep -rn "CursorCliAgent" crates/ --include="*.rs"`.

### Step 2: Fix `PreparedEntry` and `RestartRecovery` in connector_runtime.rs

In `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/connector_runtime.rs`:

For `PreparedEntry` (lines 162-168): `spawn_cleanup` calls `tokio::spawn`. Find the
`spawn_cleanup` function definition and determine if the cleanup is truly fire-and-forget
(acceptable with a runtime guard) or needs explicit lifecycle management. If the cleanup
is critical, expose a `pub async fn consume(self)` method and call it explicitly. If it is
truly best-effort, add `tokio::runtime::Handle::try_current()` guard:

```rust
impl Drop for PreparedEntry {
    fn drop(&mut self) {
        if let Some(entry) = self.0.take() {
            if tokio::runtime::Handle::try_current().is_ok() {
                spawn_cleanup(entry);
            } else {
                tracing::warn!("PreparedEntry dropped outside tokio runtime — cleanup skipped");
            }
        }
    }
}
```

For `RestartRecovery` (lines 190-224): Same pattern — add a runtime guard before calling
`tokio::spawn`:

```rust
impl Drop for RestartRecovery {
    fn drop(&mut self) {
        if !self.armed { return; }
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::warn!("RestartRecovery armed drop outside tokio runtime — supervisor restart skipped");
            return;
        }
        let entry = Arc::clone(&self.entry);
        let registry = Arc::clone(&self.registry);
        tokio::spawn(async move { /* existing body */ });
    }
}
```

### Step 3: Bound the production channels

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`:

Line 290 — JSON-RPC response queue: replace with bounded channel:
```rust
// Before:
let (resp_tx, resp_rx) = mpsc::unbounded_channel::<(u64, Value)>();
// After:
let (resp_tx, resp_rx) = mpsc::channel::<(u64, Value)>(64);
```

Lines 588-589 — event and turn-done queues in `CursorCliAgent::new()`:
```rust
// Before:
let (event_tx, event_rx) = mpsc::unbounded_channel();
let (turn_done_tx, turn_done_rx) = mpsc::unbounded_channel();
// After:
let (event_tx, event_rx) = mpsc::channel(512);   // events during a turn
let (turn_done_tx, turn_done_rx) = mpsc::channel(4); // at most 1 in flight
```

Note: `mpsc::channel` (bounded) returns `Result` on `send` when full; callers using
`event_tx.send(event)` must handle the error. Review all `event_tx.send(...)` and
`turn_done_tx.send(...)` call sites and add appropriate error handling (log and discard
for events; log and return error for turn-done signals).

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/messaging.rs`:

Line 171 — streaming event queue:
```rust
// Before:
let (event_tx, mut event_rx) = mpsc::unbounded_channel();
// After:
let (event_tx, mut event_rx) = mpsc::channel(256);
```

The `dispatch_streaming` call signature may need to accept a `Sender` (bounded) instead
of `UnboundedSender`. Check the trait definition for `dispatch_streaming` and update the
type. If the dispatcher is defined generically, no signature change is needed.

### Step 4: Narrow the lock scope in `ensure_connected`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs`, replace
the body of `ensure_connected` (lines 642-672):

```rust
async fn ensure_connected(&self) -> Result<(), String> {
    // Fast path: check under a brief lock scope.
    {
        let guard = self.connection.lock().await;
        if guard.is_some() {
            return Ok(());
        }
    }
    // Slow path: serialize concurrent spawns via the startup lock.
    // The connection mutex is NOT held during spawn/initialize/create_session.
    let _startup = cursor_startup_lock().lock().await;
    // Double-check now that we hold the startup lock.
    {
        let guard = self.connection.lock().await;
        if guard.is_some() {
            return Ok(());
        }
    }
    tracing::info!(
        "[cursor-cli] spawning agent: {} --force --approve-mcps --workspace {} acp",
        self.command,
        self.working_dir.display()
    );
    let mut conn = CursorConnection::spawn(
        &self.command,
        &self.working_dir,
        self.model.as_deref(),
        self.event_tx.clone(),
        self.turn_done_tx.clone(),
        self.resource_limits.as_ref(),
    )
    .await?;
    conn.initialize().await?;
    conn.create_session(&self.working_dir.to_string_lossy(), &self.mcp_servers)
        .await?;
    // Re-acquire connection lock only to store the result.
    *self.connection.lock().await = Some(conn);
    Ok(())
}
```

## Acceptance Criteria

1. No `tokio::spawn` in `Drop` impls without a `Handle::try_current()` guard.
2. `CursorCliAgent::Drop` does not spawn; it logs a warning if the connection was not
   explicitly shut down via `shutdown()`.
3. All three `unbounded_channel` call sites in `cursor_cli_agent.rs` replaced with
   bounded channels.
4. The `messaging.rs` streaming event channel replaced with a bounded channel.
5. `ensure_connected()` does not hold the connection mutex during `spawn`, `initialize`,
   or `create_session`.
6. `cargo test -p roko-agent` passes with no new failures.
7. `cargo test -p roko-agent-server` passes with no new failures.
8. New test added to `cursor_cli_agent.rs` tests: drop a `CursorCliAgent` outside any
   tokio runtime (use `std::thread::spawn` + `block_on` then drop outside) and verify
   no panic.

## Verification Checklist

- [ ] Run `grep -rn "tokio::spawn" crates/roko-agent/src/ crates/roko-runtime/src/ --include="*.rs"` and confirm no spawn call is inside a `fn drop(&mut self)` without a runtime guard
- [ ] Run `grep -rn "unbounded_channel" crates/roko-agent/src/ crates/roko-agent-server/src/ --include="*.rs"` and confirm no production-path unbounded channels remain
- [ ] Manually test `roko agent start --name <name>` and `roko agent stop --name <name>` to verify the shutdown path is exercised
- [ ] Run `cargo test -p roko-agent -- --nocapture 2>&1 | grep -i "panic\|FAILED"` and confirm clean
- [ ] Run `cargo test -p roko-agent-server -- --nocapture 2>&1 | grep -i "panic\|FAILED"` and confirm clean
- [ ] Run `cargo clippy -p roko-agent -p roko-agent-server -p roko-runtime --no-deps -- -D warnings` and confirm clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_cli_agent.rs` | Add `shutdown()` method; rewrite `Drop` to warning-only; replace 3 unbounded channels with bounded; narrow `ensure_connected` lock scope |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/connector_runtime.rs` | Add `Handle::try_current()` guard before `tokio::spawn` in `PreparedEntry::drop` and `RestartRecovery::drop` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/messaging.rs` | Replace unbounded streaming event channel with bounded channel (capacity 256) |
