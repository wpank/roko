# Production Audit Findings

Complete audit of roko for production deployment. Each finding links to the implementation plan in `10-IMPLEMENTATION-PLAN.md`.

## Critical (blocks production deploy)

### C1: Model routing falls back to unavailable providers
- **Impact**: Agent dispatch fails mid-task with cryptic "no API key" error
- **Root cause**: 7 hardcoded fallbacks to `"claude-sonnet-4-6"` without checking key availability. CascadeRouter has zero awareness of which providers have keys.
- **Files**: `roko-core/src/config/agent.rs:110-120`, `roko-cli/src/orchestrate.rs:14630-14641`, `roko-cli/src/model_selection.rs:330`, `roko-agent/src/dispatch_resolver.rs:103-107`, `roko-core/src/agent.rs:126-141`
- **Details**: See `02-MODEL-ROUTING-FIX.md`
- **Plan**: Task P1 in implementation plan

### C2: No file locking for concurrent state writes
- **Impact**: Multiple roko instances corrupt `.roko/` state files (episodes.jsonl, efficiency.jsonl, cascade-router.json)
- **Root cause**: All mutexes are process-local (`tokio::sync::Mutex`). No `flock()` or distributed locking.
- **Files**: `roko-learn/src/episode_logger.rs:921-923`, `roko-fs/src/file_substrate.rs:29`, `roko-learn/src/feedback_service.rs:152-172`, `roko-learn/src/cascade_router.rs:1599`
- **Plan**: Task P5 in implementation plan

### C3: Auth disabled by default with no warning on public bind
- **Impact**: All 85+ API routes unprotected when deployed. Terminal endpoint allows arbitrary command execution.
- **Root cause**: `serve.auth.enabled = false` in roko.toml. No warning when binding to `0.0.0.0` with auth off (only auto-enables if Privy credentials exist).
- **Files**: `roko-serve/src/lib.rs:124-132`, `roko-serve/src/lib.rs:772-785`
- **Plan**: Task P6 in implementation plan

## High (should fix before production)

### H1: No request timeouts or body size limits on HTTP server
- **Impact**: Hanging handlers block indefinitely. Large payloads exhaust memory.
- **Root cause**: No tower timeout middleware. No `DefaultBodyLimit`. Axum defaults are unbounded.
- **Files**: `roko-serve/src/lib.rs` (router construction)
- **Plan**: Task P7 in implementation plan

### H2: No rate limiting on any endpoint
- **Impact**: Brute force, resource exhaustion, unlimited PTY session spawning
- **Root cause**: No rate limiting middleware configured
- **Files**: `roko-serve/src/lib.rs`, `roko-serve/src/terminal.rs`
- **Plan**: Task P7 in implementation plan

### H3: Silent error swallowing in critical paths
- **Impact**: Failures invisible in production. JWKS refresh silently fails (security-critical). Bootstrap failures ignored.
- **Root cause**: 25+ instances of `let _ = ...` and `.ok()` without logging in roko-serve and orchestrate.rs
- **Key locations**: `roko-serve/src/lib.rs:830` (bootstrap), `roko-serve/src/jwks.rs:145` (JWKS), `roko-serve/src/terminal.rs:279-591` (terminal ops)
- **Plan**: Task P8 in implementation plan

### H4: Context overflow not handled
- **Impact**: If prompt exceeds model context window, task fails permanently with no retry
- **Root cause**: No pre-truncation of prompts. `ContextOverflow` is non-retryable. Truncation only applies to tool output, not input.
- **Files**: `roko-agent/src/dispatcher/truncate.rs` (output only), no input truncation exists
- **Plan**: Task P9 in implementation plan

### H5: 92 `expect()` calls in orchestrate.rs
- **Impact**: Any of these panics in production = process crash
- **Root cause**: Rapid development prioritized `expect()` over proper error propagation
- **Files**: `roko-cli/src/orchestrate.rs` (92 locations)
- **Plan**: Task P10 in implementation plan

