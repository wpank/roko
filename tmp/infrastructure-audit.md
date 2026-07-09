# Infrastructure & Anti-Pattern Audit

**Date**: 2026-05-02
**Scope**: Dev workflow, workspace lifecycle, Docker/Railway deployment, Rust anti-patterns, demo app anti-patterns, testing gaps, tool system, orchestration pipeline, concurrency patterns, learning subsystem, prompt assembly, safety contracts, ACP/Zed integration

**Total issues cataloged**: 185+

---

## 1. Dev Workflow (`roko-dev-full`)

### Problems

**Triple process spawn**: `cargo watch -w crates/ -x "build -p roko-cli" -s "./target/debug/roko serve"` fires multiple rebuild+serve cycles when many files change simultaneously. Each cycle spawns a new `roko serve` before the previous one exits, causing:
- "Address already in use" on port 6677
- Cargo lock contention (`Blocking waiting for file lock on build directory`)
- Zombie `roko serve` processes left running after `ctrl-c`

**No process lifecycle management**: The shell alias uses `trap "kill 0" EXIT` which sends SIGTERM to the process group, but:
- Background `cargo watch` may not propagate signals to child `roko serve`
- No PID file or port-based stale process detection
- No graceful shutdown sequence

**Port binding**: `roko serve` binds port 6677 with no `SO_REUSEADDR`, no retry logic, and no exponential backoff. If the previous instance hasn't released the port, the new one crashes immediately.

### Redesign

Replace the shell alias with a proper dev orchestrator:

```
roko dev
  ├── cargo watch -w crates/ -x "build -p roko-cli"  (rebuild only, no serve)
  ├── roko serve (managed, auto-restart on binary change)
  │   ├── detect stale port, kill previous instance
  │   ├── SO_REUSEADDR + retry with backoff
  │   └── graceful shutdown on SIGTERM
  └── cd demo/demo-app && npm run dev
```

Key properties:
- **Build and serve are separate** — cargo watch only rebuilds, a file-watcher restarts serve when the binary changes
- **PID file** at `.roko/serve.pid` — new instance kills old before binding
- **Port retry** with exponential backoff (100ms, 200ms, 400ms, max 5 attempts)
- **Signal propagation** — parent process sends SIGTERM to all children, waits for exit, then exits itself

---

## 2. Workspace Lifecycle

### Problems

**In-memory only**: `ephemeral_workspaces: RwLock<HashMap<...>>` in `AppState` is empty on every server restart. No persistence.

**macOS temp_dir mismatch**: `std::env::temp_dir()` returns `/var/folders/bn/.../T/` on macOS, not `/tmp/`. The demo UI assumes `/tmp/roko-ws-*` paths. After reboot or cleanup, workspace dirs vanish.

**No workspace recovery**: When a workspace path becomes invalid (server restart, temp cleanup), the API returns 404. The demo UI shows a terminal error (`cd: no such file or directory`) with no recovery path.

**Blind config copy** (B5): `POST /api/workspaces` copies `roko.toml` verbatim. If the copy fails, it silently uses defaults. Env var overrides (`ROKO__*`) are lost.

**1-hour GC interval**: `WorkspaceGc` runs every 60 minutes. In dev, workspaces are created and abandoned in seconds. Stale workspace dirs accumulate.

### Redesign

**Persistent workspace registry**:
```
.roko/workspaces.json
{
  "workspaces": {
    "prd-pipeline-abc123": {
      "path": "/tmp/roko-ws-abc123",
      "created": "2026-05-02T10:00:00Z",
      "last_accessed": "2026-05-02T10:05:00Z",
      "config_hash": "sha256:...",
      "status": "active"
    }
  }
}
```

Key properties:
- **Survives restart** — loaded from disk on serve startup
- **Validates on access** — if path doesn't exist, workspace is re-created or marked stale
- **Resolved config** — serialize the server's effective config (including env var overrides) to workspace `roko.toml`, not a blind file copy
- **Shorter GC** — 5 minutes for dev, configurable for prod
- **Workspace reattach** — demo UI sends workspace ID, server returns the same workspace if still valid or creates a new one

---

## 3. Terminal / PTY Sessions

### Problems

**No session reattach**: PTY sessions are destroyed on WebSocket disconnect. Page refresh = new terminal, old session lost. The demo UI does `ensureTerminal()` on mount which creates a fresh session every time.

**Session generation counter resets on restart**: `AtomicU64` in `TerminalSessionManager` starts at 0 on every server restart. No way to distinguish sessions from different server lifetimes.

**ZDOTDIR leak**: Terminal sessions set `ZDOTDIR` to a temp dir for `.zshrc`, but never clean it up.

**Indefinite reconnect**: WebSocket reconnect loop in `useTerminal.ts` has no max retries, 500ms fixed interval. If the server is down for minutes, the browser hammers it with reconnect attempts.

### Redesign

**Session persistence with tmux/screen backend**:
- PTY sessions backed by tmux sessions named by workspace ID
- WebSocket reconnect reattaches to existing tmux session
- Session survives server restart (tmux is independent)
- Cleanup: tmux sessions killed when workspace is GC'd

**Fallback (no tmux)**: Session state (CWD, env, scrollback buffer) persisted to `.roko/workspaces/{id}/terminal.state`. On reconnect, restore CWD and replay last N lines of scrollback.

**WebSocket reconnect**: Exponential backoff (500ms → 1s → 2s → 4s → max 30s), max 20 retries, then show "Server unreachable" with manual retry button.

---

## 4. Config Loading (B4)

### Problems

**Two loaders**: `roko serve` uses `load_roko_config()` which ignores `ROKO__*` env vars, `ROKO_CONFIG` path override, and ancestor directory search. `roko` CLI uses `load_layered()` which does all of these. Same machine, different effective config.

**Two config files**: `roko.toml` (700+ lines, local dev) and `docker/railway.roko.toml` (109 lines, Railway). They drift independently — railway.toml has no `max_output` on any model, stale `context_window` values, and different config version (2 vs 1).

**No config validation on load**: If `roko.toml` has invalid TOML or missing required fields, errors surface deep in the call stack, not at startup.

### Redesign

**Single config loader** in `roko-core`:
```rust
// roko-core/src/config/loader.rs
pub fn load_config(workdir: &Path) -> Result<RokoConfig> {
    // 1. Find config: ROKO_CONFIG env var > ancestor search > workdir/roko.toml
    // 2. Parse TOML
    // 3. Apply ROKO__* env var overrides
    // 4. Validate schema
    // 5. Return validated config
}
```

Both `roko serve` and `roko` CLI call this. CLI's `load_layered` becomes a thin wrapper adding `ResolvedConfig`/`ConfigSources` on top.

**Single config file**: `docker/railway.roko.toml` is generated from `roko.toml` via `roko config export --profile railway` which strips dev-only settings and adds Railway-specific values. Never hand-edit the railway config.

---

## 5. Docker & Railway Deployment

### Problems

**3 sidecars in 1 container**: `start-railway.sh` runs `roko serve`, `mirage-rs`, and `agent-relay` in a single container. No process supervision, no health checks, no restart on crash.

**Entire Rust toolchain in runtime image**: Dockerfile copies the toolchain into the runtime stage. Adds ~1-2GB to the image.

**Bind-mount mismatch**: `docker-compose.dev.yml` mounts `./target/release/roko` but `roko-dev-full` does debug builds → mounted binary is stale or missing.

**Health/readiness was incomplete**: `roko serve` had a top-level `/health` liveness route and richer `/api/health`, but no top-level `/ready` route for load balancer readiness/drain behavior. `roko up` also started serve through a wrapper task and aborted it on Ctrl+C instead of using the same graceful cancellation path as `roko serve`.

**CORS hardcoded**: `CorsLayer` allows all origins in dev. In prod, this should be restricted.

**SPA embedding at compile time**: `rust-embed` bakes `demo/demo-app/dist/` into the binary. Changing the frontend requires rebuilding the entire Rust binary.

### Redesign

**Multi-stage Dockerfile** (proper):
```dockerfile
# Stage 1: Frontend build
FROM node:22-alpine AS frontend
WORKDIR /app/demo/demo-app
COPY demo/demo-app/package*.json .
RUN npm ci
COPY demo/demo-app/ .
RUN npm run build

# Stage 2: Rust build
FROM rust:1.91 AS backend
WORKDIR /app
COPY . .
COPY --from=frontend /app/demo/demo-app/dist demo/demo-app/dist
RUN cargo build --release -p roko-cli

# Stage 3: Runtime (NO toolchain)
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/target/release/roko /usr/local/bin/roko
COPY --from=backend /app/docker/railway.roko.toml /etc/roko/roko.toml
```

**Process supervision**: Use `tini` as PID 1 + a simple process manager (or just run roko serve as the only process, with mirage-rs and agent-relay as separate Railway services).

**Health/readiness endpoints**: `GET /health` is an unauthenticated liveness probe returning status/version/uptime, and `GET /ready` is an unauthenticated readiness probe returning `503` with `status = "shutting_down"` once the server cancellation token is tripped. Rich telemetry remains under `GET /api/health`.

**CORS from config**: `[serve.cors]` section in `roko.toml`:
```toml
[serve.cors]
allowed_origins = ["http://localhost:5173"]  # dev
# allowed_origins = ["https://roko.nunchi.dev"]  # prod
```

**Batch 37 update (2026-05-04)**: `crates/roko-serve/src/routes/mod.rs` now registers top-level unauthenticated `GET /health` and `GET /ready`. `/health` reports process liveness with `status`, `version`, and `uptime_secs`; `/ready` reports the same healthy shape while running and returns `503`/`shutting_down` after `AppState.cancel` is cancelled. `crates/roko-cli/src/commands/server.rs` now starts `roko up` serve through `roko_serve::start_server_background()` and waits for the returned server task after cancellation instead of aborting the task. Verified with `cargo test -p roko-serve top_level --jobs 1 -- --nocapture`, `cargo test -p roko-serve health_reports_status_version_uptime_and_counts --jobs 1 -- --nocapture`, and `cargo check -p roko-cli --jobs 1`.

**Remaining S5 work**: separate or supervise the Railway sidecars, replace the runtime image/toolchain layout with a proper multi-stage runtime image, and finish production CORS/deployment defaults. The serve process and `roko up` lifecycle are no longer the blocker for health/readiness.

---

## 6. Rust Code Anti-Patterns

### 6.1 Hardcoded Magic Numbers

**Locations**: 30+ sites with `Some(15_000)` for TTFT timeout, `DEFAULT_MAX_TOKENS=16_384` used as fallback everywhere, retry counts (3, 5, 10) scattered across crates.

**Fix** (B1): Central constants in `roko-core`:
```rust
// roko-core/src/config/defaults.rs
pub const DEFAULT_TTFT_TIMEOUT_MS: u64 = 15_000;
pub const DEFAULT_MAX_TOKENS: u32 = 16_384;
pub const DEFAULT_MAX_TOOL_ITERATIONS: usize = 25;
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_BACKOFF_MS: u64 = 1_000;
```

All struct constructors and builder patterns reference these constants. Per-model or per-provider overrides come from config.

**Batch 39 update (2026-05-04)**: `roko-agent` runtime request-timeout defaults now use `roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS` instead of raw `120_000` literals. This covered the concrete agents, provider adapters, shared tool-loop backends, and dispatch resolver test fixtures under `crates/roko-agent/src`. Verified that `rg 'unwrap_or\(120_000\)|timeout_ms: 120_000|const DEFAULT_TIMEOUT_MS: u64 = 120_000|Some\(120_000\)' crates/roko-agent/src` returns no matches, `cargo check -p roko-agent --jobs 1` passes, and the existing `timeout_ms_is_forwarded_to_poster` tests pass.

**Batch 40 update (2026-05-04)**: `roko-cli` runtime request-timeout defaults now use `roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS` instead of raw `120_000` fallback literals. This covered CLI config defaults/resolution, chat direct dispatch, chat-session API timeout fallback, config display defaults, orchestrator helper paths, runner event-loop dream config, and vision-loop evaluator dispatch. `roko-cli` config provider resolution also now uses `DEFAULT_CONNECT_TIMEOUT_MS` for the connect-timeout fallback. Verified that `rg 'unwrap_or\(120_000\)|timeout_ms: 120_000|Some\(120_000\)|or\(Some\(120_000\)\)' crates/roko-cli/src` returns no matches, `cargo check -p roko-cli --jobs 1` passes, and `cargo test -p roko-cli parses_minimal_config --jobs 1 -- --nocapture` passes.

**Batch 41 update (2026-05-04)**: `roko-serve` request-timeout defaults now use `roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS`. The runtime dream endpoint and provider route fixtures no longer carry raw `120_000` timeout literals; provider fixtures also use `DEFAULT_CONNECT_TIMEOUT_MS` for connect-timeout defaults. Verified that `rg 'unwrap_or\(120_000\)|timeout_ms: 120_000|Some\(120_000\)|or\(Some\(120_000\)\)|120_000' crates/roko-serve/src` returns no matches, `cargo check -p roko-serve --jobs 1` passes, and `cargo test -p roko-serve list_providers_returns_configured_providers_with_health --jobs 1 -- --nocapture` passes.

**Batch 42 update (2026-05-04)**: `roko-acp` no longer has request-timeout `120_000` literals under `crates/roko-acp/src`; the remaining bridge-events provider fixture now uses `DEFAULT_REQUEST_TIMEOUT_MS` and `DEFAULT_CONNECT_TIMEOUT_MS`. Verified with `cargo check -p roko-acp --jobs 1`, `cargo test -p roko-acp anthropic_model_call_config_routes_legacy_claude_to_anthropic_provider --jobs 1 -- --nocapture`, and the ACP source `rg` check.

**Batch 43 update (2026-05-04)**: `roko-core::ErrorKind::retry_policy()` no longer owns raw retry tuples. Added named defaults for rate-limit, timeout, and generic transient retry classes in `crates/roko-core/src/defaults.rs`, and added an exact policy-to-defaults regression in `crates/roko-core/src/error/mod.rs`. Verified with `cargo test -p roko-core retry_policy --jobs 1 -- --nocapture` and `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`.

**Batch 44 update (2026-05-04)**: Relay stale-data and circuit-breaker defaults moved from `crates/roko-serve/src/relay.rs` into `crates/roko-core/src/defaults.rs`: stale threshold, failure threshold, base backoff, and max backoff. Relay behavior and tests now consume those defaults. Verified with `cargo test -p roko-serve circuit_breaker --jobs 1 -- --nocapture`, `cargo test -p roko-serve relay_health --jobs 1 -- --nocapture`, and `cargo test -p roko-core relay_backoff_defaults_are_ordered --jobs 1 -- --nocapture`.

**Batch 45 update (2026-05-04)**: Runner plan timeout and task-DAG retry backoff defaults moved into `crates/roko-core/src/defaults.rs`. `CoreRunnerConfig`, the CLI `RunnerConfig`, `RunConfig::default()`, and `TaskDag::default()` now consume the shared plan timeout/backoff defaults. Verified with `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`, `cargo test -p roko-cli runner::task_dag::tests --jobs 1 -- --nocapture`, and `cargo test -p roko-cli parses_minimal_config --jobs 1 -- --nocapture`.

**Course correction (2026-05-04)**: `crates/roko-cli/src/orchestrate.rs` is gated behind the `legacy-orchestrate` Cargo feature and is not on the default runner path. Do not continue central-constants work there as part of the active redesign unless that feature is deliberately brought back into production use. The active target is the runner stack (`crates/roko-cli/src/runner/*`), provider dispatch, serve routes, and shared model/tool-loop code.

**Batch 46 update (2026-05-04)**: Active provider tool-loop iteration defaults are now centralized in `crates/roko-agent/src/provider/mod.rs` using `roko_core::defaults::DEFAULT_MAX_TOOL_ITERATIONS`. Anthropic, Gemini, Cerebras, Perplexity, and OpenAI-compatible adapters call `tool_loop_max_iterations()` without provider-local numeric defaults. The OpenAI-compatible path no longer has a separate `25` iteration default. Verified with `cargo check -p roko-agent --jobs 1`, `cargo test -p roko-agent tool_loop_iterations_derive_from_workspace_default --jobs 1 -- --nocapture`, and an `rg` check for numeric `tool_loop_max_iterations(...)` calls.

**Batch 47 update (2026-05-04)**: Vision-loop defaults moved into `crates/roko-core/src/defaults.rs`: max iterations, target score, consecutive target count, regression threshold, viewport width/height, and post-write wait. Both CLI vision-loop config (`crates/roko-cli/src/vision_loop/mod.rs`) and serve API defaults (`crates/roko-serve/src/routes/vision_loop.rs`) consume the shared constants. Verified with `cargo test -p roko-core retry_backoff_ordering --jobs 1 -- --nocapture`, `cargo test -p roko-cli default_config_has_sensible_values --jobs 1 -- --nocapture`, `cargo check -p roko-serve --jobs 1`, and an `rg` check for the removed inline patterns.

**Remaining S6.1 work**: request-timeout defaults are now cleared in roko-agent, roko-cli, roko-serve, and roko-acp source. The core error retry policy, serve relay circuit-breaker defaults, runner plan timeout/DAG backoff defaults, active provider tool-loop defaults, and vision-loop defaults are centralized. Active workflow iteration literals outside these paths remain open.

### 6.2 Silent Error Swallowing

**Pattern**: `if let Ok(x) = fallible_op() { use(x) }` — failure silently ignored. Found in:
- `roko-serve/src/routes/` (multiple handlers)
- `roko-agent/src/provider/` (response parsing)
- `roko-cli/src/commands/` (file operations)

**Fix**: Every fallible operation must either:
1. Propagate with `?`
2. Log at `warn!` or `error!` level with context
3. Have an explicit `// intentionally ignoring: <reason>` comment

### 6.3 Race Conditions

**Pattern**: Check-then-act on filesystem without locks:
```rust
if path.exists() {
    fs::read_to_string(&path)?  // TOCTOU: path may be deleted between check and read
}
```

**Found in**: Workspace creation, config loading, draft scaffold writing, episode logging.

**Fix**: Use `fs::read_to_string` directly and handle `NotFound` in the error branch. For writes, use atomic write (write to `.tmp`, then rename).

### 6.4 Unwrap in Non-Test Code

**Pattern**: `.unwrap()`, `.expect()` in production code paths. Found in:
- `roko-serve/src/state.rs` (lock acquisition)
- `roko-agent/src/provider/openai_compat.rs` (JSON parsing)
- `roko-cli/src/config.rs` (path operations)

**Fix**: Replace with `?`, `.unwrap_or_default()`, or explicit error handling. `unwrap()` is only acceptable in tests and provably-infallible operations (e.g., regex compilation of a literal).

### 6.5 Retry Without Backoff

**Pattern**: Immediate retry loops:
```rust
for _ in 0..3 {
    if let Ok(r) = try_request().await { return Ok(r); }
}
```

**Found in**: HTTP client code, provider dispatch, MCP connection.

**Fix**: Use a `RetryPolicy` struct:
```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub jitter: bool,
}

impl RetryPolicy {
    pub async fn execute<F, T, E>(&self, f: F) -> Result<T, E> { ... }
}
```

### 6.6 Inconsistent Error Types

**Pattern**: Some functions return `anyhow::Result`, others return custom error enums, others return `Box<dyn Error>`. No consistent error hierarchy.

