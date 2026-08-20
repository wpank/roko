# 50 — Serve Rate and Body Limits

**Priority**: P1 — security: expensive endpoints (terminal PTY, inference, agent registration) lack per-route rate limits
**Size**: S (1 day)
**Crates**: `crates/roko-serve` (`src/routes/mod.rs`, `src/routes/terminal.rs`, `src/lib.rs`)
**Depends on**: None

---

## Background

The HTTP control plane has global rate limiting via the `governor` crate and a global body size cap. Every request regardless of route is subject to these limits. The global limits exist to bound the total throughput any single caller can consume.

The problem is that the global budget is shared across all routes. An attacker who stays within the global rate limit (100 req/s global, 30 req/s per-key) can still abuse individual expensive endpoints. The most sensitive example is terminal PTY creation: each `POST /api/terminal/sessions` spawns a real shell process and allocates a PTY. A caller at 30 req/s per key can open hundreds of PTY sessions per minute while remaining under the global limit.

Inference dispatch (`POST /api/gateway/infer`, `POST /api/run`) is similarly expensive — each request triggers an LLM API call. Agent registration (`POST /agents/register`, `POST /agents/create`) performs disk writes and registry operations. Signal injection (`POST /api/signals`) is lightweight but can be high-volume.

The terminal input endpoint (`POST /api/terminal/sessions/{id}/input`) also lacks a body limit tuned to its use case: the global 4 MiB cap is large for keystroke/paste payloads that are forwarded directly to a PTY.

The global and per-key rate limits are currently hardcoded constants (`DEFAULT_GLOBAL_RATE_PER_SEC = 100`, `DEFAULT_PER_KEY_RATE_PER_SEC = 30`). Making them configurable via `[server]` config would allow operators to tune them without recompiling.

## Current State

1. `DEFAULT_REQUEST_BODY_LIMIT_BYTES = 4 * 1024 * 1024` (4 MiB) at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs:95`. Applied via `DefaultBodyLimit::max()` at line 420 in `build_router()`.

2. `DEFAULT_GLOBAL_RATE_PER_SEC = 100` at line 101. Applied as middleware at line 422 in `build_router()`.

3. `DEFAULT_PER_KEY_RATE_PER_SEC = 30` at line 107. Applied as middleware at line 427 in `build_router()`.

4. Both rate limiters use the `governor` crate. `build_global_rate_limiter()` is at line 118, `build_keyed_rate_limiter()` at line 125. `rate_limit_middleware()` is at line 189, `keyed_rate_limit_middleware()` at line 212.

5. The 429 responses at lines 195-202 and 219-226 return a JSON body with `code: "rate_limited"` but NO `Retry-After` header.

6. A per-route webhook body override (`WEBHOOK_BODY_LIMIT_BYTES = 1 MiB`) exists in `crates/roko-serve/src/routes/webhooks.rs:22` — this is the correct pattern to follow for other route groups.

7. No per-route rate limits exist for:
   - Terminal creation (`POST /api/terminal/sessions`) and terminal WS upgrade (`GET /ws/terminal/{id}`)
   - Inference dispatch (`POST /api/gateway/infer`, `POST /api/run`)
   - Agent registration (`POST /agents/register`, `POST /agents/create`)
   - Signal injection (`POST /api/signals`)

8. The terminal WS handler at `crates/roko-serve/src/routes/terminal.rs` applies WS message/frame size limits via `apply_ws_size_limits()` (from `crates/roko-serve/src/routes/ws.rs:38`) — those are correct and should not change.

9. Rate limit tests at lines 1900 and 2131 in `routes/mod.rs` verify basic 429 behavior and per-key isolation.

## Implementation Plan

**Step 1: Add `Retry-After` header to 429 responses**

The `governor` crate's `check()` and `check_key()` return a `Result<(), NotUntil<...>>` where the error contains a `wait_time()` duration. Update `rate_limit_middleware()` and `keyed_rate_limit_middleware()` in `routes/mod.rs` to extract the wait time and include it in the response:

```rust
// In rate_limit_middleware:
if let Err(not_until) = limiter.check() {
    let retry_secs = not_until.wait_time_from(governor::clock::DefaultClock::default().now())
        .as_secs()
        .max(1);
    return Err(ApiError {
        status: StatusCode::TOO_MANY_REQUESTS,
        code: "rate_limited".into(),
        message: ...,
        details: None,
    }.with_header("Retry-After", retry_secs.to_string()));
}
```

If `ApiError` does not support adding headers, return a manual `(StatusCode, HeaderMap, Json<...>)` tuple instead. Check how other error responses add headers in the codebase before choosing an approach.

**Step 2: Add per-route rate limiters for expensive endpoints**

Create a helper function in `routes/mod.rs` that builds a keyed rate limiter from a `(per_minute, burst)` pair (to match the suggested limits, which are per-minute not per-second):

```rust
fn build_keyed_rate_limiter_per_minute(per_minute: u32, burst: u32) -> Arc<KeyedRateLimiter> {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute.max(1)).unwrap())
        .allow_burst(NonZeroU32::new(burst.max(1)).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}
