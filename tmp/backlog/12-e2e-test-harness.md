# Backlog: E2E Multi-Component Integration Test Harness

**Status**: Backlog
**Priority**: P2
**Size**: M (2 days)
**Origin**: `tmp/architecture-archive/21-tui-and-operations.md` (Section 5: E2E test harness)

---

## Problem Statement

The workspace has strong unit and integration test coverage within individual crates (`cargo test --workspace` passes 9,900+ tests) but lacks a multi-component integration harness that can start `roko-serve` and `mirage-rs` as real child processes on ephemeral ports, verify their health, exercise cross-component interactions, and guarantee cleanup on test exit.

Without this, integration tests for HTTP API workflows, SSE streaming, ACP passthrough, gate dispatch, or relay interactions must either:
- Mock the HTTP layer entirely (misses real routing bugs and middleware interactions), or
- Require manual setup of a running server (not suitable for CI), or
- Shell out with fragile bash scripts that leak processes on panic.

The predecessor system had `bardo/tests/harness/src/lib.rs` exposing `BardoTestHarness`, `HealthReport`, and `TerminalProbe`. This spec defines the roko equivalent, adapted to the current crate structure.

The existing test files in `crates/roko-serve/tests/api_integration.rs` and `tests/tests/end_to_end.rs` already follow the pattern of needing a live server; they currently use `reqwest` directly but spin up the server inline (not as a proper child process with port allocation). The harness consolidates this infrastructure.

---

## Proposed Solution

### Workspace location

New crate: `tests/harness/` — a library crate added to the workspace as a `dev-dependency` for integration test crates. It is not published.

```toml
# tests/harness/Cargo.toml
[package]
name    = "roko-test-harness"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
tokio       = { version = "1", features = ["full"] }
reqwest     = { version = "0.12", features = ["json"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
tracing     = "0.1"
```

### Core API

```rust
/// Top-level harness. Owns all spawned child processes.
/// All processes are killed when this value is dropped.
pub struct RokoTestHarness {
    processes: Vec<Child>,
    serve_handle:  Option<ServerHandle>,
    mirage_handle: Option<MirageHandle>,
}

pub struct ServerHandle {
    pub port:    u16,
    pub base_url: String,  // "http://127.0.0.1:{port}"
}

pub struct MirageHandle {
    pub port:    u16,
    pub base_url: String,
}

pub struct HealthReport {
    pub ok:            bool,
    pub status_code:   u16,
    pub response_body: serde_json::Value,
    pub elapsed_ms:    u64,
}

impl RokoTestHarness {
    /// Create a new harness with no running processes.
    pub fn new() -> Self;

    /// Spawn roko-serve on a random free port.
    /// Blocks until /api/health returns 200 or timeout_secs elapses.
    pub async fn spawn_serve(
        &mut self,
        config: ServeConfig,
    ) -> anyhow::Result<ServerHandle>;

    /// Spawn mirage-rs on a random free port.
    /// Blocks until /health returns 200 or timeout_secs elapses.
    pub async fn spawn_mirage(
        &mut self,
        config: MirageConfig,
    ) -> anyhow::Result<MirageHandle>;

    /// Poll a URL until it returns HTTP 200 or timeout expires.
    pub async fn health_check(
        url: &str,
        timeout_secs: u64,
    ) -> HealthReport;
}

impl Drop for RokoTestHarness {
    fn drop(&mut self) {
        // Kill all child processes. SIGKILL on Unix, TerminateProcess on Windows.
        for proc in &mut self.processes {
            let _ = proc.kill();
        }
    }
}
```

### `ServeConfig` and `MirageConfig`

```rust
pub struct ServeConfig {
    /// Path to roko.toml for the test workspace. Defaults to a temp dir fixture.
    pub config_path: Option<PathBuf>,
    /// Environment variables to set (e.g., ROKO_SERVE_PORT, provider keys).
    pub env: HashMap<String, String>,
    /// Maximum seconds to wait for the health check. Default: 30.
    pub health_timeout_secs: u64,
}

pub struct MirageConfig {
    pub env: HashMap<String, String>,
    pub health_timeout_secs: u64,
}
```

### Port allocation

Use `std::net::TcpListener::bind("127.0.0.1:0")` to get an OS-assigned free port, record it, close the listener, then pass the port to the child process via environment variable. This avoids port collisions between parallel test runs.

### Process spawning

Use `tokio::process::Command` with `stdout(Stdio::piped())` and `stderr(Stdio::piped())` so that test output captures server logs on failure. Binary paths are resolved via `CARGO_BIN_EXE_roko` (for roko-cli) and `CARGO_BIN_EXE_mirage-rs` (for mirage-rs) — these environment variables are set by Cargo's test runner when the binaries are listed as `[[bin]]` dev-dependencies.

### Health check retry loop

```rust
pub async fn health_check(url: &str, timeout_secs: u64) -> HealthReport {
    let deadline = tokio::time::Instant::now()
        + tokio::time::Duration::from_secs(timeout_secs);
    let client = reqwest::Client::new();
    loop {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return HealthReport { ok: true, ... };
            }
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            return HealthReport { ok: false, ... };
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
```

### Usage in integration tests

```rust
// tests/tests/end_to_end.rs
#[tokio::test]
async fn serve_health_check_passes() {
    let mut harness = RokoTestHarness::new();
    let serve = harness.spawn_serve(ServeConfig::default()).await.unwrap();

    let report = RokoTestHarness::health_check(
        &format!("{}/api/health", serve.base_url),
        30,
    ).await;

    assert!(report.ok, "serve health check failed: {:?}", report);
    // harness dropped here — serve process killed
}
```

---

## Implementation Location

| Component | Path |
|---|---|
| Harness crate | `tests/harness/` (new directory + `Cargo.toml`) |
| Core types | `tests/harness/src/lib.rs` |
| Port allocation util | `tests/harness/src/port.rs` |
| Health check impl | `tests/harness/src/health.rs` |
| Workspace entry | `Cargo.toml` (add to `[workspace] members`) |
| Usage example | `tests/tests/end_to_end.rs` (extend existing) |

---

## Acceptance Criteria

1. `RokoTestHarness::new()` compiles without errors and `cargo test -p roko-test-harness` passes.

2. `spawn_serve()` successfully starts `roko-serve` on a random port and the health check at `/api/health` returns HTTP 200 within 30 seconds in a standard CI environment.

3. Dropping `RokoTestHarness` kills all child processes: after the drop, `lsof -i :{port}` returns no entries for the allocated port. Verified by a test that records the PID and polls for process exit after drop.

4. A basic integration test in `tests/tests/end_to_end.rs` that uses the harness (spawn → health check → stop) passes in `cargo test --workspace`.

5. Parallel test runs on the same machine do not conflict: each test instance receives a distinct OS-assigned port.

---

## References

- Source spec: `/Users/will/dev/nunchi/roko/roko/tmp/architecture-archive/21-tui-and-operations.md` (Section 5)
- Predecessor reference: `bardo/tests/harness/src/lib.rs` (`BardoTestHarness`, not in this repo)
- Existing integration tests: `/Users/will/dev/nunchi/roko/roko/tests/tests/end_to_end.rs`
- Serve routes (health endpoint): `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status/health.rs`
- mirage-rs: `/Users/will/dev/nunchi/roko/roko/apps/mirage-rs/`