**Fix**: Establish a crate-level error enum per crate using `thiserror`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("tool loop error: {0}")]
    ToolLoop(#[from] ToolLoopError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}
```

`anyhow::Result` is fine at the CLI boundary but not within library crates.

---

## 7. Demo App Anti-Patterns

### 7.1 Polling Loops

**Pattern**: `setInterval` polling for state that should be pushed:
```typescript
// Polls /api/workspaces/{id}/status every 2 seconds
const interval = setInterval(async () => {
    const status = await fetch(`/api/workspaces/${id}/status`);
    // ...
}, 2000);
```

**Found in**: Workspace status, agent status, pipeline progress, terminal ready check.

**Fix**: Use SSE (Server-Sent Events) for status updates. The server already has SSE infrastructure (`/api/events`). Each resource should push status changes:
```typescript
const events = new EventSource(`/api/workspaces/${id}/events`);
events.addEventListener('status', (e) => {
    setState(JSON.parse(e.data));
});
```

### 7.2 Stale Closures

**Pattern**: `useEffect` callbacks capturing stale state:
```typescript
useEffect(() => {
    const handler = () => {
        // Uses `count` from render when effect was created, not current value
        setCount(count + 1);
    };
    ws.on('message', handler);
}, []); // empty deps = stale closure
```

**Found in**: `useTerminal.ts`, `useWorkspace.ts`, scenario runners.

**Fix**: Use `useRef` for mutable state accessed in callbacks, or use functional state updates (`setCount(c => c + 1)`).

### 7.3 Missing Cleanup

**Pattern**: `useEffect` without cleanup function:
```typescript
useEffect(() => {
    const ws = new WebSocket(url);
    ws.onmessage = handler;
    // No return () => ws.close();
}, [url]);
```

**Found in**: Terminal connections, SSE subscriptions, interval timers.

**Fix**: Every `useEffect` that creates a resource must return a cleanup function.

### 7.4 Magic Numbers / Hardcoded URLs

**Pattern**: `http://localhost:6677`, `ws://localhost:6677`, timeouts like `5000`, `2000`, `500` scattered across components.

**Fix**: Central config:
```typescript
// lib/config.ts
export const config = {
    apiBase: import.meta.env.VITE_API_URL || 'http://localhost:6677',
    wsBase: import.meta.env.VITE_WS_URL || 'ws://localhost:6677',
    pollInterval: 2000,
    reconnectBackoff: { initial: 500, max: 30000, factor: 2 },
} as const;
```

### 7.5 No Error Boundaries

**Pattern**: Unhandled promise rejections in component lifecycle. A single API error crashes the entire page.

**Fix**: React Error Boundaries around each major section. API errors caught and displayed inline with retry buttons.

### 7.6 Scenario Runner Brittleness

**Pattern**: `prd-pipeline.ts` orchestrates a multi-step pipeline with sequential `await` calls, no timeout on individual steps, no rollback on failure:
```typescript
await createWorkspace();
await runCommand('roko prd idea ...');
await runCommand('roko prd draft new ...');  // If this hangs, everything hangs
await runCommand('roko prd plan ...');
```

**Fix**: Each step gets a timeout. Pipeline state machine with explicit states:
```typescript
type PipelineState =
    | { phase: 'workspace', status: 'pending' | 'running' | 'done' | 'failed' }
    | { phase: 'idea', ... }
    | { phase: 'draft', ... }
    | { phase: 'plan', ... };
```

Failed steps show error + retry button. Pipeline can be resumed from the last successful step.

---

## 8. Testing Gaps

### Current Coverage (Estimated)

| Crate/Area | Coverage | Notes |
|---|---|---|
| roko-core | ~60% | Config, types well-tested. Signal pipeline less so. |
| roko-agent | ~40% | Unit tests for translation, some provider tests. No integration tests for actual LLM calls. |
| roko-serve | ~15% | A few route tests. No tests for workspace lifecycle, terminal, SSE, CORS, SPA fallback. |
| roko-cli | ~50% | Smoke tests exist. No tests for `prd` subcommands, `plan run`, or `dashboard`. |
| roko-std | ~70% | Tool definitions tested via golden tests. Execution less so. |
| demo-app | ~20% | Some component tests. No Playwright E2E. |
| **Overall** | **~35%** | |

### What's Missing

**Integration tests** (most critical):
1. **Config roundtrip**: Load config → serialize → reload → assert equality
2. **Workspace lifecycle**: Create workspace → run command → verify output → GC → verify cleaned
3. **PRD pipeline E2E**: `prd idea` → `prd draft new` → `prd plan` → verify artifacts exist and have content
4. **Provider smoke**: For each configured provider, send a simple prompt and verify non-empty response (can be run with `--integration` flag)
5. **Serve startup/shutdown**: Start server → verify port bound → send request → shutdown → verify port released

**Property-based tests**:
- Config TOML: arbitrary valid TOML → parse → serialize → parse → assert equal
- Tool definitions: arbitrary tool params → validate → assert no panic

**Snapshot/golden tests**:
- System prompt builder output for each role
- Tool JSON schema for each builtin tool (partially done in `roko-std`)
- API response shapes for key endpoints

**E2E (Playwright)**:
- Demo app loads without errors
- PRD pipeline scenario completes
- Terminal connects and accepts input
- Dashboard renders with data

### Testing Infrastructure Needed

1. **Test fixtures**: Shared test config, temp workspace creation, mock provider that returns canned responses
2. **`cargo test --workspace` must pass in CI** — currently flaky due to port conflicts and filesystem races
3. **Parallel test isolation**: Each test gets its own temp dir, unique port (or use `port 0` for OS-assigned)
4. **CI pipeline**: `cargo fmt --check` → `cargo clippy` → `cargo test` → `cargo test --features integration` → Playwright E2E

---

## 9. Design Pattern Recommendations

### 9.1 Builder Pattern for Config

Currently, config structs are constructed with struct literals that repeat defaults:
```rust
SomeConfig {
    ttft_timeout_ms: Some(15_000),
    max_tokens: Some(4096),
    // ... 12 more fields with defaults
}
```

Replace with builders:
```rust
SomeConfig::builder()
    .ttft_timeout_ms(15_000)
    .max_tokens(4096)
    .build()
```

Defaults live in one place (the builder). Missing fields get defaults. Compile-time enforcement of required fields.

### 9.2 State Machine for Pipeline Steps

PRD pipeline, plan execution, workspace lifecycle — all are multi-step processes currently implemented as sequential imperative code. Replace with explicit state machines:

```rust
enum PipelineState {
    Init,
    WorkspaceCreated { path: PathBuf },
    IdeaCaptured { id: String },
    DraftCreated { path: PathBuf },
    PlanGenerated { tasks: Vec<Task> },
    Failed { step: String, error: String },
}
```

Benefits: resumability, clear error states, logging of transitions, testable transitions.

### 9.3 Repository Pattern for Persistence

Currently, persistence is scattered: JSONL files, JSON state files, TOML configs, in-memory hashmaps. Unify behind a repository trait:

```rust
#[async_trait]
trait WorkspaceRepository {
    async fn create(&self, config: WorkspaceConfig) -> Result<Workspace>;
    async fn get(&self, id: &str) -> Result<Option<Workspace>>;
    async fn list(&self) -> Result<Vec<Workspace>>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

Implementations: `FileWorkspaceRepository` (persists to `.roko/workspaces/`), `InMemoryWorkspaceRepository` (tests).

### 9.4 Circuit Breaker for External Calls

LLM provider calls can hang or fail repeatedly. Wrap them in a circuit breaker:
- **Closed**: Normal operation, requests pass through
- **Open**: After N failures in M seconds, immediately fail all requests (don't waste tokens/time)
- **Half-Open**: After cooldown, allow one test request; if it succeeds, close the circuit

The `roko-conductor` crate already has circuit breaker primitives. Wire them into the provider dispatch path.

### 9.5 Atomic File Writes

Replace `fs::write(path, data)` with:
```rust
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?; // atomic on same filesystem
    Ok(())
}
```

Prevents partial writes on crash. Especially important for config files, state snapshots, and episode logs.

---

## 10. Reproducibility

### Dev Environment

**Problem**: Setting up roko requires: Rust 1.91+, Node 22+, specific env vars, `roko-dev-full` alias in `.zshrc`. New contributors have no guide.

**Fix**: `roko doctor` already exists — extend it to check all prerequisites and print actionable fixes:
```
$ roko doctor
[OK] Rust 1.91.0
[OK] Node 22.14.0
[WARN] Missing roko-dev-full alias — run: roko dev setup
[ERR] Port 6677 in use by PID 12345 — run: kill 12345
[OK] .roko/ directory exists
[OK] roko.toml valid
```

Add `roko dev setup` to install the dev alias, create `.roko/`, and generate a default `roko.toml` if missing.

### CI Reproducibility

**Problem**: CI uses latest stable rustc which may have different lints than local. Tests use real filesystem and ports, causing flaky failures.

**Fix**:
1. Pin rustc version in `rust-toolchain.toml`
2. All tests use temp dirs (not `.roko/` in the repo)
3. All server tests use port 0 (OS-assigned)
4. Cargo test with `--test-threads=1` for integration tests that touch shared state

### Config Reproducibility

**Problem**: Same `roko.toml` + different env vars = different behavior. No way to dump the effective config.

**Fix**: `roko config show --effective` dumps the fully-resolved config (after env var overrides, ancestor search, defaults). Useful for debugging "why is this model using the wrong max_tokens?"

---

## 11. Priority Order

| Priority | Area | Effort | Impact |
|---|---|---|---|
| P0 | Unify config loading (B4) | Medium | Eliminates entire class of "works in CLI, broken in serve" bugs |
| P0 | Central constants (B1) | Low | Request-timeout, core retry-policy, serve relay, runner DAG, provider tool-loop, and vision-loop defaults done batches 39-47; active workflow iteration literals remain |
| P0 | Health endpoint + graceful shutdown | Low | Done batch 37 for serve probes and `roko up`; remaining S5 work is Docker/Railway process layout |
| P1 | Workspace persistence | Medium | Fixes page-refresh data loss |
| P1 | Draft validation (B2, B6) | Low | Prevents empty PRDs from propagating |
| P1 | Strategist permissions (B3) | Low | Unblocks `prd plan` tool calling |
| P1 | Proper Dockerfile (no toolchain in runtime) | Medium | Cuts image size by ~1.5GB |
| P2 | Dev orchestrator (`roko dev`) | Medium | Eliminates triple-spawn, port conflicts |
| P2 | Terminal session reattach | High | Fixes page-refresh terminal loss |
| P2 | Integration test suite | High | Prevents regressions, enables CI |
| P3 | Error type hierarchy | High | Consistency, better error messages |
| P3 | Demo app SSE migration | Medium | Eliminates polling, improves responsiveness |
| P3 | Retry with backoff | Medium | Reliability for flaky providers |
| P3 | Atomic file writes | Low | Prevents corruption on crash |

---

## 12. Tool System Anti-Patterns

### 12.1 No Parameter Schemas Sent to LLMs (**FIXED 2026-05-03**)

~~Every builtin tool uses `ToolSchema::any_object()`, sending `{"type": "object"}` with no `properties`, `required`, or per-parameter descriptions.~~

**FIXED**: All 16 std tools now have full JSON Schema with `properties`, `required`, per-parameter `description`, and `additionalProperties: false`. Golden tests updated and passing. TOOL_COUNT updated from 30→33 (reflects 3 added chain.insight tools).

### 12.2 Bash Tool Has No Process Confinement

`crates/roko-std/src/tool/builtin/bash.rs:86-100` — `cmd.current_dir(ctx.worktree())` sets CWD but does not confine the process. Spawned shell has full filesystem access. The denylist (`DEFAULT_DENY_SUBSTRINGS`) is trivially bypassed and incomplete:
- `rm -rf /` blocked but `rm -rf /home/user/` is not
- `sudo ` blocked but aliases/obfuscation bypasses it
- No protection against `cat ~/.ssh/id_rsa`, `dd`, env var exfiltration

Two separate denylists exist (handler-level and safety-layer-level) with different entries. If no `SafetyLayer` is attached, only the weaker in-handler list applies.

### 12.3 Safety Layer Is Optional

`crates/roko-agent/src/dispatcher/mod.rs:108-118` — `safety` defaults to `None`. Every dispatcher constructed without `.with_safety()` has zero safety checks. No compile-time or runtime warning.

### 12.4 Safety Contracts Broken in Deployed Binaries — **FIXED (batch 38, 2026-05-04)**

`crates/roko-agent/src/safety/contract.rs:540` — Contracts loaded via `env!("CARGO_MANIFEST_DIR")` which resolves to the build machine's source tree. In deployed binaries, this path doesn't exist. `RestrictedFallback` (default) then denies all tools silently.

**FIXED (2026-05-03)**: `contract_for_role()` in `safety/mod.rs` now checks if the role has TOML-configured tools or overrides. When it does, the restricted fallback clears `allowed_tools` so the TOML role-tools whitelist is the binding constraint. Unknown roles with no config still get deny-all (fail-closed preserved).

**FIXED (batch 38, 2026-05-04)**: `AgentContract::load_for_role()` no longer reads from `env!("CARGO_MANIFEST_DIR")` or the build machine source tree. `crates/roko-agent/src/safety/contract.rs` now embeds all bundled contract JSON/YAML assets with `include_str!` in a role registry, parses from that registry, and reports missing bundled roles with a relative asset label instead of an absolute source path. Added regressions for the bundled role registry and missing-role relative path, and re-ran the focused `safety::contract` suite.

### 12.5 Gate-Approval Check Trusts LLM-Supplied Arguments — **FIXED (batch 35, 2026-05-04)**

`contract.rs:628-656` — `RequireGateBeforeCommit` checks if `gate_passed: true` is in the tool call's arguments. An LLM can bypass this by including `{"gate_passed": true}`. Same for `MaxTokensPerTurn` (reads `estimated_tokens` from LLM args) and `MaxCostPerTurn` (reads `estimated_cost_usd`).

**FIXED (batch 35, 2026-05-04)**:
- `RequireGateBeforeCommit` now ignores tool-call arguments entirely. Gate approval is accepted only from orchestrator-recorded `ToolContext.external_actions` entries: `gate_passed` or `run_gate` with `metadata.passed = true`.
- `MaxTokensPerTurn` ignores LLM-supplied `estimated_tokens` and `max_tokens`; token checks derive from actual string payload fields (`content`, `prompt`, `input`, `source`) using the existing chars/4 estimate.
- `MaxCostPerTurn` no longer reads `estimated_cost_usd` from the pending call. It sums recorded external-action cost metadata (`actual_cost_usd`, `cost_usd`, `total_cost_usd`, `usage.cost_usd`, or `usage.total_cost_usd`) and rejects once recorded turn spend exceeds the contract limit.
- Added unit regressions for all bypass shapes: `gate_passed: true` in call args is rejected, recorded gate actions pass, bogus low/high token and cost claims are ignored, and recorded provider cost cannot be bypassed by `estimated_cost_usd: 0`.

**Remaining integration note**: the contract now enforces recorded spend correctly, but normal provider/tool-loop paths still need to consistently record provider cost into `ToolContext.external_actions` if `MaxCostPerTurn` is expected to guard live model spend in all dispatch modes.

### 12.6 Implementer Contract Forbids Wrong Tool Names — **FIXED (pre-existing)**

Verified 2026-05-03: `contracts/implementer.yaml` uses `ForbiddenTools: ["web_fetch", "web_search"]` (correct tool names). The audit item was based on stale information.

### 12.7 No File Size Limits on Read/Write — **FIXED 2026-05-03 (read + write)**

- `read_file.rs` — Now checks `metadata().len()` against `DEFAULT_MAX_FILE_READ_BYTES` (10 MB) before reading. Returns clear error on oversized files.
- `write_file.rs` — Now checks `content.len()` against `DEFAULT_MAX_FILE_WRITE_BYTES` (5 MB) before writing. Returns clear error on oversized content.
- `glob.rs` — results bounded by `DEFAULT_MAX_GLOB_RESULTS` (1000) (pre-existing).

### 12.8 Non-Atomic File Writes in Tools — **FIXED 2026-05-03 (write_file)**

- `write_file.rs` — Now uses write-to-tmp-then-rename pattern (`.roko-tmp` extension). On rename failure, cleans up tmp file.
- `apply_patch.rs`, `edit_file.rs`, `multi_edit.rs` — still use direct write (read-modify-write cycle makes atomic more complex; tracked separately).

### 12.9 `run_tests` Only Parses Cargo Output Format — **FIXED 2026-05-03**

`test_gate.rs:parse_test_counts` now dispatches per-`BuildSystem`:
- **Cargo/Forge/Make**: existing `test result:` parser (unchanged)
- **Go**: existing `--- PASS:/FAIL:/SKIP:` marker parser (unchanged)
- **Npm**: new `parse_npm_test_counts` — handles Jest/Vitest (`Tests: N passed, ...`), Mocha (`N passing (12ms)`), and TAP (`# pass N`)
- **Python**: new `parse_pytest_counts` — handles pytest decorated summary (`=== N passed, N failed in ...`)
- Each npm/python parser falls back to cargo format if no native summary is detected.
- 11 new unit tests cover all formats.

### 12.10 Contract Check Runs Twice Per Dispatch — **FIXED (pre-existing)**

`dispatcher/mod.rs` — Comment at line 333-336 confirms the double-invocation was already removed. `check_pre_execution` includes the contract check internally; `check_contract` is no longer called separately from the dispatch path.

### 12.11 No Concurrency Cap on Parallel Tool Dispatch — **FIXED 2026-05-03**

`dispatcher/mod.rs` — Replaced `join_all` with `futures::stream::buffer_unordered(DEFAULT_MAX_CONCURRENT_TOOLS)` (8). At most 8 parallel tools execute simultaneously; excess is backpressured via the stream.

### 12.12 TOOL_COUNT Constant Stale — **FIXED 2026-05-03**

`TOOL_COUNT = 33` is correct (16 std + 17 chain = 33, matching `CHAIN_TOOL_COUNT`). The only issue was stale doc comments referencing "14 chain tools" — updated to "17". The `CHAIN_TOOL_COUNT` is enforced at compile time via array length checks.

### 12.13 `web_fetch` Creates New HTTP Client Per Call — **FIXED (2026-05-03)**

`web_fetch.rs:175-186` — New `reqwest::Client` (including TLS context, connection pool) on every invocation. `reqwest::Client` is designed to be shared. Wastes resources, prevents connection reuse.

**Fix**: Module-level `static HTTP_CLIENT: LazyLock<reqwest::Client>` shared across all invocations. Connection pooling and TLS sessions are now amortized.

---

## 13. Orchestration Pipeline Anti-Patterns

### 13.1 `max_iterations = 1000` Exits with `Ok` — **FIXED (2026-05-03)**

`orchestrate.rs:7512-7521` — When the iteration cap is hit, the loop breaks and returns `Ok(OrchestrationReport)`. Caller can't distinguish "completed normally" from "hit cap". Cap is arbitrary, not derived from plan size.

**Fix**: Added `hit_iteration_limit` flag; when set, function returns `anyhow::bail!()` with context (tasks completed/failed) after saving state for `--resume`.

### 13.2 Parallel Task JoinError Silently Drops Tasks — **FIXED (2026-05-03)**

`orchestrate.rs:10057-10064` — When a spawned parallel task panics, `JoinError` is logged and the result is dropped. No `record_task_failure` call. The plan may continue as if those tasks ran successfully.

