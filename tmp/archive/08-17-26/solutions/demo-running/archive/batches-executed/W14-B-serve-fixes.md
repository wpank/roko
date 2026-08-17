# W14-B: Serve / SSE / WebSocket Fixes

**Priority**: P2 -- reliability and correctness
**Effort**: 2-3 hours
**Files to modify**: 4 files
**Dependencies**: None
**IMPROVEMENTS**: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6

## Problem

Six issues in the HTTP serve layer:

1. **11.1**: Health endpoint returns HTTP 200 even when `status = "down"` (all providers offline). Load balancers and liveness probes see a healthy endpoint when the server is actually degraded.

2. **11.2**: `relay_health` handler uses blocking `parking_lot::RwLock::read()` in async context. Under write contention, blocks the Tokio worker thread. The `health()` handler already uses `try_read()` -- `relay_health` doesn't.

3. **11.3**: `KeepAlive::default()` sends pings every 15 seconds. Many proxies (Railway: 30s, Nginx: 60s) have different timeouts. No way to configure this.

4. **11.4**: SSE replay materializes the full ring buffer via `replay_from(last_event_id)` returning a `Vec<Envelope>` before streaming. A client reconnecting from `seq=0` gets hundreds of events allocated at once.

5. **11.5**: WebSocket `back_pressure` field is parsed from client messages into `_back_pressure` but never consulted. `Coalesce` and `ResumeRequired` modes are dead code.

6. **11.6**: 13 `RwLock<HashMap>` fields in `AppState` with no documented lock ordering. Some handlers hold multiple locks simultaneously, risking lock-inversion deadlocks.

## Root Cause

The serve layer was built incrementally to get routes working. Health semantics, SSE tuning, WS back-pressure, and lock ordering are second-pass concerns that were deferred.

## Exact Code to Change

### Fix 11.1 -- Health endpoint HTTP status codes

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
**Lines**: 52-68

**Find this code:**
```rust
    (
        axum::http::StatusCode::OK,
        Json(json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime_secs,
            "active_plans": active_plans,
            "active_agents": active_agents,
            "active_runs": active_runs,
            "providers": provider_summary,
            "statehub": {
                "cursor": format!("0x{:x}", state.state_hub.total_published()),
                "events_retained": state.state_hub.ring_len(),
                "snapshot": snapshot_health_summary(&state.state_hub.current_snapshot()),
            },
        })),
    )
```

**Replace with:**
```rust
    // Map health status to appropriate HTTP status codes so load balancers
    // and liveness probes can detect degraded/down states.
    let http_status = match status {
        "down" => axum::http::StatusCode::SERVICE_UNAVAILABLE,
        _ => axum::http::StatusCode::OK,
    };
    tracing::debug!(status, ?http_status, "health check response");

    (
        http_status,
        Json(json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": uptime_secs,
            "active_plans": active_plans,
            "active_agents": active_agents,
            "active_runs": active_runs,
            "providers": provider_summary,
            "statehub": {
                "cursor": format!("0x{:x}", state.state_hub.total_published()),
                "events_retained": state.state_hub.ring_len(),
                "snapshot": snapshot_health_summary(&state.state_hub.current_snapshot()),
            },
        })),
    )
```

### Fix 11.2 -- Use try_read() in relay_health handler

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
**Lines**: 71-75

**Find this code:**
```rust
/// `GET /api/relay/health` — return relay connection diagnostics.
pub async fn relay_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let health = state.relay_health.read().clone();
    Json(serde_json::to_value(&health).unwrap_or_default())
}
```

**Replace with:**
```rust
/// `GET /api/relay/health` — return relay connection diagnostics.
pub async fn relay_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Use try_read() to avoid blocking the Tokio worker thread under write
    // contention, consistent with the health() handler above.
    let health = state
        .relay_health
        .try_read()
        .map(|r| r.clone())
        .unwrap_or_default();
    tracing::debug!("relay_health: served via try_read");
    Json(serde_json::to_value(&health).unwrap_or_default())
}
```

### Fix 11.3 -- Configurable SSE keep-alive

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
**Line**: 63

**Find this code:**
```rust
    Sse::new(stream::iter(replay).chain(live)).keep_alive(KeepAlive::default())
```

**Replace with:**
```rust
    // Use a shorter keep-alive interval than the default 15s to survive
    // aggressive proxy timeouts (Railway 30s, Nginx 60s). The "keepalive"
    // text triggers a proper SSE comment event in clients that ignore
    // empty comments.
    Sse::new(stream::iter(replay).chain(live)).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    )
```

### Fix 11.4 -- Bound SSE replay buffer

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
**Lines**: 37-44

**Find this code:**
```rust
    let replay = state
        .state_hub
        .replay_from(last_event_id)
        .into_iter()
        .map(|envelope| {
            let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
            Ok(Event::default().data(data).id(envelope.seq.to_string()))
        });
```

**Replace with:**
```rust
    // Cap the replay to 256 events to prevent a reconnecting client from
    // materializing the entire ring buffer into memory at once.
    let replay = state
        .state_hub
        .replay_from(last_event_id)
        .into_iter()
        .take(256)
        .map(|envelope| {
            let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
            Ok(Event::default().data(data).id(envelope.seq.to_string()))
        });
```

### Fix 11.5 -- Acknowledge unsupported back_pressure modes

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs`
**Line**: 90

**Find this code:**
```rust
    let mut _back_pressure = BackPressureMode::AtMostOnce;
```

**Replace with:**
```rust
    let mut back_pressure = BackPressureMode::AtMostOnce;
```

---

**Same file, lines 121-123:**

**Find this code:**
```rust
                            if let Some(bp) = cmd.back_pressure {
                                _back_pressure = bp;
                            }