### H6: Mutex lock unwraps will panic on poisoning
- **Impact**: After any panic while holding a lock, all subsequent operations on that lock crash
- **Key locations**: `roko-cli/src/dispatch/warm_pool.rs:109-155` (4 locations), `roko-cli/src/orchestrate.rs:1437-1441` (enrichment stats), `roko-cli/src/orchestrate.rs:17388` (gate sink)
- **Plan**: Task P10 in implementation plan

## Medium (should fix, not blocking)

### M1: Unbounded log files
- **Impact**: Docker volumes fill up over days/weeks
- **Root cause**: `episodes.jsonl`, `signals.jsonl`, `efficiency.jsonl` are append-only with no auto-rotation. GC exists but must be called manually.
- **Files**: `roko-fs/src/gc.rs` (GC logic exists but not auto-triggered)
- **Plan**: Task P11 in implementation plan

### M2: No state schema migration
- **Impact**: Future schema changes break resume from old snapshots
- **Root cause**: Only `LayoutVersion::V1` exists. `ExecutorSnapshot` rejects version mismatches with no upgrade path.
- **Files**: `roko-fs/src/layout.rs:24-56`, `roko-cli/src/orchestrate.rs:764`
- **Plan**: Not blocking for initial deploy

### M3: SPA path traversal risk
- **Impact**: Potential directory escape in static file serving
- **Root cause**: `embedded.rs:read_from_disk()` uses `dir.join(path)` without canonicalization
- **Files**: `roko-serve/src/embedded.rs:50-67`
- **Plan**: Task P6 in implementation plan

### M4: SSE streams bypass secret scrubbing
- **Impact**: If event payloads contain secrets, they leak unredacted
- **Root cause**: `text/event-stream` intentionally skipped in scrub middleware to avoid buffering
- **Files**: `roko-serve/src/routes/middleware.rs:495`
- **Plan**: Task P8 in implementation plan

### M5: `eprintln!()` instead of structured logging
- **Impact**: ~50 locations bypass log aggregation, alerting, observability
- **Root cause**: Quick debugging output never migrated to `tracing::*`
- **Files**: `roko-cli/src/orchestrate.rs:6077-7785` (conductor, health, fleet status)
- **Plan**: Task P12 in implementation plan

### M6: Dockerfile doesn't build demo-app
- **Impact**: Embedded SPA assets are empty unless manually pre-built
- **Root cause**: Dockerfile only has Rust build stage, no Node stage
- **Files**: `Dockerfile` (root)
- **Plan**: See `04-DOCKERFILE-FIX.md`

### M7: Health endpoint returns 200 when providers are "down"
- **Impact**: Load balancers think the service is healthy when it can't do useful work
- **Root cause**: Status embedded in JSON body, HTTP code always 200
- **Files**: `roko-serve/src/routes/status/health.rs`
- **Note**: `roko serve` exposes **`GET /health`** (minimal liveness, root) **and** **`GET /api/health`** (under the API nest). Railway’s `healthcheckPath` must match the route your **process** actually serves; see **09-OPERATIONS-RUNBOOK.md**.
- **Plan**: Task P7 in implementation plan

### M8: `todo!()` and `unimplemented!()` in non-test code
- **Impact**: Panic if these code paths are hit
- **Files**: `roko-serve/src/routes/bench.rs:800,827`
- **Plan**: Task P10 in implementation plan

## Low (nice to have)

### L1: Stale temp files after crash
- `*.tmp` files from atomic writes linger if process killed between write and rename
- Fix: startup cleanup routine

### L2: No explicit SIGTERM handler
- Relies on Rust defaults. Works but could be more explicit for containers.

### L3: WebSocket idle timeout
- No disconnect for clients that connect but never read. Buffer pressure possible.

### L4: Wave-based concurrency instead of true concurrency
- 8-task waves block on slowest task. Not a bug, but suboptimal throughput.