**Fix**: Wrapped spawned task future in `futures::FutureExt::catch_unwind`. Panics are caught and produce a `ParallelTaskResult` with `AgentResult::fail(...)` — task is now recorded as failed with the panic message as context.

### 13.3 `SHUTDOWN_DRAIN_GRACE_SECS = 3` Is Too Short — **FIXED (2026-05-03, prev batch)**

`orchestrate.rs:227,8087` — 3-second grace period before `force_shutdown()` sends SIGTERM to the entire process group. In-flight agent work is always force-killed. Makes Ctrl-C always destructive.

**Fix**: Now uses `roko_core::defaults::DEFAULT_SHUTDOWN_DRAIN_SECS` (15 seconds).

### 13.4 SIGTERM to Entire Process Group — **FIXED (2026-05-03)**

`orchestrate.rs:5756-5767` — `libc::kill(0, libc::SIGTERM)` kills the entire process group, including parent processes (CI runner, daemon supervisor, shell scripts).

**Fix**: Now checks `getpid() == getpgrp()` before sending group SIGTERM. If roko isn't the process group leader, skips the group signal to avoid killing parent processes.

### 13.5 Non-Atomic Group Write of 3 State Files — **ACCEPTABLE (verified 2026-05-03)**

`orchestrate.rs:6974-7014` — Each file uses write-tmp-rename (individually atomic). The group isn't atomic, but `executor.json` is the authoritative resume source. Events and task-trackers are supplementary (rebuilt from the executor snapshot on resume if missing). A WAL-based approach would be ideal but adds substantial complexity for a low-probability failure mode.

### 13.6 `load_roko_config` Called In Hot Loop Without Caching — **FIXED 2026-05-03**

`orchestrate.rs` — Added `OnceLock`-based single-entry cache (workdir + config). First call loads and caches; subsequent calls for the same workdir return the cached clone. Reduces O(tasks × rungs) disk reads to O(1) per workdir per process run.

### 13.7 Config Loaded from Worktree `exec_dir`, Not Project Root

`orchestrate.rs:1568` — `load_roko_config(&cfg.exec_dir)` where `exec_dir` is a worktree that has no `roko.toml`. Falls through to `unwrap_or_default()`, silently using empty config. All user-configured provider routing ignored for parallel tasks.

### 13.8 Hardcoded Model Strings with Version Mismatches — **FIXED (2026-05-03)**

Multiple locations hardcode model IDs:
- Line 9950: `"claude-opus-4-6"`
- Line 10354: `"claude-sonnet-4-6"`
- Line 17648: `"claude-sonnet-4-20250514"` (different vintage)
- Line 14232: escalation ladder hardcoded as `["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]`

These will silently use wrong model IDs when Anthropic releases new versions.

**Fix**: Added `MODEL_DEEP`, `MODEL_FOCUSED`, `MODEL_FAST`, `MODEL_ESCALATION_LADDER` to `roko_core::defaults`. Replaced all production-code hardcoded model strings in orchestrate.rs and model_routing.rs. Test code retains specific model names intentionally.

**Batch 5 update (2026-05-03)**: Root `roko.toml` Cerebras defaults were also corrected after checking the current provider model list: `cerebras-scout` and `cerebras-70b` now use `gpt-oss-120b`, and `cerebras-8b` uses `llama3.1-8b`.

### 13.9 Generic Agent Handler Ignores Config — **FIXED 2026-05-03**

`handle_generic_agent` now respects `self.max_retries_override` (CLI `--max-retries` flag), falling back to `DEFAULT_RETRY_ATTEMPTS` (3) from `roko_core::defaults`. The escalation ladder was already centralized in `defaults::MODEL_ESCALATION_LADDER`.

### 13.10 Three Gate Rungs Always Skipped

`orchestrate.rs:17423-17443` — Symbol, PropertyTest, and Integration rungs are permanently skipped with debug-level messages referencing "T1-11" (pending capability detection). Plans relying on these gates silently get no validation.

### 13.11 Gate Thresholds Not Saved in Autosave (**NOT AN ISSUE — see note**)

`orchestrate.rs:5702-5717` — `adaptive_thresholds` and `gate_ratchet` only saved during `shutdown()`, not in `save_state()`. Process crash loses all gate threshold adaptations from the run.

**RESOLVED (2026-05-03 audit)**: In the runner v2 event loop (`event_loop.rs`), gate thresholds are saved on the critical path via `update_gate_thresholds()` immediately after each gate completion. `thresholds.save(path)` is called synchronously per gate result — not deferred to autosave or shutdown. A crash only loses the current in-flight gate's threshold update.

### 13.12 Rung Config Uses Raw Integer Constants

`orchestrate.rs:17619-17678` — Same rung semantics represented as `Rung` enum AND raw `u32` comparisons (`if rung == 5`, `if rung > 6`). If enum indices shift, these hardcoded comparisons silently break.

---

## 14. PRD Lifecycle Anti-Patterns

### 14.1 Promote Is Write-Then-Delete, Not Atomic

`prd.rs:746-747` — Writes published file first, then removes draft. Crash between leaves both files. Next promote silently overwrites the existing published file with no conflict detection.

### 14.2 No Pre-existing Published File Check — **FIXED (2026-05-03)**

`prd.rs:720-747` — `cmd_promote` doesn't check if a published PRD already exists at the destination. Silent overwrite.

**Fix**: Added `dst.exists()` guard before writing — returns error with guidance to remove/rename first.

### 14.3 Frontmatter Parser Is a Line Scanner, Not YAML

`prd.rs:484-521` — Manual line-by-line parsing breaks on:
- Values with colons (`title: Wire: the thing` → truncated)
- Quoted values
- YAML list syntax for `depends_on`
- Indented keys

### 14.4 `status: draft` Replace Acts on Entire File — **FIXED (2026-05-03)**

`prd.rs:736` — `content.replace("status: draft", "status: published")` replaces ALL occurrences in the file, not just frontmatter. If "status: draft" appears in the PRD body, it gets replaced too.

**Fix**: New `replace_in_frontmatter()` helper only mutates text between the first `---` pair. Body content left untouched. Unit test added.

### 14.5 Mtime-Based File Modification Detection

`commands/prd.rs:442` — `prd draft new` detects agent file writes via mtime comparison. Fails on filesystems with 1-second mtime resolution (HFS+, NFS) where an agent writes within the same second.

**FIXED (batch 34, 2026-05-04)**: `prd draft new` now snapshots the draft file bytes before/after agent execution and compares content, so direct agent writes are detected even when mtimes are coarse or unchanged.

### 14.6 Auto-Plan Failure Swallowed as Warning — **FIXED (2026-05-03)**

`prd.rs:825-828` — `maybe_generate_plan_after_promote` returns `Ok(None)` on failure. PRD is promoted and marked published but no plan is created. No actionable feedback.

**Fix**: Changed `eprintln!("warning: ...")` to `eprintln!("error: ...")` with actionable recovery guidance (`Run roko prd plan <slug> manually`). Still returns `Ok(None)` since the PRD promotion itself succeeded.

---

## 15. Concurrency Anti-Patterns

### 15.1 `parking_lot::Mutex` (Sync) in Async Functions

`roko-serve/src/state.rs:361` — `affect_engine: Mutex<DaimonState>` is a sync parking_lot Mutex. Accessed from async handlers in `dispatch.rs:2322`, `dreams.rs:233`. Blocks the OS thread when contended, starving other tasks on the same Tokio worker.

Same pattern: `roko-learn/src/runtime_feedback.rs` for `affect_engine`, `pattern_miner`, `experiment_store`, `local_rewards`, `section_effectiveness`.

### 15.2 Two Separate Mutexes in `CacheCell` — Deadlock Risk — **FIXED 2026-05-03**

`roko-agent/src/model_call_service.rs` — Redesigned to single `parking_lot::RwLock<CacheCellInner>` wrapping both entries HashMap and LRU order Vec. Read-path uses `read()` for miss check, `write()` only on hit/store/evict. Eliminates dual-mutex deadlock risk entirely. Also converted `ConvergenceDetectionCell` to `parking_lot::Mutex` (no poison). `SpendingLimiter` budget field converted from `std::sync::Mutex` to `parking_lot::Mutex`. `ToolDispatcher.tool_cache` similarly migrated. `SharedBudgetTracker` type alias updated.

### 15.3 Nested AsyncMutex in `PlaybookStore`

`roko-learn/src/playbook.rs:727-741` — `save_or_merge` acquires `__playbook_merge__/global` lock, then `exact` lock, both held across `.await` points. No ordering guarantee prevents deadlock.

### 15.4 MCP Client Lock Held Across I/O

`roko-agent/src/mcp/client.rs:157-160,210-245` — `StdioTransport` locks `stdin` and `stdout` mutexes across child-process I/O awaits. If the child hangs, every concurrent MCP call stalls.

### 15.5 Unbounded Channels Throughout — **PARTIALLY FIXED 2026-05-03**

**Fixed (bounded now)**:
- `bus_backends.rs` — `BroadcastBus` and `MemoryBus` subscriber channels now use `mpsc::channel(DEFAULT_CHANNEL_BUFFER=256)` with `try_send` (drop on full)
- `routes/aggregator.rs:737` — MuxEnvelope multiplexer channel now `mpsc::channel(DEFAULT_MUX_CHANNEL_BUFFER=512)`
- `routes/aggregator.rs:872` — Per-agent input channels now `mpsc::channel(DEFAULT_CHANNEL_BUFFER=256)`
- `routes/aggregator.rs:AgentStreamHandle.sender` — `Sender<String>` (bounded)

**Not fixed (trait-constrained)**:
- `LlmBackend::send_turn_streaming` trait uses `UnboundedSender<StreamChunk>` — changing this requires modifying 10+ backend implementations and the `DispatchLike` trait in agent-server. These channels are bounded by LLM response rate (natural backpressure). Tracked as Phase 2 trait refactor.
- `roko-plugin/src/lib.rs` — test-only code, acceptable.

Constants added to `roko_core::defaults`: `DEFAULT_CHANNEL_BUFFER = 256`, `DEFAULT_MUX_CHANNEL_BUFFER = 512`.

### 15.6 `active_runs` / `operations` JoinHandles Never Removed — Memory Leak — **FIXED 2026-05-03**

`roko-serve/src/state.rs` — Added `gc_completed_handles()` method that retains only handles whose `JoinHandle::is_finished()` is false. Wired into a 60-second interval timer (`start_handle_gc`) alongside the workspace GC in `lib.rs`.

### 15.7 `std::thread::sleep` in Async Contexts — **ACCEPTABLE (verified 2026-05-03)**

| File | Line | Duration | Verdict |
|------|------|----------|---------|
| `process/registry.rs` | 121 | 200ms | **Sync context** — process kill sequence, not in async runtime |
| `agent_serve.rs` | 1072, 1083 | 100-200ms | **Sync context** — process shutdown path, not blocking tokio workers |
| `tui/verdicts.rs` | 137-143 | per-call thread spawn | Only flagged issue; TUI `tick()` should use `tokio::task::spawn_blocking` |

The `process/registry.rs` and `agent_serve.rs` cases are in synchronous code paths (process kill + drain). Only `verdicts.rs tick()` is a genuine concern (spawns OS thread from async context at TUI refresh rate).

### 15.8 Polling-Based Cancellation

`dispatcher/cancel.rs:26-35` — `wait_cancelled` polls at 50ms intervals instead of using `tokio::sync::Notify`. Creates consistent 50ms jitter. Same polling pattern at `dispatcher/mod.rs:993` (80ms) and `mod.rs:1203` (60ms).

---

## 16. Learning Subsystem Anti-Patterns

### 16.1 Episode Log Compaction Never Called at Runtime

`episode_logger.rs` — `compact()` exists with atomic rename. `RetentionPolicy` and `EpisodeStorageConfig` exist. Neither is called anywhere in `orchestrate.rs` or serve routes. Log grows unboundedly. `max_age_days = 90` and `max_episodes = 200` are dead code.

**FIXED (2026-05-03)**: `compact_episodes_if_needed()` wired into post-run cleanup in `event_loop.rs`. Runs after `shutdown_subsystems()` with default `RetentionPolicy` (200 episodes, 90 days). Errors logged but not propagated (best-effort).

### 16.2 Episode ID Uses Unstable `DefaultHasher` — **FIXED 2026-05-03**

`episode_logger.rs` — Replaced `DefaultHasher` with inline FNV-1a (64-bit) implementation. FNV-1a is algorithmically specified (offset=0xcbf29ce484222325, prime=0x100000001b3), producing identical hashes across all Rust/platform versions. No external crate needed.

### 16.3 Cascade Router Silently Resets on Parse Error

`cascade_router.rs:1682-1691` — `load_or_new` returns fresh empty router on any JSON parse error. All prior observations, stage transitions, and role table silently lost. No warning log.

**FIXED (2026-05-03)**: `load_or_new()` now backs up corrupted file to `<path>.corrupted`, logs `tracing::warn!` with error details, then creates fresh router. No more silent data loss.

### 16.4 Experiment Outcomes Never Persisted — **NOT AN ISSUE (verified 2026-05-03)**

`model_experiment.rs:296-319` — `record_outcome` updates in-memory stats but never calls `save()`. Caller must call save manually.

**Status**: Verified that `record_model_experiment_outcome()` in orchestrate.rs (line 10657) calls `experiment_store.save(&experiment_path)` immediately after `record_outcome()`. The save IS wired.

### 16.5 Multiple Concurrent Experiments Cause Starvation — **FIXED 2026-05-03**

`active_for_role` and `active_for_category` now use `min_by_key(|e| e.total_trials())` instead of `find()`. When multiple experiments target the same role/category, the one with fewest total observations is selected, balancing allocation deterministically.

### 16.6 Cascade Router Hardcoded Slug Lists — **ACCEPTABLE (verified 2026-05-03)**

`default_role_model_table` is parameterized by `model_slugs: &[String]` from config. `pick_static_slug` returns the first match from a priority-ordered candidate list that exists in the actual configured slugs. Preference lists are reasonable defaults (Perplexity for research, fast models for Fast tier, etc.). The fallback to `candidates[0].to_string()` only triggers when zero configured models match — which means the config is effectively empty. Current design is config-aware and intentional.

**Batch 5 update (2026-05-03)**: Router arm initialization now uses config-derived `RokoConfig::model_slugs_for_cascade()` instead of env-gated provider availability checks. `CascadeRouter` persistence therefore keeps the full configured non-embedding slug set across process restarts, while gateway/orchestrator dispatch narrows to `available_model_slugs_for_cascade()` only at selection time.

### 16.7 No WAL for Cascade Router Observations

Observations accumulated in memory, saved to disk only on task completion or run end. Crash mid-run loses all LinUCB weight updates and confidence stats.

**Batch 8 update (2026-05-03)**: Direct agent-exec captures now use the canonical `.roko/learn` runtime instead of `.roko/memory`, resolve model keys to API slugs before recording episodes, and persist the cascade router immediately through `LearningRuntime::record_completed_run()`. This does not add a WAL, but it removes the PRD/research/plan one-shot blind spot where observations were landing in a non-canonical tree or missing the configured slug arm.

**Batch 9 update (2026-05-03)**: The same direct learning helper logic is now shared with `dispatch_via_model_call_service()`. That path attaches a `.roko/learn/cascade-router.json` router to `FeedbackService`, saves it after the call, and records persisted provider health by configured provider ID.

**Batch 12 update (2026-05-03)**: `ChatAgentSession::send_turn_api()` now uses the same `.roko/learn` feedback surface: it attaches a persisted cascade router to `FeedbackService`, flushes feedback after each API-mode chat turn, saves the router, and records provider health under configured provider IDs. Inline chat session mode is covered because non-CLI inline turns delegate through the same `send_turn_api()` path.

**Batch 15 update (2026-05-03)**: The older direct provider chat REPL now also records per-turn `FeedbackEvent::ModelCall` events, persists provider health by configured provider ID, and saves direct chat cascade observations on exit.

**Batch 16 update (2026-05-03)**: `roko vision-loop` evaluator calls now record `.roko/learn` model-call feedback, persist provider health by configured provider ID, and save cascade-router observations. Invalid or unparseable evaluator JSON is treated as a learning failure, while provider-health still tracks the underlying provider call result.

**Batch 17 update (2026-05-03)**: Dispatch-v2 provider-factory bridge calls now record `.roko/learn` model-call feedback, persist provider health by resolved provider ID, and save cascade-router observations across non-streaming, streaming, and MCP-preloaded bridge execution.

**Batch 18 update (2026-05-03)**: Serve template dispatch now records `.roko/learn` model-call feedback, persists provider health by configured provider ID, updates the in-memory serve provider-health tracker, and saves cascade-router observations. Global template dispatch observes through the cached `AppState` router so shutdown persistence does not overwrite the new observation; repo-specific dispatch writes under the repo layout.

**Batch 21 update (2026-05-03)**: The direct model-call persistence sequence now lives in `roko-learn::model_call_feedback`. Direct provider chat, vision evaluator, dispatch-v2 bridge calls, and serve template dispatch share the same recorder for efficiency feedback, provider-health persistence, and cascade-router saves. Serve's global template path still observes through the cached `AppState` router before using the shared recorder for feedback/health.

**Still open**: Long-running in-memory observations still need WAL/append-only durability between task completion checkpoints.

---

## 17. Prompt Assembly Anti-Patterns

### 17.1 Synchronous Unbounded Directory Scan — **FIXED 2026-05-03**

`prompt_assembly_service.rs` — Added `SOURCE_SCAN_MAX_DEPTH = 5` and `SOURCE_SCAN_MAX_FILES = 500` bounds. The recursive scan now early-returns when either limit is hit, preventing unbounded traversal on large monorepos. Combined with existing `SOURCE_SAMPLE_LIMIT = 12` for content reads, the function is now O(bounded).

### 17.2 Heuristic Token Counter Wrong for Code — **FIXED (2026-05-03)**

`prompt_assembly_service.rs:473-476` — Hardcoded 4.0 chars/token ratio. Claude's BPE averages ~3.5 for code. Undercount by 15-25% for code-heavy prompts → token budget overruns at the API layer.

**Fix**: Changed heuristic fallback from 4.0 to 3.5 in token_counter.rs (Claude/GPT path), prompt_assembly_service.rs, and compaction.rs truncate_to_budget.

### 17.3 Duplicate `## Relevant Knowledge` Headings — **FIXED (2026-05-03)**

`prompt_assembly_service.rs:570-581` — `format_knowledge_section` and `format_techniques_section` both produce `## Relevant Knowledge` heading. Both can appear in the same prompt, creating duplicate sections.

**Fix**: `format_techniques_section` now uses `## Relevant Techniques` heading.

### 17.4 `build_with_counter` Skips Cache Normalization — **FIXED (2026-05-03)**

`system_prompt_builder.rs:352-426` — `build()` calls `normalize_for_caching` but `build_with_counter()` does not. Prompts via token-budget path have different whitespace → different cache keys → reduced cache hit rates.

**Fix**: Added `normalize_for_caching()` wrapper around `assemble_selected_sections()` return value in `build_with_counter()`.

### 17.5 Episode Context Reads Only Live Log File

`prompt_assembly_service.rs:343` — After episode log rotation (at 10 MiB), the most recent 5 episodes may be in a rotated file. The builder reads only the live `episodes.jsonl`, producing stale or empty episode context.

**NOTE (2026-05-03)**: This is a non-issue in practice — roko doesn't rotate episodes.jsonl (compaction writes back to the same file). If rotation is added later, the fix is to `read_all_lossy` from the rotated file first, then the live file.