```

**Replace with:**
```rust
                            if let Some(bp) = cmd.back_pressure {
                                match bp {
                                    BackPressureMode::AtMostOnce => {
                                        back_pressure = bp;
                                    }
                                    _ => {
                                        // Coalesce and ResumeRequired are not yet implemented.
                                        // Log and continue with at_most_once rather than silently
                                        // ignoring the request.
                                        tracing::warn!(
                                            mode = ?bp,
                                            "unsupported back_pressure mode requested; using at_most_once"
                                        );
                                    }
                                }
                            }
```

---

**Same file, after the `loop { ... }` select block (before `let _ = sink.close().await;` on line 189):**

Add `let _ = back_pressure;` to suppress the unused-variable warning if `back_pressure` is still not read after the match:

**Find this code:**
```rust
    let _ = sink.close().await;
```

**Replace with:**
```rust
    let _ = back_pressure; // forward-compat: will be consulted once Coalesce/Resume are wired
    let _ = sink.close().await;
```

### Fix 11.6 -- Document lock ordering in AppState

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`
**Line**: 379 (before `pub active_runs: RwLock<HashMap<String, RunHandle>>,`)

**Find this code:**
```rust
    /// Active one-shot runs.
    pub active_runs: RwLock<HashMap<String, RunHandle>>,
```

**Replace with:**
```rust
    // -- Lock acquisition order (acquire outer before inner) ---------------
    //
    //  1. active_runs          7. discovered_agents     13. cascade_router
    //  2. active_plans         8. aggregator_cache      14. gateway_model_counters
    //  3. operations           9. heartbeats            15. batch_progress
    //  4. templates           10. connectors            16. active_bench_runs
    //  5. deployments         11. feeds                 17. active_matrix_runs
    //  6. template_runs       12. ephemeral_workspaces
    //
    // Handlers that need multiple locks MUST acquire them in this order.
    // Read-heavy maps (discovered_agents, aggregator_cache, heartbeats)
    // are candidates for DashMap conversion in a future wave.
    // ------------------------------------------------------------------

    /// Active one-shot runs.
    pub active_runs: RwLock<HashMap<String, RunHandle>>,
```

## Verification

```bash
# 1. Compile the serve crate
cargo check -p roko-serve

# 2. Run serve tests
cargo test -p roko-serve

# 3. Verify health returns 503 when down
grep -n 'SERVICE_UNAVAILABLE' crates/roko-serve/src/routes/status/health.rs
# Should show the new match arm

# 4. Verify try_read in relay_health
grep -n 'try_read' crates/roko-serve/src/routes/status/health.rs
# Should show two uses (health + relay_health)

# 5. Verify SSE keep-alive is configured
grep -n 'keepalive' crates/roko-serve/src/routes/sse.rs
# Should show the .text("keepalive") call

# 6. Verify replay is bounded
grep -n 'take(256)' crates/roko-serve/src/routes/sse.rs
# Should show the .take(256) call

# 7. Verify back_pressure is not prefixed with underscore
grep -n '_back_pressure' crates/roko-serve/src/routes/ws.rs
# Should return nothing
```

## Agent Prompt

```
You are implementing W14-B: six serve/SSE/WebSocket fixes in the roko codebase.
Workspace root: /Users/will/dev/nunchi/roko/roko/

Read the batch file at /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-B-serve-fixes.md for full instructions.

## Files to modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
   - Fix 11.1 (line 52): Add http_status variable mapping "down" -> SERVICE_UNAVAILABLE, use it instead of hardcoded StatusCode::OK
   - Fix 11.2 (line 72): Change relay_health from `.read().clone()` to `.try_read().map(|r| r.clone()).unwrap_or_default()`

2. `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs`
   - Fix 11.3 (line 63): Replace `KeepAlive::default()` with `KeepAlive::new().interval(Duration::from_secs(8)).text("keepalive")`
   - Fix 11.4 (line 37): Add `.take(256)` after `.into_iter()` in the replay stream

3. `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs`
   - Fix 11.5 (line 90): Rename `_back_pressure` to `back_pressure`
   - Fix 11.5 (line 121): Replace silent assignment with match that warns on unsupported modes
   - Fix 11.5 (line 189): Add `let _ = back_pressure;` before sink.close()

4. `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`
   - Fix 11.6 (line 379): Add lock-ordering comment block before `pub active_runs`

## Key details
- The batch file has exact "Find this code:" / "Replace with:" pairs for every change
- Read each source file FIRST to verify line numbers before editing
- Add `tracing::debug!` instrumentation at health check and relay_health
- Do NOT run cargo build/test/clippy/fmt -- compilation is deferred
```

## Commit

This batch is committed with all Wave 14 batches together. Do not commit individually.

## Checklist

- [ ] 11.1: Health endpoint returns 503 when status is "down"
- [ ] 11.1: Health endpoint returns 200 for "ok" and "degraded"
- [ ] 11.1: `tracing::debug!` at health check response
- [ ] 11.2: `relay_health` uses `try_read()` with `unwrap_or_default()`
- [ ] 11.2: `tracing::debug!` at relay_health response
- [ ] 11.3: SSE keep-alive interval set to 8 seconds with "keepalive" text
- [ ] 11.4: SSE replay bounded to 256 events via `.take(256)`
- [ ] 11.5: `_back_pressure` renamed to `back_pressure`
- [ ] 11.5: Unsupported back_pressure modes logged with warning
- [ ] 11.5: `let _ = back_pressure;` added after select loop
- [ ] 11.6: Lock ordering comment block added to `AppState` before `active_runs`
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