```

Apply per-route-group rate limits using `axum::middleware::from_fn_with_state` on the sub-routers. The suggested limits:

| Route group | Per-key limit | Burst | Rationale |
|---|---|---|---|
| Terminal creation + WS upgrade | 2/min | 3 | Each spawns a PTY process |
| Inference dispatch (gateway/infer, run) | 30/min | 10 | Each triggers an LLM call |
| Agent registration (register, create) | 5/min | 5 | Disk writes + registry ops |
| Signal injection | 20/s | 40 | High-volume but cheap |

Find where each route group is assembled. In `build_router()` in `routes/mod.rs`, sub-routers from each module (e.g. `terminal::routes()`, `agents::routes()`, `run::routes()`) are merged via `.merge()`. Wrap each group before merging:

```rust
let terminal_routes = terminal::routes()
    .layer(axum::middleware::from_fn_with_state(
        build_keyed_rate_limiter_per_minute(2, 3),
        keyed_rate_limit_middleware,
    ));
```

**Step 3: Add a terminal-specific body limit**

In `routes/terminal.rs`, identify the terminal input endpoint (`POST /api/terminal/sessions/{id}/input`). Apply `DefaultBodyLimit::max(256 * 1024)` (256 KiB) on the terminal input sub-router. The pattern to follow is the webhook override in `webhooks.rs:383`.

Alternatively, apply it in `build_router()` when constructing the terminal route group:

```rust
let terminal_routes = terminal::routes()
    .layer(DefaultBodyLimit::max(256 * 1024))
    .layer(axum::middleware::from_fn_with_state(...));
```

**Step 4: Make global and per-key limits configurable**

In `crates/roko-core/src/config/schema.rs`, add to `ServerConfig`:

```rust
/// Global rate limit (requests per second). Defaults to 100.
#[serde(default = "ServerConfig::default_rate_limit_per_sec")]
pub rate_limit_per_sec: u32,
/// Per-key rate limit (requests per second per API key or IP). Defaults to 30.
#[serde(default = "ServerConfig::default_rate_limit_per_key_per_sec")]
pub rate_limit_per_key_per_sec: u32,
```

In `build_router()`, replace the hardcoded constants with values from `state.load_roko_config().server.rate_limit_per_sec` and `.rate_limit_per_key_per_sec`.

**Step 5: Add tests**

Add tests that verify:
- Terminal creation routes return 429 after exceeding the per-terminal rate limit
- Inference dispatch routes return 429 after exceeding the per-inference rate limit
- 429 responses include a `Retry-After` header with a positive integer value

## Acceptance Criteria

1. Terminal creation (`POST /api/terminal/sessions`) and WS upgrade (`GET /ws/terminal/{id}`) have a per-key rate limit of approximately 2/min burst 3.
2. Inference dispatch routes (`POST /api/gateway/infer`, `POST /api/run`) have a per-key rate limit of approximately 30/min burst 10.
3. Agent registration routes (`POST /agents/register`, `POST /agents/create`) have a per-key rate limit of approximately 5/min burst 5.
4. Terminal input (`POST /api/terminal/sessions/{id}/input`) has a body limit of 256 KiB.
5. Global and per-key rate limits are configurable via `[server]` config fields `rate_limit_per_sec` and `rate_limit_per_key_per_sec`.
6. All 429 responses include a `Retry-After` header with a non-zero integer.
7. `cargo test -p roko-serve` passes.
8. New tests cover per-route rate limits returning 429 and the `Retry-After` header.

## Verification Checklist

- [ ] Start `roko serve`; use a loop to send 4+ terminal creation requests within a few seconds; verify the 4th returns 429 with a `Retry-After` header
- [ ] Verify the 429 response body contains `"code": "rate_limited"`
- [ ] Start `roko serve` with `[server] rate_limit_per_sec = 2` in `roko.toml`; verify the third request to `/api/status` (or any route) returns 429
- [ ] Run `cargo test -p roko-serve` — all tests pass including new per-route rate limit tests
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` | Add `Retry-After` header to `rate_limit_middleware()` (line 189) and `keyed_rate_limit_middleware()` (line 212); add `build_keyed_rate_limiter_per_minute()` helper; apply per-route limiters in `build_router()` (line 238); read rate limits from config instead of constants |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/terminal.rs` | Apply 256 KiB body limit on terminal input endpoint |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Add `rate_limit_per_sec` and `rate_limit_per_key_per_sec` fields to `ServerConfig` |