### 17.6 Episode Parse Errors Silently Swallowed — **FIXED 2026-05-03**

`prompt_assembly_service.rs` — Changed from `EpisodeLogger::read_all` (strict, aborts on first bad line) to `EpisodeLogger::read_all_lossy` (skips corrupt/truncated lines, preserves valid episodes). Agent now gets episode context even if the log has crash artifacts.

---

## 18. Context Window Pressure Watcher — Dead Subsystem

### 18.1 Only Covers Claude Models

`context_window_pressure.rs:22-124` — `context_window_tokens` hardcodes only Opus (1M), Sonnet (200K), and Haiku (200K). Returns `None` for Gemini, Perplexity, GLM, and all other models. The watcher fires for no non-Claude model.

### 18.2 Intervention Signal Has No Consumer

The watcher correctly emits `conductor.intervention` signals. But nothing in `orchestrate.rs` or the conductor pipeline subscribes to this signal. The watcher detects pressure and emits a signal that is ignored. The entire subsystem has no runtime effect.

### 18.3 Checks Only Most Recent Signal

`context_window_pressure.rs:53-86` — `decide` reads only the last `TokenUsage` signal. Alternating 85%/30% utilization intermittently fires, creating noisy guidance.

---

## 19. Expanded Design Pattern Recommendations

### 19.1 Write-Ahead Log (WAL) for Critical State

Cascade router observations, experiment outcomes, and gate thresholds are accumulated in memory and only saved on clean shutdown. Any crash loses the entire session's learning. Add a WAL:

```rust
pub struct WalWriter {
    file: File,
}

impl WalWriter {
    pub fn append(&mut self, entry: &WalEntry) -> io::Result<()> {
        serde_json::to_writer(&mut self.file, entry)?;
        self.file.write_all(b"\n")?;
        self.file.sync_data()?;
        Ok(())
    }
}
```

On startup, replay WAL entries to reconstruct state. After successful snapshot, truncate WAL.

### 19.2 Proper JSON Schema for Tool Definitions

Replace `ToolSchema::any_object()` everywhere with typed schemas:

```rust
impl BashTool {
    fn tool_def() -> ToolDef {
        ToolDef::new("bash", "Execute a shell command")
            .with_parameters(ToolSchema::object()
                .property("command", ToolSchema::string()
                    .description("The shell command to run"))
                .property("timeout_ms", ToolSchema::integer()
                    .description("Timeout in milliseconds (default 120000)"))
                .required(["command"])
            )
    }
}
```

This gives LLMs structured guidance, reduces hallucinated arguments, and enables schema-level validation before dispatch.

### 19.3 Config Caching with File Watch Invalidation

Replace the repeated `load_roko_config` calls with a cached loader:

```rust
pub struct ConfigCache {
    config: ArcSwap<RokoConfig>,
    _watcher: notify::RecommendedWatcher,
}

impl ConfigCache {
    pub fn get(&self) -> Arc<RokoConfig> {
        self.config.load_full()
    }
}
```

Load once at startup, invalidate on file change via `notify`. All consumers call `cache.get()` instead of re-parsing.

### 19.4 Bounded Resource Limits

Add resource limits to tool execution:

```rust
pub struct ResourceLimits {
    pub max_file_read_bytes: usize,   // 10 MB
    pub max_file_write_bytes: usize,  // 10 MB
    pub max_process_memory_mb: u64,   // 512 MB
    pub max_glob_results: usize,      // 1000
    pub max_concurrent_tools: usize,  // 5
}
```

Applied at the dispatcher level, not per-tool. Central enforcement.

### 19.5 Explicit Lock Ordering Contract

For any subsystem with multiple locks, document and enforce the order:

```rust
/// Lock ordering (MUST acquire in this order to prevent deadlock):
/// 1. stage_tracking
/// 2. confidence_stats
/// 3. linucb
struct CascadeRouter {
    stage_tracking: Mutex<StageTracking>,      // acquired first
    confidence_stats: Mutex<ConfidenceStats>,  // acquired second
    linucb: Mutex<LinUCB>,                     // acquired third
}
```

Or better: merge into a single lock when the critical section is short.

### 19.6 Safety Layer as Required, Not Optional

Change `ToolDispatcher` to require `SafetyLayer` at construction:

```rust
impl ToolDispatcher {
    pub fn new(
        registry: Arc<dyn ToolRegistry>,
        resolver: Arc<dyn HandlerResolver>,
        safety: SafetyLayer,  // required, not Option
    ) -> Self { ... }

    // Explicit opt-out for tests only
    #[cfg(test)]
    pub fn new_unsafeguarded(...) -> Self { ... }
}
```

---

## 20. Expanded Testing Strategy

### Unit Test Priorities

| Area | Tests Needed |
|------|-------------|
| Tool schema validation | Each builtin tool: valid args pass, missing required fails, wrong type fails |
| Contract enforcement | `gate_passed` bypass attempt → rejected. LLM-supplied `estimated_tokens` → ignored |
| Config roundtrip | Parse → serialize → parse → assert equal for every config section |
| Episode ID stability | Same inputs → same ID across process restarts |
| Cascade router | `load_or_new` with corrupt JSON → warn + fresh state (not silent) |
| Frontmatter parsing | Colons in values, quoted strings, YAML lists |

### Integration Test Priorities

| Test | What It Validates |
|------|------------------|
| Server lifecycle | `roko serve` starts → port bound → `/health` returns 200 → SIGTERM → port released |
| Workspace roundtrip | Create → list → get → run command → GC → verify cleaned |
| Config unification | Set `ROKO__AGENT__MODEL=haiku` → both CLI and serve use it |
| PRD pipeline | `idea` → `draft new` → `draft promote` → `plan` → artifacts exist with content |
| Gate pipeline | Run compile gate on test project → pass/fail correctly |
| Resume after crash | Save state → simulate crash → resume → verify no re-dispatched tasks |

### Property-Based Tests (proptest/quickcheck)

```rust
proptest! {
    #[test]
    fn config_roundtrip(config in arb_roko_config()) {
        let toml = toml::to_string(&config).unwrap();
        let parsed: RokoConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn tool_schema_validates_own_examples(tool in arb_builtin_tool()) {
        let schema = tool.parameters();
        for example in tool.examples() {
            assert!(schema.validate(&example).is_ok());
        }
    }
}
```

### E2E Tests (Playwright)

```typescript
test('PRD pipeline completes', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="prd-pipeline-start"]');
    await expect(page.locator('[data-testid="pipeline-status"]'))
        .toHaveText('completed', { timeout: 120_000 });
    await expect(page.locator('[data-testid="tasks-generated"]'))
        .toBeVisible();
});
```

### CI Pipeline

```yaml
jobs:
  check:
    - cargo +nightly fmt --all -- --check
    - cargo clippy --workspace --no-deps -- -D warnings
  test:
    - cargo test --workspace
  integration:
    - cargo test --workspace --features integration
  e2e:
    - npm ci --prefix demo/demo-app
    - npm run build --prefix demo/demo-app
    - cargo build --release -p roko-cli
    - ./target/release/roko serve &
    - npx playwright test
```

---

## 21. Updated Priority Matrix

| Priority | Area | Effort | Impact | Section |
|---|---|---|---|---|
| **P0** | Unify config loading | Medium | Eliminates "works in CLI, broken in serve" class | S4 |
| **P0** | Fix safety contract loading (CARGO_MANIFEST_DIR) | Low | Done batch 38; contract assets are embedded and no longer depend on build-machine source paths | S12.4 |
| **P0** | Fix gate_passed bypass | Low | Done batch 35; contract no longer trusts LLM-supplied gate/token/cost claims | S12.5 |
| **P0** | Central constants (B1) | Low | Request-timeout, core retry-policy, serve relay, runner DAG, provider tool-loop, and vision-loop defaults done batches 39-47; active workflow iteration literals remain | S6.1 |
| **P0** | Health endpoint + graceful shutdown | Low | Done batch 37 for top-level `/health`/`/ready` and `roko up` graceful serve shutdown; Docker/Railway sidecar work remains | S5 |
| **P1** | Proper tool schemas | Medium | Reduces LLM hallucination, enables validation | S12.1, S19.2 |
| **P1** | Safety layer required, not optional | Low | Eliminates unguarded dispatchers | S12.3, S19.6 |
| **P1** | Fix parallel task JoinError handling | Low | Tasks no longer silently dropped | S13.2 |
| **P1** | Config caching | Medium | Eliminates 70+ disk reads per plan run | S13.6, S19.3 |
| **P1** | Workspace persistence | Medium | Fixes page-refresh data loss | S2 |
| **P1** | Draft validation (B2, B6) | Low | Prevents empty PRDs | S14.1 |
| **P1** | Fix experiment persistence | Low | Experiment outcomes survive restart | S16.4 |
| **P1** | Proper Dockerfile | Medium | Cuts image size by ~1.5GB | S5 |
| **P2** | Dev orchestrator (`roko dev`) | Medium | Eliminates triple-spawn, port conflicts | S1 |
| **P2** | Replace unbounded channels | Medium | Prevents OOM under load | S15.5 |
| **P2** | Episode log compaction wiring | Low | Prevents unbounded storage growth | S16.1 |
| **P2** | Cascade router corrupt-file resilience | Low | No silent learning regression on restart | S16.3 |
| **P2** | Fix sync Mutex in async contexts | Medium | Prevents Tokio worker starvation | S15.1 |
| **P2** | Integration test suite | High | Prevents regressions, enables CI | S20 |
| **P2** | Terminal session reattach | High | Fixes page-refresh terminal loss | S3 |
| **P3** | WAL for critical state | High | Crash recovery for learning data | S19.1 |
| **P3** | Error type hierarchy | High | Consistency, better error messages | S6.6 |
| **P3** | Demo app SSE migration | Medium | Eliminates polling | S7.1 |
| **P3** | Resource limits on tools | Medium | Prevents OOM/disk fill from tools | S19.4 |
| **P3** | Lock ordering contracts | Low | Prevents deadlocks | S19.5 |
| **P3** | Context window pressure wiring | Medium | Currently a dead subsystem | S18 |
| **P3** | Atomic file writes | Low | Prevents corruption on crash | S9.5 |
| **P3** | PRD frontmatter YAML parser | Low | Prevents silent data loss | S14.3 |

---

## 22. UX Gaps: Silent Operations and Missing Feedback

### 22.1 Terminal Hangs Silently During LLM Calls (OpenAI-Compat)

**What user sees:** `"creating agent via provider adapter"` then NOTHING for minutes.
**Root cause:** `OpenAiCompatLlmBackend::send_turn()` (`openai_compat_backend.rs:378-402`) blocks on HTTP with zero progress feedback. No heartbeat, no streaming, no spinner. Unlike `ClaudeCliAgent` which has a 15-second heartbeat task, the `ToolLoopAgent` path has none.

**Fix:** Spawn a heartbeat task alongside `send_turn`:
```rust
let heartbeat = tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    loop {
        interval.tick().await;
        eprintln!("[{model}] waiting for response... ({elapsed}s elapsed)");
    }
});
let result = backend.send_turn(...).await;
heartbeat.abort();
```

### 22.2 No Streaming Content During Claude CLI Response

**What user sees:** `"waiting for response... (30s elapsed)"` every 15 seconds, but no actual text.
**Root cause:** `emit_stream_summary()` in `claude_cli_agent.rs:495-547` handles `content_block_delta` by only accumulating `text_bytes` — it never prints the text delta. Content appears only in the `"result"` summary after the full response.

**Fix:** Print partial text as it arrives:
```rust
// In emit_stream_summary, when handling content_block_delta:
if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
    eprint!("{text}"); // stream to stderr
}
```

### 22.3 `prd draft new` Exits Non-Zero Even When Draft Is Written

**What user sees:** Demo pipeline shows "PRD draft command failed" even when the draft file exists on disk.
**Root cause:** `commands/prd.rs:557` returns the agent's exit code, not whether the artifact was created. Claude CLI can exit non-zero for benign reasons (token limit, warnings). Lines 447-472 check artifact state and print success, but the function still returns `Ok(exit_code)` where exit_code came from the subprocess.

**Fix:** Return 0 when the artifact exists with substantive content:
```rust
if file_was_modified && has_substantive_content {
    Ok(0)  // artifact created successfully
} else {
    Ok(exit_code)  // agent actually failed
}
```

**FIXED (batch 34, 2026-05-04)**:
- `prd draft new` now treats a substantive draft artifact as command success even when the agent returns non-zero.
- Non-empty text output is materialized into the draft regardless of agent exit status, but only if it passes `has_substantive_markdown_content()`.
- Empty/non-substantive output still removes the scaffold and returns failure.
- Learning episodes now record artifact success rather than raw subprocess success for this command.
- Added an E2E mock fixture where the agent writes `.roko/prd/drafts/failing-but-written.md` and exits non-zero; the CLI returns success and preserves the draft.

### 22.4 Truncated Tool-Call Arguments Silently Salvaged

**What user sees:** `WARN truncated tool-call arguments — salvaging as raw content tool=write_file len=15800`
**Root cause:** `translate/openai.rs:95-104` — when a model hits its output token limit mid-tool-call JSON, the truncated string is wrapped in `{"__truncated": true, "raw": "..."}`. The tool dispatcher receives garbage arguments and fails. The warning is only in tracing (not visible without `RUST_LOG`).

**Fix:** Detect `__truncated` at the dispatcher level and return a clear error to the model:
```
"ERROR: Your write_file tool call was truncated (15,800 chars) because you hit the output
token limit. Split the file content into smaller chunks or reduce the file size."
```

### 22.5 No Progress Feedback During Tool Loop Iterations

**What user sees:** Complete silence between turns for OpenAI-compat agents.
**Root cause:** `ToolLoop::run_inner()` has an `on_turn` callback slot, but it's never wired for `prd draft new` or `prd plan` invocations. Only `CodexAgent` (via `ToolLoopAgent`) uses it.

**Fix:** Wire a default `on_turn` that prints iteration progress:
```
[glm-5.1] Turn 1/25: called read_file (1 result)
[glm-5.1] Turn 2/25: called write_file (1 result)
[glm-5.1] Turn 3/25: no tool calls — generating final output
```

### 22.6 "Shell prompt not detected" Warning Only in DevTools

`useTerminal.ts:365` — 8-second timeout for shell prompt detection fires `console.warn` only. User sees nothing in the UI. Should surface as a toast notification or inline warning in the terminal panel.

---

## 23. Tool Calling Across Providers: Inconsistencies

### 23.1 Three Different DEFAULT_MAX_TOKENS

| Provider | Default max_tokens | Source |
|---|---|---|
| OpenAI-compat (all) / Codex | 16,384 | `codex_agent.rs:44` |
| Anthropic Messages API | 4,096 | `claude_agent.rs:35` |
| Claude CLI | N/A (CLI-controlled) | No roko control |

Switching a model from `claude_cli` to `anthropic_api` provider silently cuts output tokens by 4x.

### 23.2 Missing `max_output` in roko.toml — **FIXED (batch 36, 2026-05-04 coverage)**

These models lack `max_output`, falling back to DEFAULT_MAX_TOKENS:
- All kimi models (kimi-k2-5 supports 65,535 but gets capped at 16,384)
- sonar-pro, sonar (perplexity)
- gemma4, llama32, cerebras-8b, cerebras-scout
- o3, o4-mini

For kimi-k2.5, this is the **direct cause of the truncated tool-call bug**: the model tries to produce more than 16,384 tokens for a `write_file` call, the provider truncates mid-JSON.

**FIXED (batch 36, 2026-05-04)**:
- Added explicit `max_output` to every non-embedding model in the root `roko.toml`, including Kimi, Perplexity/Sonar, Gemini aliases, Cerebras aliases, Ollama aliases, `o4-mini`, and `glm-5v-turbo`.
- Added explicit `max_output` to every model in `docker/railway.roko.toml`, closing the Railway drift called out in the audit preamble.
- Added `project_model_profiles_have_explicit_max_output` in `roko-core` so both config files are parsed and future non-embedding models without `max_output` fail a focused regression test.

**Remaining from §23**: this covers configured ceilings only. Adaptive retry on `finish_reason == "length"`, per-model tool-iteration caps, and broader budget-strategy fields remain separate work under §23.3/§25.2.

### 23.3 Tool Loop Iteration Limits Differ by Provider

| Provider | Base limit | Effective (Balanced) |
|---|---|---|
| OpenAI-compat | 25 | 25 |
| Anthropic API | 50 | 50 |
| Gemini | 50 | 50 |
| Cerebras | 50 | 50 |
| Perplexity | 50 | 50 |

OpenAI-compat gets half the iterations of every other provider. Not configurable per model.

### 23.4 Gemini OpenAI-Compat URL Double-Prefix

`roko.toml` sets `base_url = "https://generativelanguage.googleapis.com/v1beta/openai"` for the gemini provider. The `GeminiAdapter` at `gemini/adapter.rs:30` appends `/v1beta/openai/v1` to this, producing a doubled path: `.../v1beta/openai/v1beta/openai/v1/chat/completions`. Since gemini models in roko.toml use `kind = "openai_compat"` (not `kind = "gemini_api"`), the `GeminiAdapter` is never invoked and this bug is latent.

### 23.5 `GeminiAdapter` Never Used

All gemini models in roko.toml point to `provider = "gemini"` which has `kind = "openai_compat"`. The elaborate routing logic in `gemini/adapter.rs` (native, compat, grounding, embedding) is never invoked. Grounding and code execution are bypassed for all gemini models.

### 23.6 Claude CLI Usage Always Reports Zero Tokens

`translate/mod.rs:312-316` — `extract_usage()` returns `Usage::default()` for `StreamJson` (Claude CLI's response type). All token accounting, cost estimation, budget tracking, and efficiency metrics for Claude CLI agents show zeros. The most expensive models in the cascade have no cost tracking.

### 23.7 `finish_reason` Extraction Broken for Claude CLI

`translate/mod.rs:222-232` — `extract_finish_reason_raw()` always returns `None` for `StreamJson`. The tool loop's output-limit detection (`tool_loop/mod.rs:545-547`) never fires for Claude CLI. Token-budget exhaustion is invisible.

### 23.8 Anthropic API Tool Loop Is Dead Code

All claude models in roko.toml use `kind = "claude_cli"`. The `AnthropicApiAdapter` and its tool-loop code in `anthropic_api/tool_loop.rs` have zero model entries pointing to them. The entire Anthropic Messages API tool-loop implementation is untested in production.

### 23.9 Claude Models Have Wrong `tool_format`

roko.toml lists `tool_format = "openai_json"` for all claude models. The actual Anthropic format is `"anthropic_blocks"`. Currently harmless because Claude CLI handles its own formatting, but would cause incorrect behavior if anyone switches to `anthropic_api` provider.

### 23.10 No Per-Model Iteration Cap

No `max_iterations` field exists in `ModelProfile` or roko.toml model config. The only way to change the cap is via the global temperament setting. A model known to need many tool calls gets the same cap as one that rarely calls tools.

### 23.11 Tool Name Sanitization Asymmetry

`translate/openai.rs:166-181` sanitizes dotted names (`chain.balance` → `chain__DOT__balance`). `translate/ollama.rs` does not. Dotted tool names break with Ollama backends.

### 23.12 `render_assistant_message` Missing for Anthropic/Ollama

Only OpenAI and Gemini translators implement `render_assistant_message`. Anthropic API and Ollama return `None`, meaning assistant messages are never injected into conversation history after tool-call turns. Models lose track of their own prior actions.

---

## 24. Observability and Metrics Gaps

### 24.1 No Real TTFT Measurement at HTTP Layer

`ResponseMetadata.provider_latency_ms` exists in `chat_types.rs:174` but is **never populated** by any provider. The `AgentEfficiencyEvent.time_to_first_token_ms` is derived from internal signal timestamps (includes queueing), not true provider TTFT. Cannot compare actual HTTP-level TTFT across providers.

### 24.2 No Prometheus Scrape Endpoint

`roko-serve` has ~85 routes but no `/metrics`. The `MetricRegistry` renders to `metrics/prometheus.txt` at session end. No live scraping for Grafana/Prometheus dashboards during a run.

### 24.3 No Distributed Request Tracing

No `trace_id`, `span_id`, W3C `traceparent`. No OTLP/Jaeger/Zipkin export. Cannot trace a request from CLI → orchestrate → agent → gate → response.

### 24.4 Bench Gate Verdicts and Retries Always Empty/Zero

`routes/bench.rs:294` — `gate_verdicts: Vec::new()` hardcoded. Line 296 — `retries_used: 0` hardcoded. Bench runs show no gate details or retry counts despite the full pipeline running.

### 24.5 Bench Cost Uses Hardcoded Rates

`bench.rs:700-713` — hardcoded per-1K-token rates by model name substring. Does not use the `CostTable` pricing tables from `roko-agent`. Models added to roko.toml with custom pricing use wrong rates in bench.

### 24.6 No Error Rate Tracking by Category

`GatewayEvent` has `success: bool` and `error: Option<String>` but no error category (rate_limit, timeout, model_error, network_error). No `roko_llm_errors_total{model,provider,error_type}` counter.

### 24.7 No Per-Plan/Per-Task Cost Aggregation in API

The HTTP API can query cost by model but not by plan or task. Gateway events have `caller` (role) but not plan/task hierarchy. Cannot answer "what did plan X cost?" via the API.

### 24.8 No Throughput Metrics

No fleet-level tasks/hour or aggregate tokens/second. `LatencyStats` tracks tokens/sec per model but no system-wide throughput for capacity planning.

### 24.9 Context Window Utilization Not Recorded Per Call

The pressure watcher detects high utilization but no per-call metric records the fraction used. Cannot analyze whether agents consistently hit context limits.

### 24.10 No Bench Regression Detection

Bench runs persist and can be compared, but no automated regression detection. `roko-learn/src/regression.rs` exists but isn't wired to bench. No alerting when pass_rate drops.

---

## 25. Performance and UX Redesign

### 25.1 Streaming-First Architecture

Currently: Non-streaming is the default. Streaming only when explicitly opted in.
**Redesign:** Streaming is the default. Every LLM call streams. Non-streaming is a special case for embedding/completion-only models.

```rust
trait LlmBackend {
    // Primary: streaming
    async fn stream_turn(&self, messages: &[Message]) -> Result<impl Stream<Item = Chunk>>;

    // Convenience: collect stream into response (used internally)
    async fn send_turn(&self, messages: &[Message]) -> Result<BackendResponse> {
        self.stream_turn(messages).await?.collect().await
    }
}
```

Benefits:
- User always sees progress (partial text, tool call names as they appear)
- TTFT is measurable as time-to-first-chunk
- Cancellation is immediate (drop the stream)
- Heartbeat is unnecessary (streaming IS the heartbeat)

### 25.2 Per-Model Token Budget Config

Add to `ModelProfile` / roko.toml:

```toml
[models.kimi-k25]
max_output = 65535
max_tool_iterations = 50  # NEW
token_budget_strategy = "adaptive"  # NEW: adaptive | fixed | unlimited
```

Adaptive strategy: start with `max_output`, if `finish_reason == "length"`, retry with doubled budget (up to model max). Eliminates the truncated-tool-call class of bugs.

### 25.3 Progress Event Bus

Every significant operation emits a `ProgressEvent`:

```rust
enum ProgressEvent {
    AgentCreated { model: String, provider: String },
    LlmRequestStarted { model: String, iteration: usize },
    LlmStreamingChunk { bytes: usize, tokens: usize },
    LlmResponseComplete { tokens: Usage, duration: Duration },
    ToolCallStarted { tool: String, iteration: usize },
    ToolCallComplete { tool: String, duration: Duration, result_size: usize },
    PipelinePhase { phase: String, status: String },
}
```

Consumers: CLI stderr output, demo app SSE, TUI dashboard, bench metrics.

### 25.4 Smart Exit Codes

```rust
enum PrdDraftExitCode {
    Success = 0,           // artifact exists with content
    AgentFailed = 1,       // agent crashed, no artifact
    EmptyScaffold = 2,     // artifact exists but empty/scaffold only
    ValidationFailed = 3,  // artifact exists but fails validation
    Timeout = 4,           // agent timed out
    TokenBudget = 5,       // hit token limit before completion
}
```

The demo app and CI can distinguish between "the model couldn't generate a PRD" and "the PRD was generated but the model process had a non-zero exit for a benign reason."

### 25.5 Observability Baseline

Minimum viable observability for performance optimization:

| Metric | Type | Labels | Where |
|---|---|---|---|
| `roko_llm_ttft_seconds` | Histogram | model, provider | HTTP client layer |
| `roko_llm_total_seconds` | Histogram | model, provider | Per-call |
| `roko_llm_tokens_total` | Counter | model, provider, direction | Per-call |
| `roko_llm_cost_usd` | Counter | model, provider | Per-call |
| `roko_llm_errors_total` | Counter | model, provider, error_type | Per-call |
| `roko_tool_duration_seconds` | Histogram | tool_name | Per-dispatch |
| `roko_gate_duration_seconds` | Histogram | rung | Per-gate |
| `roko_task_duration_seconds` | Histogram | role, outcome | Per-task |
| `roko_context_utilization` | Gauge | model | Per-call |

Exposed via `GET /metrics` in roko-serve. Scrapable by Prometheus. Dashboardable in Grafana.

### 25.6 Model Comparison Dashboard

The bench system has comparison but lacks live performance comparison. Add:

```
GET /api/models/performance
```

Returns for each configured model:
- Median TTFT, p95 TTFT
- Median total latency, p95 total latency
- Token throughput (tokens/sec)
- Cost per 1K input/output tokens
- Error rate (last 100 calls)
- Tool call success rate
- Average iterations per task

This enables informed model selection without running a full bench suite.

---

## 27. ACP / Zed Editor Integration

### 27.1 Global Config Not Loaded

**Severity**: P0 — ACP in non-roko projects shows only Anthropic/Sonnet.

The ACP config loader (`crates/roko-acp/src/config.rs:48`) walks parents looking for `roko.toml` but **never checks `~/.roko/config.toml`** (the global user config). The CLI calls `merge_global_providers()` at 8+ callsites (`run.rs:396`, `chat_session.rs:530`, `chat_inline.rs:1523`, `serve_runtime.rs:469`, `main.rs:2490`, `learning_helpers.rs:355`, `run.rs:1833,2721`), but this function lives in `roko-cli/src/config.rs:2753` and is never called from the ACP path.

**Result**: When Zed opens a project that isn't the roko repo, ACP finds no `roko.toml` in any ancestor, falls through to `roko_core::config::load_config` which returns defaults, and `build_config_options` sees empty providers → falls back to `build_config_options_static` → hardcodes only Anthropic/Sonnet.

**Root cause**: Global config is a CLI-only concept. `merge_global_providers` should live in `roko-core` so all consumers (CLI, ACP, serve, agent-server) get it.

**Batch 10 update (2026-05-03)**: ACP no longer papers over missing explicit config by synthesizing an Anthropic API provider from env/effective-provider compatibility in the cognitive dispatch path. This makes the missing global/shared config loader more visible instead of silently routing through env-only Anthropic fallback.

**Batch 11 update (2026-05-03)**: Core `RokoConfig::effective_providers()` also no longer creates an Anthropic API provider from process env or legacy `agent.env` values. Batch 20 removes the remaining empty-config `claude_cli` provider fallback.

**Batch 13 update (2026-05-03)**: `create_agent_for_model()` no longer synthesizes provider/model entries from known protocol commands. A known protocol command with an unknown model key now fails with a missing-config error, so remaining implicit config behavior is easier to locate in higher-level CLI compatibility helpers.

**Batch 14 update (2026-05-03)**: The CLI command-backed compatibility helpers now build explicit transient provider/model entries before calling the provider factory. This keeps legacy run/orchestrate command paths working without reintroducing hidden provider-factory inference.

**Batch 19 update (2026-05-03)**: Core `RokoConfig::effective_models()` now returns only explicit `[models.*]` profiles. `agent.default_model`, `agent.fallback_model`, and `agent.tier_models.*` are validated as references instead of silently creating runtime model profiles.

**Batch 20 update (2026-05-03)**: Core `RokoConfig::effective_providers()` now returns an empty registry for empty provider configs instead of synthesizing `claude_cli`. Explicit `[providers.claude_cli]` entries still get command-default compatibility, while command-backed CLI paths continue to materialize transient explicit providers at their boundary.

**Batch 25 update (2026-05-03)**: ACP `--config` now loads the exact file passed by the editor/CLI via `roko_core::config::loader::load_config_file()`. Before this batch, ACP took the parent directory of the explicit path and reran discovery, so a nonstandard editor config filename could be ignored in favor of a sibling or ancestor `roko.toml`. The explicit-file path still receives global merge, env overrides, interpolation, and secret resolution through `LoadOptions::acp()`.

### 27.2 Config Loader Proliferation (12 Separate Implementations)

The codebase has **12 distinct config loading functions**, each with different behavior:

| # | Function | Location | Global? | Env vars? | Ancestor walk? | Validation? |
|---|----------|----------|---------|-----------|----------------|-------------|
| 1 | `AcpConfig::load_roko_config` | `roko-acp/src/config.rs:48` | No | ROKO_CONFIG only | Yes | Lenient |
| 2 | `load_roko_config` | `roko-cli/src/config_helpers.rs:121` | No | No | No | No |
| 3 | `load_roko_config` | `roko-cli/src/orchestrate.rs:858` | No | No | No | No |
| 4 | `load_roko_config` | `roko-cli/src/agent_serve.rs:559` | No | No | No | No |
| 5 | `load_roko_config` | `roko-cli/src/main.rs:2480` | No | No | No | No |
| 6 | `load_roko_config` | `roko-cli/src/event_sources.rs:77` | No | No | No | No |
| 7 | `load_roko_config` | `roko-cli/src/subscriptions.rs:249` | No | No | No | No |
| 8 | `load_roko_config` | `roko-cli/src/vision_loop/orchestrator.rs:300` | No | No | No | No |
| 9 | `load_roko_config` | `roko-serve/src/lib.rs:435` | No | No | No | No |
| 10 | `AppState::load_roko_config` | `roko-serve/src/state.rs:626` | No | No | No | Cached |
| 11 | `load_layered` | `roko-cli/src/config.rs:2877` | Via merge | ROKO__* | Yes | Full |
| 12 | `load_config` / `load_config_strict` | `roko-core/src/config/mod.rs:115-125` | No | No | No | Full |

Only `load_layered` (#11) calls `merge_global_providers`. Only `load_layered` handles `ROKO__*` env var overrides. Most of the others are trivial `fs::read_to_string` + `toml::from_str` one-offs.

**Anti-pattern**: Each component reimplements config loading because `roko-core::config::load_config` doesn't do enough (no global merge, no env overrides), and `roko-cli::config::load_layered` does too much (adds `ResolvedConfig`/`ConfigSources`/`RepoRegistry` wrapping that serve/ACP don't need).

**Batch 25 update (2026-05-03)**: Added a second canonical core entry point for exact files: `load_config_file(path, opts)`. This closes the ACP explicit-path gap but does not finish the broader loader consolidation. CLI `load_layered()` still owns provenance and repo-registry assembly, and its duplicated `global_config_path()` / `discover_project_config()` helpers should be moved to wrappers around `roko-core::config::loader`.

**Batch 29 update (2026-05-03)**: `roko-cli::config::global_config_path()`, `discover_project_config()`, and `merge_global_providers()` now delegate to `roko_core::config::loader`. This removes duplicated path/global-merge logic from CLI while preserving `load_layered()` provenance and repo-registry behavior for now.

### 27.3 Static Fallback Hardcodes Anthropic/Sonnet

`build_config_options_static` (`session.rs:1050-1077`) is the fallback when `roko_config.providers.is_empty() || roko_config.models.is_empty()`. It hardcodes exactly one provider (Anthropic) and one model (Sonnet). This means:

- Projects with only a global config see only Anthropic/Sonnet
- Projects with partially-configured `roko.toml` (providers but no models, or vice versa) fall to static
- No way for users to discover what global providers are available

The static fallback should not exist at all if global config is properly loaded.

**Batch 26 update (2026-05-03)**: Removed `build_config_options_static()` and stopped default ACP session state from hardcoding `anthropic` / `sonnet`. Empty resolved configs now produce empty provider/model selections and empty provider/model option lists; configured projects still derive defaults from `[models.*]` and `[providers.*]`. This closes the hardcoded fallback itself. Follow-up: add session revalidation and provider-health/status display so empty or unavailable provider lists are actionable rather than silent.

### 27.4 Stale Persisted Sessions

`load_from_disk` (`session.rs:770-781`) deserializes a persisted session including its `provider` and `model` fields but **never validates them against the current config**. If:

- User changes providers in config → old session still references removed provider
- User switches workdir → session carries model keys from a different project
- Config file is deleted → session uses models that no longer resolve

The deserialized session should re-validate its provider/model against the current `RokoConfig` and fall back to defaults if invalid.

**Batch 27 update (2026-05-03)**: `SessionManager::load_from_disk()` now calls `AcpSession::revalidate_config_state()` before returning a resumed session. Missing providers reset provider/model to current config defaults; missing models under a still-valid provider reset to the first model for that provider or empty when none exists. Config options are rebuilt from the current config so resumed sessions no longer carry stale serialized provider/model option lists.

### 27.5 No Workdir Override from Zed

Zed's `settings.json` passes `roko acp` with no arguments. The `--workdir` flag defaults to `"."` (`main.rs:483-484`), which resolves to whatever directory Zed's project is in. There's no mechanism for:

- Passing a global workdir from Zed settings
- Detecting that the workdir has no `roko.toml` and escalating to global
- Informing the user that they're using default config

**Result**: Config behavior depends entirely on which folder is open in Zed, with no user visibility into why some projects have all providers and others have only Anthropic.

### 27.6 Provider Kind Mismatch

Global config (`~/.roko/config.toml`) may define a provider with `kind = "anthropic_api"`, while the project `roko.toml` defines `kind = "claude_cli"`. When configs are merged:

- `merge_global_providers` uses `entry.or_insert()` — project provider wins if key matches
- But if key differs (e.g., global has `anthropic` with `kind = "anthropic_api"`, project has `claude_cli` with `kind = "claude_cli"`), both survive and models may reference the wrong one
- No validation that model.provider matches an existing provider key after merge

### 27.7 Model Slug Duplicates and 404s

`roko.toml` contains duplicate model key mappings:
- `kimi-k25` and `kimi-k2-5` both map to slug `"kimi-k2.5"`
- `kimi-k26` and `kimi-k2-6` both map to slug `"kimi-k2.6"`
- `kimi-k2` maps to slug `"kimi-k2"` which Moonshot API doesn't recognize → 404

No config validation catches duplicate slugs or validates slugs against provider-known models.

**Batch 22 update (2026-05-03)**: CLI model selection no longer routes unknown slugs by inferred provider kind. A selected model must resolve to an explicit `[models.*]` profile; otherwise the CLI returns an `UnknownModel` error telling the user to add the profile. This does not yet validate slugs against provider-known lists, but it prevents unconfigured GPT-like slugs from silently using a configured `openai_compat` provider.

**Batch 23 update (2026-05-03)**: Serve dashboard/gateway provider-health surfaces now use provider IDs only when a model key or slug resolves to an explicit `[models.*]` profile. Unknown model strings no longer get provider health or health mutations through `AgentBackend::from_model()` inference.

**Batch 24 update (2026-05-03)**: Provider-factory creation now rejects configured model profiles whose `provider` field points at a missing provider. This prevents malformed config graphs from falling through to the raw `ExecAgent` subprocess fallback.

### Redesign

1. **Move global config into `roko-core`**: `global_config_path()` and `merge_global_providers()` become part of a unified `load_config()` that always checks `~/.roko/config.toml`
2. **Single canonical loader**: Replace all 12 implementations with one `roko_core::config::load_config(workdir, opts)` where opts controls strictness, global merge, env overrides
3. **Kill static fallback**: If unified loader always merges global, empty config is impossible when user has `~/.roko/config.toml`
4. **Session re-validation**: On load, validate provider/model exist in current config; reset to defaults if stale
5. **Config change detection**: Watch `roko.toml` and `~/.roko/config.toml`; invalidate cached sessions on change
6. **Slug validation**: At config load time, warn on duplicate slugs and validate slugs against known provider model lists where possible

**Current state after batches 25-26**:
- Core has a unified workdir loader and an exact-file loader used by ACP `--config`.
- ACP no longer hardcodes Anthropic/Sonnet when no providers/models are configured.
**Batch 27 update**: ACP persisted sessions now revalidate provider/model selections on resume.
**Batch 29 update**: CLI path/global helper bodies now delegate to the core loader helpers.
- Remaining work in this section is `load_layered()` effective-loader consolidation, config change invalidation, and provider/model status surfacing.

---

## 28. ACP Option Matrix: Provider × Model × Feature Compatibility

### 28.1 Current Option Surface

The ACP status bar presents 6 config options. From the screenshots: `[Provider] [Model] [Thinking] [Workflow] [Clippy] [Tests]`. The `SessionConfigState` struct also holds `temperament`, `routing_mode`, `review_strictness`, and `max_iterations` but these are hidden from the UI.

**Current options exposed to user**:

| Option | Values | Visible? |
|--------|--------|----------|
| Provider | 11 providers (anthropic, openai, zhipu, zai, gemini, moonshot, cerebras, perplexity, ollama, openrouter, claude_cli) | Yes |
| Model | 36 models (filtered by provider) | Yes |
| Thinking (effort) | Quick, Standard, Deep, Max | Yes |
| Workflow | None, Express, Standard, Full, Auto | Yes |
| Clippy | On, Off | Yes |
| Tests | On, Off | Yes |
| Temperament | cautious, balanced, aggressive | **Hidden** |
| Routing Mode | auto_override, manual, cascade | **Hidden** |
| Review Strictness | none, quick, standard, thorough | **Hidden** |
| Max Iterations | 1-3 | **Hidden** |

**Theoretical combinatorial space**: 11 × ~3.3 × 4 × 5 × 2 × 2 = ~2,904 combinations (using average 3.3 models per provider). The 4 hidden options would multiply this further.

### 28.2 What's Actually Wired vs. Theater

| Option | Stored | Passed to dispatch | Affects LLM call | Affects behavior | Verdict |
|--------|--------|-------------------|------------------|-----------------|---------|
| **Provider** | Yes | Yes (model resolution) | Yes (backend routing) | Yes | **Wired** |
| **Model** | Yes | Yes (resolve_model) | Yes (slug → API) | Yes | **Wired** |
| **Thinking (effort)** | Yes | **No** | **No** | **No** | **Theater** — UI toggle does nothing |
| **Workflow** | Yes | Yes | N/A | Yes (pipeline routing) | **Wired** |
| **Clippy** | Yes | Yes (PipelineConfig) | N/A | Yes (gate gating) | **Wired** (only in workflow≠none) |
| **Tests** | Yes | Yes (PipelineConfig) | N/A | Yes (gate gating) | **Wired** (only in workflow≠none) |
| **Temperament** | Yes | **No** | **No** | **No** | **Theater** — never read |
| **Routing Mode** | Yes | **No** | **No** | **No** | **Theater** — never read |
| **Review Strictness** | Yes | Yes (PipelineConfig) | N/A | **No** | **Theater** — stored but unused |
| **Max Iterations** | Yes | Yes (PipelineConfig) | N/A | Yes | **Wired** (only in workflow≠none) |

**3 of 6 visible options are theater** (Thinking does nothing). **All 4 hidden options are theater**.

### 28.3 Provider × Feature Capability Matrix

Not all features make sense for all providers. Current ACP presents identical options regardless of provider:

| Provider | Thinking support | Tool support | Workflow viable? | Notes |
|----------|-----------------|--------------|-----------------|-------|
| **Anthropic** (API) | Yes (thinking tokens) | Yes (native) | Yes | Thinking NOT wired in ACP dispatch |
| **Claude CLI** | Yes (--effort flag) | Yes (native) | N/A (not ACP) | ACP doesn't use Claude CLI path |
| **OpenAI** | Yes (o3/o4 reasoning) | Yes (native) | Yes | Thinking NOT wired |
| **ZhiPu** (GLM) | Binary (enabled/disabled) | Yes | Yes | GLM ignores effort *levels*, uses binary |
| **Zai** (GLM) | Binary (enabled/disabled) | Yes | Yes | Same engine as ZhiPu |
| **Moonshot** (Kimi) | Binary (enabled/disabled) | Yes | Yes | Kimi ignores effort *levels*, uses binary |
| **Gemini** (native) | Yes (thinking_level) | Yes | Yes | Only backend that maps effort levels |
| **Cerebras** | No | Yes | Yes | Thinking option is noise |
| **Perplexity** | No | **No** (supports_tools=false) | **No** | Workflow makes no sense |
| **Ollama** | Model-dependent | Model-dependent | Maybe | Depends on model behind Ollama |
| **OpenRouter** | Model-dependent | Model-dependent | Maybe | Depends on routed model |

### 28.4 Invalid/Nonsensical Option Combinations

These combinations are silently accepted but produce unexpected results:

| Combination | What happens | What should happen |
|------------|-------------|-------------------|
| Perplexity + Workflow=Standard | Pipeline tries tool calls → model can't use tools → empty responses or errors | Workflow options should be hidden or disabled |
| Cerebras + Thinking=Max | Setting ignored, model uses default reasoning | Thinking option should be hidden |
| GLM/Kimi + Thinking=Quick vs Max | Both produce identical behavior (binary enable) | Should show only On/Off, not 4 levels |
| Any provider + Workflow=Express but Clippy=Off, Tests=Off | Runs: implement → compile-only gate → commit. Only compile gate remains. | Warn user that pipeline is almost vacuous |
| Perplexity + Clippy=On | Clippy toggle has no effect (workflow=none is implicit) | Should not show gate options for no-tool models |
| Model switch mid-conversation | Session keeps conversation but new model has different capabilities | Should reset incompatible options |
| kimi-k2 selected | API returns 404 (invalid slug) | Should not be selectable, or should validate on selection |

### 28.5 Model-Specific Thinking Capabilities (What Each Actually Supports)

Only 1 of 36 models has `supports_thinking = true` in roko.toml (`glm-5-1`). But several more support thinking through hardcoded detection in `capability.rs`:

| Model | roko.toml `supports_thinking` | Hardcoded in capability.rs | Actual API support | Effort levels meaningful? |
|-------|------------------------------|---------------------------|-------------------|--------------------------|
| claude-opus | false | No | Yes (thinking tokens) | Yes (budget → thinking_budget_tokens) |
| claude-sonnet | false | No | Yes (thinking tokens) | Yes |
| haiku | false | No | Yes (thinking tokens) | Yes |
| glm-5-1 | **true** | Yes (slug starts_with "glm-5") | Yes (binary only) | No — binary enabled/disabled |
| glm51 | false | Yes (slug starts_with "glm-5") | Yes (binary only) | No |
| glm-5v-turbo | false | Yes | Yes (binary only) | No |
| kimi-k25 | false | Yes (slug starts_with "kimi-k2") | Yes (binary only) | No |
| kimi-k2-5 | false | Yes | Yes (binary only) | No |
| o3 | false | No | Yes (reasoning_effort) | Yes (low/medium/high) |
| o3-mini | false | No | Yes (reasoning_effort) | Yes |
| o4-mini | false | No | Yes (reasoning_effort) | Yes |
| gemini-2-5-pro | false | No | Yes (thinking_budget) | Yes |
| gemini-2-5-flash | false | No | Yes (thinking_budget) | Yes |
| All others | false | No | No | Thinking option is pure noise |

**Summary**: Thinking options should be:
- **4 levels** (Quick/Standard/Deep/Max) for: Claude, OpenAI reasoning models, Gemini
- **Binary** (On/Off) for: GLM, Kimi
- **Hidden** for: Cerebras, Perplexity, Llama, GPT-4o/5.4-mini (non-reasoning), Gemma

### 28.6 Ideal UX Design

**Principle**: Options should be **adaptive** — show only what's meaningful for the current provider+model combination. Never show a toggle that does nothing.

**Option visibility rules**:

```
Provider selected → filter models to that provider
Model selected → determine capabilities from ModelProfile:
  - If supports_thinking:
      If provider has level support (Claude/OpenAI-o*/Gemini) → show 4-level Thinking selector
      If provider has binary only (GLM/Kimi) → show On/Off toggle
      Else → hide Thinking
  - If !supports_thinking → hide Thinking entirely
  - If supports_tools:
      Show Workflow selector
      Show Clippy/Tests (only if Workflow ≠ none)
  - If !supports_tools:
      Hide Workflow (force to "none" internally)
      Hide Clippy/Tests

Workflow selected:
  - If "none" → hide Clippy/Tests/Review/Iterations (they're irrelevant)
  - If "express" → show Clippy/Tests
  - If "standard" → show Clippy/Tests + show Review Strictness
  - If "full" → show all (Clippy/Tests/Review/Iterations)
  - If "auto" → show all (pipeline may use them)
```

**Capability metadata on ModelProfile** (additions needed):

```rust
pub struct ModelProfile {
    // existing fields...
    pub thinking_mode: ThinkingMode, // NEW: replaces bool
    pub tool_capability: ToolCapability, // NEW: more granular
}

pub enum ThinkingMode {
    None,                    // Model has no thinking support
    Binary,                  // GLM/Kimi: enabled or disabled, no levels
    Leveled(Vec<String>),    // Claude/OpenAI/Gemini: supports effort levels
}

pub enum ToolCapability {
    None,                    // Perplexity: no tools at all
    TextOnly,                // ReAct-style text parsing (unreliable)
    Native,                  // Full JSON tool calling
}
```

**Dynamic config options response**:

```rust
fn build_config_options(
    state: &SessionConfigState,
    roko_config: &RokoConfig,
) -> Vec<ConfigOption> {
    let model_profile = roko_config.models.get(&state.model);
    let mut options = vec![provider_option, model_option]; // always present

    // Thinking: adaptive to model capability
    if let Some(profile) = model_profile {
        match profile.thinking_mode {
            ThinkingMode::None => { /* don't add thinking option */ }
            ThinkingMode::Binary => {
                options.push(thinking_binary_option(state)); // On / Off
            }
            ThinkingMode::Leveled(ref levels) => {
                options.push(thinking_level_option(state, levels)); // Quick/Standard/Deep/Max
            }
        }

        // Workflow: only for tool-capable models
        if profile.tool_capability != ToolCapability::None {
            options.push(workflow_option(state));

            // Gates: only when workflow is active
            if state.workflow != "none" {
                options.push(clippy_option(state));
                options.push(tests_option(state));
            }
        }
    }

    options
}
```

**Provider health indicator**: Each provider option should show a status:
- Green dot: API key present, last health check passed
- Yellow dot: API key present, no recent health check
- Red dot: API key missing or last check failed
- This prevents users from selecting a provider and getting a cryptic 401/404

**Batch 28 update (2026-05-03)**: ACP provider options now include unavailable configured providers instead of filtering them out. Option descriptions indicate `Ready`, missing API key env, missing API key configuration, or generic unavailable status. Model options are no longer hidden only because the selected provider is missing credentials. This makes missing env/config visible before dispatch. Remaining work: endpoint reachability checks, slug pre-validation, and richer error formatting.

**Model validation on selection**: When user selects a model, pre-validate:
1. Provider API key exists
2. Model slug is likely valid (check against known model lists if available)
3. If model changed and current thinking/workflow settings are incompatible, auto-adjust and notify

**Batch 30 update (2026-05-03)**: ACP config updates now validate selected provider/model keys against the current config. Unknown providers are ignored; unknown models and cross-provider model selections are ignored. This prevents stale editor state or malformed clients from moving the session back to unconfigured provider/model IDs after resume revalidation. Remaining work: validate configured slugs against provider-known model lists where available and surface a user-visible warning.

**Option tooltips with capability info**: Each model option should show what it supports:
```
Glm 5.1              ⚡ thinking  🔧 tools  📦 128K context
Sonar Pro             🔍 search              📦 127K context
Claude Sonnet         ⚡ thinking  🔧 tools  📦 200K context  🖼️ vision
```

### 28.7 Error Message Quality

Current: `Error: model stream failed: agent error (kimi-k2): network error: http 404: {"error":{"message":"Not found the model kimi-k2 or Permission denied","type":"resource_not_found_error"}}`

This is a raw JSON blob from the API. Should be:

```
Model "kimi-k2" not found on Moonshot API.
Possible causes:
  • Model slug may be incorrect (configured as "kimi-k2", try "kimi-k2.5" or "kimi-k2.6")
  • Your API key may not have access to this model

Run `roko config providers health moonshot` to diagnose.
```

---

## 30. PRD Pipeline Demo: Workspace CWD Mismatch (2026-05-03)

**Symptom**: `roko prd draft promote` and `roko prd plan` fail with "draft not found" / "PRD not found" when run from the demo app's PRD pipeline scenario.

**Observed sequence** (from terminal output):
```
/var/folders/.../roko-prd-pipeline-1777822235431 % roko prd idea "..."       # ✓ works
/var/folders/.../roko-prd-pipeline-1777822235431 % roko prd draft new "..."  # ✓ agent writes draft
~/dev/nunchi/roko/roko % roko prd draft promote btc-funding-alert-cli       # ✗ "draft not found"
~/dev/nunchi/roko/roko % roko prd plan btc-funding-alert-cli                # ✗ "PRD not found"
~/dev/nunchi/roko/roko % roko plan validate .roko/plans                     # 0 diagnostics (empty)
~/dev/nunchi/roko/roko % roko plan run .roko/plans                          # "No plans found"
```

### 30.1 Root Cause: CWD Shifts Between Pipeline Steps

The draft file gets created at `/var/folders/.../roko-prd-pipeline-1777822235431/.roko/prd/drafts/btc-funding-alert-cli.md` but subsequent commands run from `~/dev/nunchi/roko/roko`, where that file doesn't exist.

**Why the CWD shifts**: The `prd draft new` command spawns an LLM agent that runs tool calls (4× `read_file`, 1× `write_file`). The agent's `read_file` calls attempt to read codebase files like `Cargo.toml`, `crates/roko-core/...`, etc. These relative paths don't exist in the temp workspace (it only has the scaffolded `src/main.rs`, `src/lib.rs`, `Cargo.toml`). The agent's tool calls that reference project-relative paths fail or cause the tool system to resolve against the project root rather than the temp workspace. If the agent subprocess changes the shell's CWD during execution, subsequent commands inherit the new CWD.

**Contributing factors**:

1. **`resolve_workdir()` uses `.` (CWD), never `--repo`** (`main.rs:2488-2507`): Every roko command resolves its workspace from the shell's current directory. The `roko()` helper in `terminal-session.ts:90-97` never injects `--repo`:
   ```typescript
   export function roko(ctx: ScenarioContext, subcommand: string): string {
     const bin = getRoko();
     const model = ctx.activeModel;
     if (model) return `${bin} --model ${model} ${subcommand}`;
     return `${bin} ${subcommand}`;
   }
   ```

2. **No CWD assertion between pipeline steps**: The scenario runner (`prd-pipeline.ts`) does `enterWorkspace(main, dir)` once at the start but never verifies the terminal is still in the correct directory after each command.

3. **Temp workspace is minimal**: `POST /api/workspaces` (`workspaces.rs:88-137`) creates a bare workspace with `.roko/` layout and `roko.toml`. The `rustSetupCommand()` adds `Cargo.toml` + `src/main.rs` + `src/lib.rs`. But the agent's `read_file` tool calls look for codebase files that don't exist in this minimal scaffold.

4. **Agent path resolution is ambiguous**: The `prd draft new` prompt tells the agent to write to `{target.display()}` where target is constructed from `workdir`. If workdir is `.` (relative), the agent sees a relative path and may resolve it differently depending on its own CWD semantics.

### 30.2 Why All Subsequent Steps Fail

Once promote fails, the entire downstream pipeline is dead:
- `prd plan` requires a published or draft PRD file at the slug path → file doesn't exist
- `plan validate` requires plans in `.roko/plans` → no plan was generated
- `plan run` requires at least one `tasks.toml` → nothing to execute
- `learn all` works because it reads from wherever `.roko/learn/` lives

### 30.3 The Fix (Structural, Not Band-Aid)

The root problem is that ephemeral workspaces have no stable identity and every roko command re-discovers the workspace from CWD. Three things need to change:

**A. Inject `--repo <dir>` into every roko command in the scenario runner.**

The `roko()` helper should accept the workspace dir and always pass `--repo`:
```typescript
export function roko(ctx: ScenarioContext, subcommand: string): string {
  const bin = getRoko();
  const parts = [bin];
  if (ctx.workspaceDir) parts.push(`--repo "${ctx.workspaceDir}"`);
  if (ctx.activeModel) parts.push(`--model ${ctx.activeModel}`);
  parts.push(subcommand);
  return parts.join(' ');
}
```

This makes workspace identity explicit and CWD-independent. Every command hits the same `.roko/` directory regardless of where the shell thinks it is.

**B. Add CWD guard to `enterWorkspace` or the scenario runner.**

After each command that spawns a subprocess (especially agent commands), assert the terminal is still in the expected directory:
```typescript
async function assertCwd(handle: TerminalHandle, expected: string): Promise<void> {
  const result = await handle.execCmd('pwd', 3000);
  if (!result.stdout?.trim().endsWith(expected)) {
    await handle.execCmd(`cd "${expected}"`, 3000);
  }
}
```

**C. Make `resolve_workdir()` propagate absolute paths through agent subprocesses.**

Currently `resolve_workdir()` returns `PathBuf::from(".")` when no `--repo` is given. Agent subprocesses inherit this relative resolution. Instead, canonicalize the workdir early and pass absolute paths to agent prompts:
```rust
fn resolve_workdir(cli: &Cli) -> PathBuf {
    let dir = cli.repo.clone().unwrap_or_else(|| PathBuf::from("."));
    // ... existing .roko/ autocorrect ...
    // Always canonicalize to avoid relative-path drift in subprocesses
    dir.canonicalize().unwrap_or(dir)
}
```

**Batch 31 update (2026-05-03)**:
- `resolve_workdir()` now canonicalizes existing workdirs, including `--repo`, before returning them or doing `.roko/` autocorrection.
- The demo `roko(ctx, ...)` command helper now injects `--repo '<ctx.workspaceDir>'` into every generated command, with shell quoting, before optional `--model`.
- This makes PRD pipeline commands workspace-explicit even if an agent subprocess or terminal state changes CWD between steps.
- Verified with CLI bin resolver tests and `npm run build` for the demo app.

**Batch 32 update (2026-05-04)**:
- Added `ensureWorkspaceCwd()` to `demo/demo-app/src/lib/terminal-session.ts` as the shared CWD guard for long-lived terminal sessions.
- `enterWorkspace()` now uses the guard, and `showCmd()` can run it as a `workspaceDir` preflight before typing visible commands.
- `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts` now guards hidden scaffold/init setup and every generated visible PRD pipeline command.
- Guard failures mark the pipeline failed with the expected workspace path instead of allowing later PRD commands to fail with misleading missing-draft/missing-PRD errors.
- Verified with `npm run build` in `demo/demo-app`.

**Batch 33 update (2026-05-04)**:
- Added a CLI E2E regression for the demo command shape: commands run from an initialized decoy CWD while passing `--repo <selected-workspace>`.
- The E2E covers `prd idea`, `prd draft new`, `prd draft promote`, `prd plan`, and `plan validate .roko/plans`.
- It asserts the idea, draft, published PRD, `tasks.toml`, and `plan.md` land in the selected workspace and not the decoy CWD.
- Added a dedicated `mock-prd-pipeline-fixture` with explicit background distillation turns so the planning turn is deterministic.
- Fixed `roko plan validate` to resolve relative plan paths and file-reference validation against `resolve_workdir(cli)`, so global `--repo` is honored instead of process CWD.
- Verified with `CARGO_TARGET_DIR=target/codex-batch33 cargo test -p roko-cli --test prd_pipeline_workspace -- explicit_repo_prd_pipeline_artifacts_stay_in_selected_workspace --nocapture`.

### 30.4 Files to Change

| File | What | Why |
|---|---|---|
| `demo/demo-app/src/lib/terminal-session.ts:90-97` | Add `--repo` to `roko()` helper | Commands become CWD-independent |
| `crates/roko-cli/src/main.rs:2488-2507` | Canonicalize `resolve_workdir()` result | Absolute paths survive subprocess hops |
| `crates/roko-cli/src/commands/prd.rs:326-397` | Use canonicalized workdir in agent prompt path | Agent sees absolute path, not relative |
| `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts` | Add CWD assertion between steps | Done in batch 32; hidden setup and visible PRD commands preflight through `ensureWorkspaceCwd()` |
| `crates/roko-cli/src/commands/plan.rs` | Make `plan validate` honor global `--repo` | Done in batch 33; relative plan paths and validation workdir are resolved from `resolve_workdir(cli)` |
| `crates/roko-cli/tests/prd_pipeline_workspace.rs` | E2E PRD pipeline workspace regression | Done in batch 33; verifies selected workspace receives all PRD/plan artifacts while decoy CWD remains untouched |

---

## 29. Final Priority Matrix (Updated)

| Priority | Area | Effort | Impact | Section |
|---|---|---|---|---|
| **P0** | Fix `prd draft new` exit code (artifact check) | Low | Demo pipeline stops failing falsely | S22.3 |
| **P0** | Add `max_output` to all models in roko.toml | Low | Done batch 36; root + Railway configs have regression coverage | S23.2 |
| **P0** | Fix safety contract loading (CARGO_MANIFEST_DIR) | Low | Done batch 38; contract assets are embedded and no longer depend on build-machine source paths | S12.4 |
| **P0** | Fix gate_passed LLM bypass | Low | Done batch 35; gate/cost/token contract checks no longer trust call-argument claims | S12.5 |
| **P0** | Unify config loading (12 loaders → 1) | Medium | Eliminates CLI vs serve vs ACP config divergence | S4, S27.2 |
| **P0** | ACP global config loading | Low | Non-roko projects see all user providers | S27.1 |
| **P0** | Central constants (TTFT, max_tokens, iterations) | Low | Request-timeout, core retry-policy, serve relay, runner DAG, provider tool-loop, and vision-loop defaults done batches 39-47; active workflow iteration literals remain | S6.1 |
| **P0** | Health endpoint + graceful shutdown | Low | Done batch 37 for top-level `/health`/`/ready` and `roko up` graceful serve shutdown; Docker/Railway sidecar work remains | S5 |
| **P1** | Add heartbeat/progress for OpenAI-compat agents | Low | Eliminates silent hangs | S22.1 |
| **P1** | Detect `__truncated` tool calls at dispatcher | Low | Clear error instead of silent failure | S22.4 |
| **P1** | Wire `on_turn` callback for PRD commands | Low | Per-iteration progress feedback | S22.5 |
| **P1** | Fix Claude CLI zero-token usage | Medium | Cost tracking for most expensive models | S23.6 |
| **P1** | Proper tool schemas for LLMs | Medium | Reduces hallucinated arguments | S12.1 |
| **P1** | Config caching with file watch | Medium | Eliminates 70+ disk reads per plan | S13.6 |
| **P1** | Workspace persistence | Medium | Fixes page-refresh data loss | S2 |
| **P1** | Safety layer required, not optional | Low | Eliminates unguarded dispatchers | S12.3 |
| **P1** | Fix parallel task JoinError handling | Low | Tasks no longer silently dropped | S13.2 |
| **P1** | Experiment persistence | Low | Experiment outcomes survive restart | S16.4 |
| **P1** | Proper Dockerfile | Medium | Cuts image size by ~1.5GB | S5 |
| **P2** | Prometheus `/metrics` endpoint | Medium | Live observability for running instances | S24.2 |
| **P2** | Streaming-first architecture | High | Universal progress, measurable TTFT | S25.1 |
| **P2** | Per-model token budget config | Medium | Fine-grained control, adaptive strategy | S25.2 |
| **P2** | Progress event bus | Medium | Unified progress for CLI/TUI/demo/bench | S25.3 |
| **P2** | Dev orchestrator (`roko dev`) | Medium | Eliminates triple-spawn, port conflicts | S1 |
| **P2** | Replace unbounded channels | Medium | Prevents OOM under load | S15.5 |
| **P2** | Fix sync Mutex in async contexts | Medium | Prevents Tokio worker starvation | S15.1 |
| **P2** | Integration test suite | High | Prevents regressions, enables CI | S20 |
| **P2** | Terminal session reattach | High | Fixes page-refresh terminal loss | S3 |
| **P3** | Distributed tracing (OTLP) | High | End-to-end request tracing | S24.3 |
| **P3** | Model comparison dashboard | Medium | Informed model selection | S25.6 |
| **P3** | Error category tracking | Medium | Root cause analysis | S24.6 |
| **P3** | WAL for critical state | High | Crash recovery for learning data | S19.1 |
| **P3** | Bench regression detection | Medium | Automated quality alerting | S24.10 |
| **P3** | Context window utilization metric | Low | Capacity analysis | S24.9 |
| **P3** | Demo app SSE migration | Medium | Eliminates polling | S7.1 |
| **P3** | Error type hierarchy | High | Consistency, better errors | S6.6 |
| **P1** | ACP session re-validation on config change | Low | Stale provider/model references cleaned up | S27.4 |
| **P1** | Kill static fallback config options | Low | Eliminates hardcoded Anthropic/Sonnet | S27.3 |
| **P2** | Model slug dedup + validation at load time | Low | Catches duplicate keys, invalid slugs | S27.7 |
| **P2** | ACP workdir + config visibility in Zed | Low | User knows which config is active | S27.5 |
| **P0** | Fix PRD pipeline CWD mismatch (`--repo` injection) | Low | Pipeline demo completely broken | S30 |
| **P0** | Wire ACP effort/thinking to dispatch | Low | 1 of 6 visible options is pure theater | S28.2 |
| **P0** | Adaptive config options by model capability | Medium | Eliminates nonsensical option combinations | S28.6 |
| **P1** | Model slug pre-validation on selection | Low | Prevents kimi-k2 style 404s at runtime | S28.4 |
| **P1** | Provider health indicator in ACP | Medium | Users see availability before selecting | S28.6 |
| **P1** | Human-readable ACP error messages | Low | Replaces raw JSON error blobs | S28.7 |
| **P2** | ThinkingMode enum (None/Binary/Leveled) | Medium | Options match actual model capabilities | S28.5 |
| **P2** | Remove or wire hidden config options | Low | temperament, routing_mode, review_strictness are dead code | S28.2 |

---

## 31. Chat Mode Freeze: Terminal Unresponsive After Error (2026-05-04)

**Observed**: Running `roko` from `~` (home directory), sending "hello", getting "no API key for provider 'anthropic_api'" error. Terminal then freezes — Ctrl+C has no effect, must kill the terminal.

### 31.1 Root Cause: Synchronous Event Poll Blocks Signal Handling

**Severity**: P0 — Terminal becomes permanently unresponsive.

**File**: `crates/roko-cli/src/chat_inline.rs:1280`

The main event loop uses `crossterm::event::poll(Duration::from_millis(33))` — a synchronous blocking syscall. This runs on the tokio thread but does NOT integrate with tokio's async signal handling. During the 33ms poll window, the process cannot handle Ctrl+C via the normal async pathway.

Worse: in the `Phase::Error` state (lines 1323-1338), the key handler only matches `'r'` (retry), `'q'` (quit), and `Esc`. **Ctrl+C is caught by the `_ => {}` wildcard and silently discarded.**

```rust
// chat_inline.rs:1323-1338
Phase::Error { ref prompt, .. } => {
    match key.code {
        KeyCode::Char('r') => { /* retry */ }
        KeyCode::Char('q') | KeyCode::Esc => { /* cancel */ }
        _ => {} // <-- Ctrl+C lands here and is EATEN
    }
}
```

**Contrast**: The codebase already has working signal handling in `chat_session.rs:1306` and `orchestrate.rs:832` using `tokio::select!` with `signal::ctrl_c()`. The inline chat doesn't use this pattern.

### 31.2 No Raw Mode Cleanup Guard

**File**: `crates/roko-cli/src/inline/terminal.rs:52`

`enable_raw_mode()` is called in `InlineTerminal::new()` but there's no RAII `Drop` guard to restore normal terminal mode. If the process panics, exits abnormally, or gets stuck, the terminal remains in raw mode — which means:
- No line editing
- No echo
- Ctrl+C doesn't generate SIGINT (it's a raw keypress)
- The terminal appears "frozen"

### 31.3 Error Phase Doesn't Surface Recovery Options

When the API key error fires, the UI shows the error but the only documented escape is `'q'`/`Esc` (which returns to input mode where the same error will recur) or `'r'` (retry, same error). There's no way to:
- Exit the program
- Configure the missing API key
- Switch providers

### Redesign

1. **Add Ctrl+C handling to ALL phases**, including `Phase::Error`:
   ```rust
   Phase::Error { .. } => {
       match key.code {
           KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
               session.phase = Phase::Done;
               break;
           }
           // ... existing handlers
       }
   }
   ```

2. **Add RAII terminal cleanup guard**:
   ```rust
   struct RawModeGuard;
   impl Drop for RawModeGuard {
       fn drop(&mut self) {
           let _ = disable_raw_mode();
       }
   }
   ```
   Create the guard immediately after `enable_raw_mode()` so cleanup happens on any exit path.

3. **Register a panic hook** that restores terminal mode before printing the panic:
   ```rust
   let default_hook = std::panic::take_hook();
   std::panic::set_hook(Box::new(move |info| {
       let _ = disable_raw_mode();
       default_hook(info);
   }));
   ```

4. **Add async signal integration** — wrap the event poll in `tokio::select!` so Ctrl+C works even when the event loop is blocked:
   ```rust
   tokio::select! {
       _ = tokio::signal::ctrl_c() => { break; }
       event = poll_terminal_event() => { /* handle */ }
   }
   ```

5. **Validate provider availability at startup** — before entering the event loop, check that at least one provider has a valid API key. If not, print a clear message and exit immediately instead of entering the chat REPL.

---

## 32. ACP "Failed to Launch" in Zed (2026-05-04)

**Observed**: Zed shows "Failed to Launch — Internal error: server shut down unexpectedly" when trying to use roko as an ACP provider.

### 32.1 Root Cause: Config Load Failure = Silent Crash

**File**: `crates/roko-acp/src/handler.rs:26-35`

The ACP server entry point calls `config.load_roko_config()` which uses `.unwrap_or_default()` — so config failures are swallowed. But the subsequent `setup_file_logging(config.log_file())` (line 27) can fail if the log directory doesn't exist:

```rust
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    let _guard = setup_file_logging(config.log_file()).with_context(|| {
        format!("failed to initialize ACP logging at {}", config.log_file().display())
    })?;  // <-- If .roko/ doesn't exist, this fails and ACP exits
```

When Zed launches `roko acp --workdir .`, if the project directory has no `.roko/` folder, the log file path `.roko/acp.log` is invalid → logging setup fails → `run_acp_server` returns `Err` → `main.rs:1768` prints to stderr (which Zed doesn't show) → `std::process::exit(1)`.

Zed sees the process exit immediately and reports "server shut down unexpectedly."

### 32.2 No Startup Diagnostics to Editor

**File**: `crates/roko-cli/src/main.rs:1765-1774`

The ACP error path writes to stderr:
```rust
Err(e) => {
    eprintln!("error: {e:#}");
    EXIT_FAILURE
}
```

But Zed's ACP integration captures stdout (the JSON-RPC channel) and may not surface stderr. The editor gets no structured error information — just a dead process.

### 32.3 Workdir Default is Relative

**File**: `crates/roko-cli/src/main.rs:483`

```rust
#[arg(long, default_value = ".")]
workdir: PathBuf,
```

The default `"."` is relative to whatever CWD the editor launches the process with. Zed may launch from the workspace root, the user's home dir, or some other location depending on the project configuration. There's no canonicalization of this path.

### 32.4 No .roko/ Auto-Creation in ACP Path

The CLI chat path (`unified.rs:44`) calls `ensure_workspace(&workdir)` which creates `.roko/` if missing. The ACP path does NOT call this — it assumes `.roko/` already exists.

### Redesign

1. **Auto-create `.roko/` in ACP path** — call `ensure_workspace()` before attempting to write the log file
2. **Canonicalize workdir** — `std::fs::canonicalize(workdir)` at ACP startup
3. **Graceful log fallback** — if `.roko/acp.log` fails, fall back to `/tmp/roko-acp-{pid}.log` or memory-only logging
4. **JSON-RPC error response before exit** — if ACP can't start, send a proper JSON-RPC error response on stdout before exiting, so the editor can display a meaningful message
5. **Startup self-check** — validate config, providers, and required directories before entering the server loop; report issues as editor notifications

---

## 33. Config Resolution Chaos: Same Machine, Different Config (2026-05-04)

**Observed**: Running `roko` from `~` shows `glm-5.1 (OpenAI-compat)` as the auth method, then fails with "no API key for provider 'anthropic_api'". This reveals multiple interconnected config resolution failures.

### 33.1 Auth Detection ≠ Dispatch Config

**Files**: `crates/roko-cli/src/auth_detect.rs`, `crates/roko-cli/src/chat_inline.rs`

`detect_auth()` probes env vars in priority order: Claude CLI → ANTHROPIC_API_KEY → ZAI_API_KEY → OPENAI_API_KEY. It finds ZAI_API_KEY and reports `glm-5.1 (OpenAI-compat)`.

But the actual dispatch path loads `roko.toml` config and resolves models through `ChatAgentSession`, which may select a different model/provider than what auth detection found. The auth detection and dispatch config are **two completely independent systems** that can disagree.

**Result**: The banner says "auth: glm-5.1 (OpenAI-compat)" but the actual dispatch tries to use `anthropic_api` because that's what the config's default model points to.

### 33.2 Home Directory Workspace Trap

Running `roko` from `~` creates `~/.roko/` (the workspace) which collides with `~/.roko/config.toml` (the global config). The global config directory and the workspace directory are the same path when CWD is `~`. This is not detected or warned about.

### 33.3 The 20+ Config Loader Problem

Updated count from the original 12-loader audit: there are now **20+ config loading entry points** scattered across the codebase:

| # | Function | Location |
|---|----------|----------|
| 1 | `AcpConfig::load_roko_config` | `roko-acp/src/config.rs:48` |
| 2 | `load_roko_config` | `roko-cli/src/config_helpers.rs:121` |
| 3 | `load_roko_config` | `roko-cli/src/orchestrate.rs:863` |
| 4 | `load_roko_config` | `roko-cli/src/agent_serve.rs:559` |
| 5 | `load_roko_config` | `roko-cli/src/main.rs:2480` |
| 6 | `load_roko_config` | `roko-cli/src/event_sources.rs:77` |
| 7 | `load_roko_config` | `roko-cli/src/subscriptions.rs:249` |
| 8 | `load_roko_config` | `roko-cli/src/vision_loop/orchestrator.rs:305` |
| 9 | `load_roko_config` | `roko-serve/src/lib.rs:437` |
| 10 | `AppState::load_roko_config` | `roko-serve/src/state.rs:626` |
| 11 | `load_layered` | `roko-cli/src/config.rs:2811` |
| 12 | `load_config` | `roko-core/src/config/mod.rs:101` |
| 13 | `load_config_strict` | `roko-core/src/config/mod.rs:112` |
| 14 | `load_config_unified` | `roko-core/src/config/loader.rs:72` |
| 15 | `load_config_with_options` | `roko-core/src/config/loader.rs:77` |
| 16 | `load_config_file` | `roko-core/src/config/loader.rs:90` |
| 17 | `load_config_validated` | `roko-core/src/config/loader.rs:97` |
| 18 | `load_config_validated_with_options` | `roko-core/src/config/loader.rs:106` |
| 19 | `load_roko_config_file` | `roko-cli/src/serve_runtime.rs:474` |
| 20 | `load_roko_config_models` | `roko-cli/src/run.rs:2983` |

Each has different behavior regarding global config merge, env var overrides, validation strictness, and error handling. This is the root cause of "same machine, different behavior depending on entry point."

### 33.4 Auth Method → Provider Mapping is Hardcoded

`detect_auth()` returns `AuthMethod::OpenAiCompat { key, base_url, model }` but this struct has no connection to the `roko.toml` provider registry. The chat inline path has to independently re-resolve which configured provider matches the detected auth method. If the mapping disagrees, the user sees one thing in the banner and another in the error.

### Redesign

1. **Separate global config dir from workspace dir** — `~/.roko/` is always the global config location. The workspace `.roko/` is `$CWD/.roko/`. When CWD is `~`, warn and skip creating a workspace (use in-memory or `/tmp/`).

2. **Auth detection should USE the config, not bypass it** — instead of probing env vars directly, `detect_auth()` should:
   a. Load the unified config (which already knows about providers and their env vars)
   b. Check which configured providers have valid credentials
   c. Select the best available provider from config
   d. Return the config-resolved provider, not an independent struct

3. **Single config loader, period** — collapse all 20 loaders into `roko_core::config::loader::load(workdir, opts)`. Everything else calls this. The core loader always merges global config, applies env overrides, and validates.

4. **Config validation at startup** — before entering any interactive mode, validate that:
   - At least one provider has a valid API key
   - The default model's provider is configured and credentialed
   - Required directories exist
   - Print actionable diagnostics if anything is missing

---

## 34. Cross-Cutting Root Causes (2026-05-04)

The three user-visible problems (chat freeze, ACP crash, config confusion) share deeper architectural issues:

### 34.1 No "Boot Sequence" Abstraction

Each entry point (CLI chat, ACP, serve, plan run, oneshot) implements its own ad-hoc startup:
- Different config loading
- Different workspace initialization
- Different signal handling
- Different error reporting

There should be a single `RokoBootstrap` that every entry point calls:

```rust
struct RokoBootstrap {
    config: RokoConfig,
    workdir: PathBuf,
    available_providers: Vec<ProviderStatus>,
    workspace_ready: bool,
}

impl RokoBootstrap {
    fn new(workdir: &Path, opts: BootOpts) -> Result<Self, BootError> {
        // 1. Canonicalize workdir
        // 2. Load unified config (global + project + env)
        // 3. Ensure workspace (.roko/) exists
        // 4. Validate providers (check API keys)
        // 5. Return boot state or actionable error
    }
}
```

### 34.2 No Graceful Degradation Pattern

Current behavior: missing API key → error message → frozen terminal. No provider → ACP crashes.

Should be: missing API key → try next provider → if no providers available → clear message with setup instructions → clean exit.

### 34.3 Terminal Lifecycle is Unmanaged

Raw mode is entered without a cleanup guard. Signals are handled inconsistently. Panic hooks don't restore terminal state. The pattern should be:

```rust
// RAII guard ensures cleanup on ALL exit paths (normal, error, panic)
let _terminal_guard = TerminalGuard::enter_raw_mode()?;
// ... rest of chat mode
// Drop automatically restores terminal on scope exit
```

### 34.4 Error Reporting is Entry-Point-Specific

CLI errors go to stderr. ACP errors go to stderr (invisible to Zed). Serve errors go to tracing. There's no unified error reporting strategy that matches the entry point's communication channel.

| Entry Point | Error Channel | Current | Should Be |
|---|---|---|---|
| CLI chat | Terminal (with raw mode) | eprintln (broken in raw mode) | Restore terminal first, then print |
| ACP | JSON-RPC stdout | eprintln (invisible) | JSON-RPC error response |
| Serve | HTTP | tracing + HTTP 500 | Correct |
| Plan run | Terminal | eprintln | Correct |

---

## Updated Priority Matrix (2026-05-04 additions)

| Priority | Area | Effort | Impact | Section |
|---|---|---|---|---|
| **P0** | Chat mode Ctrl+C freeze | Low | Terminal completely unresponsive | S31.1 |
| **P0** | Terminal raw mode cleanup guard | Low | Terminal restored on any exit | S31.2 |
| **P0** | ACP auto-create .roko/ | Low | Fixes "Failed to Launch" in Zed | S32.4 |
| **P0** | ACP log file fallback | Low | ACP doesn't crash on missing dir | S32.3 |
| **P0** | Auth detect uses config | Medium | Banner matches actual dispatch | S33.1 |
| **P0** | Unified config loader (20→1) | Medium | Same config everywhere | S33.3 |
| **P1** | Boot sequence abstraction | Medium | All entry points share init | S34.1 |
| **P1** | ACP JSON-RPC error on startup failure | Low | Zed shows actual error | S32.2 |
| **P1** | Home dir workspace collision detection | Low | Warns when CWD=~ | S33.2 |
| **P1** | Startup provider validation | Low | Early fail with instructions | S33.4 |
| **P1** | Panic hook terminal restore | Low | Raw mode cleaned up on panic | S31.2 |
| **P2** | Graceful degradation (provider fallback) | Medium | Tries next provider on auth fail | S34.2 |
| **P2** | Entry-point-appropriate error reporting | Medium | Errors go to right channel | S34.4 |

---

## 35. First-Run Experience Friction (2026-05-04)

### 35.1 Silent Fallback to `cat` Agent

**Severity**: P1 — New user gets echoed input with no explanation.

Running `roko` in a directory without `roko.toml` prints a warning ("no config found — agent command is 'cat'") but then enters the chat REPL with the `cat` agent, which just echoes input back. The user thinks roko is broken.

**Should**: Refuse to enter chat mode without a valid provider. Print actionable setup instructions:
```
No roko.toml found and no providers configured.
Run `roko init` to create a config, or set ANTHROPIC_API_KEY.
```

### 35.2 No Provider Validation During `roko init`

`roko init` generates a `roko.toml` with `claude_cli` as default provider but never checks if `claude` is actually installed or if any API keys are available. After init, the user immediately hits errors on first use.

**Should**: At the end of `roko init`, run a quick validation:
1. Check if the configured provider binary exists on PATH
2. Check if API keys are set for API providers
3. Print a summary: "Provider: claude CLI (found on PATH)" or "WARNING: claude not found — install it or configure a different provider"

### 35.3 No Guided Provider Setup

`roko config init` wizard asks for agent command but doesn't:
- List available provider options ("Choose a provider: 1. Claude CLI, 2. Anthropic API, 3. OpenAI, 4. Ollama...")
- Prompt for API keys
- Test the provider connection
- Explain what each provider does

Users must already know what to configure and manually edit TOML.

### 35.4 Doctor Doesn't Offer to Fix

`roko doctor` reports problems ("[fail] config: missing project roko.toml") but never offers to fix them. It's diagnostic-only.

**Should**: For fixable issues, offer the fix:
```
[fail] config: missing project roko.toml
  fix: run `roko init` to create one
```

### 35.5 No Example `.env` or Starter Template

The repo has a comprehensive 22KB `roko.toml` (54 models, 11 providers) but no minimal starter template or `.env.example` file. Users see the full config and are overwhelmed.

**Should**: Provide `roko.toml.minimal` with just 1 provider and 1 model, plus `.env.example` listing all supported env vars.

---

## 36. Error Message Quality Audit (2026-05-04)

### 36.1 Silent Error Swallowing — 120+ instances

`let _ = ...` and `.ok()` without logging or fallback across production code:

| File | Count | Risk |
|---|---|---|
| `roko-acp/src/bridge_events.rs` | 60+ | Cognitive events drop silently; TUI stops updating |
| `roko-neuro/src/context.rs` | 3 | Knowledge store writes fail silently; tier progression lost |
| `roko-serve/src/relay.rs:357` | 1 | Agent registration fails silently; broken deploy |
| `roko-agent/src/process/kill.rs` | 3 | Process kill fails silently; resource leak |
| `roko-gate/src/generated.rs` | 3 | Gate cleanup fails; stale fixtures pile up |

**Fix pattern**: Every `let _ =` should either:
1. Emit `tracing::warn!` with context
2. Contribute to a failed `Result`
3. Be commented as deliberate cleanup

### 36.2 Unwrap/Expect in Production — 150+ instances

| File | Count | Worst Example |
|---|---|---|
| `roko-chain/src/marketplace.rs` | 86 | `self.jobs.get_mut(job_id).unwrap()` |
| `roko-neuro/src/distiller.rs` | 8 | `.expect("distill")` — user sees "thread panicked at: distill" |
| `roko-mcp-scripts/src/main.rs` | 10 | `fs::create_dir_all(&dir).expect("create scripts dir")` |
| `roko-core/src/error/mod.rs` | 2 | `panic!("wrong variant")` **inside a Display impl** |

**Fix**: Replace with `.context("descriptive message")?` — never panic in production code.

### 36.3 Panic in Display Impl

**File**: `roko-core/src/error/mod.rs:652-685`

The `Display` implementation for an error type contains `panic!("wrong variant")`. This means formatting an error can itself panic, producing a double fault with no useful information.

### 36.4 Generic Error Messages

Many `anyhow!()` / `bail!()` calls lack context:
```rust
// BAD:
bail!("agent message response did not include run_id or direct response");
// User has no idea: is the sidecar broken? The proxy? The agent?

// GOOD (from chat_session.rs):
#[error("no API key for provider '{provider}': set {env_var} or configure it in roko.toml")]
ApiKeyMissing { provider: String, env_var: String },
// Tells user exactly what to do
```

**Pattern to follow**: `SessionError` enum with typed variants and `#[error(...)]` macros. Roll this pattern into agent spawn, plan execution, gate, and MCP connection failures.

---

## 37. CLI UX Friction (2026-05-04)

### 37.1 No Progress Feedback During Long Operations

`roko plan run` executes for minutes/hours with zero CLI output. No spinners, no task counters, no cost updates. User must either:
- Open `roko dashboard` in another window
- Tail `.roko/runner-stderr.log` manually
- Poll `roko status` in a loop

**Should**: Print task-level progress: `[3/15] Running task "implement-auth" (claude-sonnet-4-6, $0.42 so far)...`

### 37.2 No `roko status --quick`

`roko status` dumps 50+ lines covering signals, agents, plans, episodes, costs, experiments, thresholds. Most users want a 3-line summary:
```
Plan: auth-refactor (8/12 tasks done, 2 running)
Cost: $1.47 today ($24.99 budget remaining)
Health: 3 providers OK, 1 unhealthy (moonshot: rate limited)
```

### 37.3 No `roko config providers add` Command

Adding a provider requires manually editing `roko.toml`. Should have:
```bash
roko config providers add anthropic --api-key sk-ant-...
roko config providers add openai --api-key sk-...
roko config models add gpt-5 --provider openai --slug gpt-5.4-mini
```

### 37.4 `roko chat` vs `roko` vs `roko run` — Unclear Differences

Three chat-like entry points with no clear guidance on when to use which:
- `roko` (no args) — inline chat with auto-detected provider
- `roko agent chat --agent X` — chat with named agent via serve
- `roko run "<prompt>"` — one-shot compose→agent→gate→persist loop

No help text explains the differences or recommends one over another.

### 37.5 No Streaming in Chat Responses

`roko agent chat` buffers the full response before displaying. User sees nothing for 10-30 seconds, then the entire response appears. Poor UX compared to every other LLM chat interface.

### 37.6 Shell Completions Lack Install Guide

`roko completions bash` outputs the script but doesn't tell the user where to put it. Should include:
```bash
# Add to ~/.bashrc:
eval "$(roko completions bash)"
```

### 37.7 Output Tables are Hand-Formatted

All CLI tables use manual `format!("{:<16} {:<40}")` — columns misalign when content exceeds hardcoded widths. Should use a table formatter library or at minimum dynamic column sizing.

---

## 38. Concurrent Access & File Locking (2026-05-04)

### 38.1 No Multi-Process File Locking

**Severity**: P0 — Silent data corruption.

`RokoLayout` defines a lock file path (`.roko/runtime/roko.lock`) but **never creates or checks it**. Two simultaneous `roko plan run` commands will:
- Both read `.roko/state/executor.json`
- Both append to `.roko/episodes.jsonl`
- Both modify `.roko/learn/*.json`
- Result: Race conditions, lost writes, corrupted state

**Fix**: Advisory file lock via `flock(2)` or `fs2::FileExt::lock_exclusive()` at startup. Second process gets a clear error: "Another roko process is running (PID 12345). Use --force to override."

### 38.2 In-Process Only Concurrency

`tokio::sync::Mutex` and `parking_lot::RwLock` protect in-process access but provide zero protection across processes. The `roko serve` + `roko plan run` combination is explicitly expected to coexist but has no shared-state coordination.

---

## 39. Missing Dependency Pre-Flight (2026-05-04)

### 39.1 Provider Binary Not Found

Running with `claude_cli` provider when `claude` is not installed produces "spawn failed: No such file or directory" **after** the task context is already built. The user waited minutes for nothing.

**Fix**: Check provider binary availability during `RokoBootstrap`, not at dispatch time.

### 39.2 Gate Dependencies Not Checked

Gates assume `cargo`, `git`, `clippy` are available. If missing, the gate fails mid-execution with a system error.

**Fix**: `roko doctor` should check all gate dependencies and report them. `roko plan run` should pre-validate before starting.

### 39.3 Config Migration — No Old-Roko Path

Mori→Roko migration exists (`config/compat.rs`). But if a user has an old `roko.toml` from an earlier roko version, there's no migration or schema version warning. Fields silently default to empty values.

**Fix**: Check `config_version` / `schema_version` fields. If outdated, print: "Config schema version 1 is outdated. Run `roko config migrate` to update."

---

## 40. Build & Developer Friction (2026-05-04)

### 40.1 main.rs Silences ALL Clippy

**File**: `crates/roko-cli/src/main.rs:10-20`

```rust
#![cfg_attr(clippy, allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, missing_docs))]
```

This blanket suppression means CI "passes" clippy but actual issues are hidden. 325 item-level `#[allow(clippy::...)]` suppressions exist across the workspace. Clippy is effectively disabled for the largest file in the codebase.

### 40.2 Dead Code Suppressions

~70 `#[allow(dead_code)]` instances across the workspace, mostly in Phase 2+ scaffolding (chain, audit, effects). This code compiles but is never called. It should be feature-gated or removed.

### 40.3 No rust-toolchain.toml

Workspace requires 1.91+ but doesn't lock it locally via `rust-toolchain.toml`. Developers with older rustc get confusing compilation errors from `alloy` deps.

### 40.4 Build Time

31 crates + `codegen-units = 1` in dev profile makes incremental builds slower than necessary. Consider `codegen-units = 4` for dev builds.

---

## Full Updated Priority Matrix (2026-05-04)

### P0 — Blocks basic usage

| Area | Effort | Section |
|---|---|---|
| Chat mode Ctrl+C freeze | Low | S31 |
| Terminal raw mode cleanup guard | Low | S31.2 |
| ACP auto-create .roko/ | Low | S32.4 |
| ACP log file fallback | Low | S32.3 |
| Auth detect uses config | Medium | S33.1 |
| Unified config loader (20→1) | Medium | S33.3 |
| Multi-process file locking | Medium | S38.1 |

### P1 — Frustrating but workable

| Area | Effort | Section |
|---|---|---|
| Boot sequence abstraction | Medium | S34.1 |
| ACP JSON-RPC error on startup failure | Low | S32.2 |
| Startup provider validation | Low | S33.4, S35.2 |
| Silent fallback to cat agent | Low | S35.1 |
| Provider validation during init | Low | S35.2 |
| Doctor offers fixes | Low | S35.4 |
| Pre-flight dependency check | Medium | S39 |
| Config migration for old roko | Low | S39.3 |
| `roko status --quick` mode | Low | S37.2 |
| Plan run progress output | Medium | S37.1 |

### P2 — Polish and ergonomics

| Area | Effort | Section |
|---|---|---|
| Guided provider setup wizard | Medium | S35.3 |
| `config providers add` command | Medium | S37.3 |
| Chat/run/agent-chat disambiguation | Low | S37.4 |
| Streaming chat responses | Medium | S37.5 |
| Shell completions install guide | Low | S37.6 |
| Table formatting library | Low | S37.7 |
| Starter template / .env.example | Low | S35.5 |
| Remove clippy blanket suppression | Medium | S40.1 |
| Feature-gate dead code | Low | S40.2 |
| Add rust-toolchain.toml | Low | S40.3 |

### P3 — Error quality (ongoing)

| Area | Effort | Section |
|---|---|---|
| Audit 120+ silent error swallows | High | S36.1 |
| Replace 150+ unwrap/expect | High | S36.2 |
| Fix panic in Display impl | Low | S36.3 |
| Typed error enums for all subsystems | High | S36.4 |

---

## 41. Live UX Walkthrough: End-to-End Output Problems (2026-05-04)

Tested a full workflow in `/tmp/roko-ux-test` (init → prd idea → prd draft → promote → prd plan → plan run). Every command was run and output captured.

### 41.1 Log Noise Dominates User Output

Every command emits tracing INFO/WARN lines mixed with user-facing output:

```
2026-05-04T05:23:14.219Z  WARN roko_core::config::loader: config uses config version 1 (no [providers] section)
2026-05-04T05:23:14.220Z  INFO roko_agent::dispatcher: Executing agent command: "claude" [...800 char command...]
```

These are developer diagnostics, not user information. User output is buried.

**Should**: Tracing goes to `.roko/roko.log` by default. CLI output uses a separate `ProgressReporter` trait with semantic events:
- `started(task_name)`
- `progress(message)`
- `completed(summary)`
- `failed(error)`

### 41.2 "Waiting for response" Polling Loop

```
Waiting for response...
(15 seconds pass)
Waiting for response...
(15 seconds pass)
```

Text-based polling every 15s. No spinner, no elapsed time, no indication of what's happening.

**Should**: `indicatif` spinner with elapsed time and current status:
```
⠋ Waiting for claude-sonnet-4-6...  (34s)
```

### 41.3 Plan Run Output is Machine Logs, Not Human Status

Actual output during `plan run`:

```
[2026-05-04T05:30:01Z INFO  roko_cli::orchestrate] === Task 1/2: scaffold ===
[2026-05-04T05:30:01Z INFO  roko_cli::orchestrate] Dispatching agent...
[2026-05-04T05:30:35Z INFO  roko_cli::orchestrate] Agent completed.
[2026-05-04T05:30:35Z INFO  roko_cli::orchestrate] Running gate pipeline...
[2026-05-04T05:30:45Z INFO  roko_cli::orchestrate] Gate passed (rung 3).
```

This is structured logging leaking to stdout, not user UX.

**Should** (target output):
```
⟐ Running plan: temp-converter (2 tasks)

  [1/2] scaffold — Create Cargo project and main.rs
        ⠋ Implementing...  (34s)
        ✓ cargo check passed
        ✓ All gates passed (3 gates, 28s)

  [2/2] implement — Write temperature conversion logic
        ⠋ Implementing...  (42s)
        ✓ cargo test passed (7 tests)
        ✓ All gates passed (5 gates, 45s)

  ✓ Plan complete: 2/2 tasks, $0.68, 6m 5s
```

### 41.4 `prd plan` Silently Fails to Extract Tasks

Running `roko prd plan temp-converter` dispatches an agent that uses **tool calls** (write_file) instead of outputting TOML to stdout. The extraction code only looks for fenced ` ```toml ` blocks in stdout. Result:

- Agent response: "assistant requested tool use" (28 bytes)
- Extraction: finds no TOML fence → writes empty/no tasks.toml
- User feedback: NONE — command exits 0 with no output

**Root cause**: The system prompt tells the agent to produce tasks.toml, but doesn't prevent it from using write_file. When it does, the extraction pipeline can't find the content.

**Fix**: Either:
1. Strip tool-use capability from the plan-generation prompt (force text-only output)
2. Intercept write_file tool calls and extract the content from the tool arguments
3. Add validation: if no tasks.toml produced, error with "Agent did not produce plan output"

### 41.5 `plan validate` and `plan run` Have Different Parsers

Tested empirically:
- `[meta]` without `plan` field: `plan validate` **passes**, `plan run` **fails** ("missing field `plan`")
- Tasks without `role` field: `plan validate` **passes**, `plan run` **fails** ("missing field `role`")
- `[plan]` instead of `[meta]`: both fail (but different error messages)

There are TWO independent TOML parsers that disagree on the schema. Users who validate then run hit unexpected failures.

**Fix**: Single `parse_plan()` function called by both commands. Extract into `roko-orchestrator/src/plan_schema.rs`.

### 41.6 Error Messages Duplicated

Every error appears twice:

```
Error: missing field `role`
error: missing field `role`
```

One from the `anyhow` Result chain propagation, one from the explicit `eprintln!` in error handling code. The dual-print pattern exists throughout the CLI.

**Fix**: Remove explicit `eprintln!` in command handlers. Let the top-level `main()` error reporter handle all errors once. Use `report_error()` helper that checks if the error has already been printed.

### 41.7 Config Version Warning on Every Command

`roko init` generates `roko.toml` with `config_version = 2`. But running any command emits:

```
WARN roko_core::config::loader: config uses config version 1 (no [providers] section)
```

The loader's version detection counts `config_version = 2` as version 1 if there's no `[providers]` table. This is a false positive — an init'd workspace with no providers is valid, just unconfigured.

**Fix**: Remove the warning for valid empty configs. Only warn if `config_version` is literally 1 (legacy format).

### 41.8 `roko init` Output Lacks Polish

Current:
```
Created .roko/ workspace
Generated roko.toml
```

Compare to Claude CLI:
```
✓ Created .claude/settings.json
✓ Ready to go! Run 'claude' to start.
```

**Target**:
```
✓ Created .roko/ workspace
✓ Generated roko.toml (no providers configured)

Next steps:
  Set ANTHROPIC_API_KEY to use Claude, or run `roko config providers add`
```

### 41.9 `roko status` Shows Negative Cost

```
Cost: $-0.0000 total
```

Negative cost comes from either:
- Unsigned integer underflow somewhere in the accumulator
- Float precision issue with zero-usage models (Claude CLI reports 0 tokens)

Should clamp to 0.0 minimum.

### 41.10 `roko plan list` Shows Nothing Useful

Output when plans exist:
```
Plans:
  plans/
```

Just lists directory names. No task count, no completion status, no dates.

**Target**:
```
Plans:
  temp-converter  2 tasks  ✓ complete  2026-05-04
  auth-refactor   8 tasks  ⠋ 5/8 done  2026-05-03
```

### 41.11 Overall UX Design Principles — Comparison with Claude CLI and Codex

| Aspect | Claude CLI | Codex | Roko (current) |
|--------|-----------|-------|----------------|
| Spinners | `ora` (animated) | `ora` | None |
| Colors | Semantic (green=ok, red=err) | Minimal | None |
| Progress | Per-token streaming | Per-step | "Waiting for response..." |
| Icons | ✓ ✗ ⚡ | Minimal | None |
| Tracing | Hidden (--verbose flag) | Hidden | Dumps to stdout |
| Tables | `cli-table` / formatted | JSON | Hand-formatted |
| Error style | Single line + help text | Single line | Duplicated, no help |
| Default noise | Quiet | Quiet | Very noisy |

**Design principles for roko CLI output redesign**:

1. **Quiet by default** — no tracing to stdout. Use `--verbose` / `RUST_LOG` for debug.
2. **Semantic output levels**: silent < normal < verbose < debug
3. **Progressive disclosure**: task name → elapsed → result. Details only on failure.
4. **Spinner for any operation >1s** — user must always see activity.
5. **Colors for semantics only**: green=success, red=error, yellow=warning, dim=metadata.
6. **Single error reporting path** — never print errors twice.
7. **Structured final summaries**: cost, time, tasks completed, next steps.

---

## Updated Priority Matrix (S41 additions)

| Priority | Area | Effort | Section |
|---|---|---|---|
| **P0** | `prd plan` silent extraction failure | Medium | S41.4 |
| **P0** | Schema mismatch (validate vs run) | Low | S41.5 |
| **P1** | Tracing noise to stdout (quiet by default) | Medium | S41.1 |
| **P1** | Spinner for waiting states | Low | S41.2 |
| **P1** | Plan run human-readable progress | Medium | S41.3 |
| **P1** | Duplicated error messages | Low | S41.6 |
| **P1** | False positive config version warning | Low | S41.7 |
| **P1** | Negative cost display | Low | S41.9 |
| **P2** | Init output polish | Low | S41.8 |
| **P2** | Plan list useful summary | Low | S41.10 |
| **P2** | Full CLI output redesign (spinners, colors, icons) | High | S41.11 |
