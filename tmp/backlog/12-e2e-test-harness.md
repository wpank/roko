# 12 — E2E Multi-Component Integration Test Harness

**Priority**: P2 — needed for reliable CI coverage of cross-component interactions (serve + TUI + ACP)
**Size**: M (2 days)
**Crates**: New crate `tests/harness/` (roko-test-harness); touches `tests/Cargo.toml`, workspace `Cargo.toml`, `tests/tests/end_to_end.rs`
**Depends on**: None

---

## Background

Roko is a multi-crate Rust system. The `roko-serve` binary starts an HTTP control plane on port 6677; the `roko-cli` binary runs plans, agents, and the TUI. These two binaries must interoperate: the TUI connects to the serve API, agents call back to the serve routes, and ACP clients communicate through it.

The workspace already has 9,900+ unit and in-process integration tests (`cargo test --workspace`). What it lacks is a test harness that can start `roko-serve` as a real OS child process on an ephemeral port, wait for it to become healthy, run tests against it over real HTTP, and guarantee the process is killed when the test exits — even on panic.

Without this, multi-component tests must either mock HTTP (which misses real routing and middleware bugs) or require a manually started server (which is not CI-compatible). The existing `tests/tests/end_to_end.rs` does in-process testing; it doesn't spawn actual server processes.

The roko-tests crate already exists at `/Users/will/dev/nunchi/roko/roko/tests/` with its own `Cargo.toml` and `tests/` directory. This item adds a new `roko-test-harness` library crate at `tests/harness/` that the integration tests can use as a dev-dependency.

## Current State

1. The workspace `tests/` directory is at `/Users/will/dev/nunchi/roko/roko/tests/`.
2. `tests/Cargo.toml` exists and defines crate `roko-tests` (verified by reading the file).
3. `tests/tests/end_to_end.rs` exists — it wires together `FileSubstrate`, `PromptComposer`, `CompileGate`, and `EpisodePolicy` in-process, without spawning any external binaries.
4. `tests/tests/tool_equivalence.rs` and `tests/tests/tool_replay.rs` also exist.
5. No `tests/harness/` directory or `roko-test-harness` crate exists yet.
6. The workspace `Cargo.toml` at `/Users/will/dev/nunchi/roko/roko/Cargo.toml` has a `[workspace] members = [...]` block that needs a new entry.
7. The serve health endpoint is at `GET /health` (unauthenticated), implemented in `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` at line 398 and 452.

## Implementation Plan

### Step 1: Create the `tests/harness/` directory and `Cargo.toml`

Create `/Users/will/dev/nunchi/roko/roko/tests/harness/Cargo.toml`:

```toml
[package]
name    = "roko-test-harness"
version = "0.1.0"
edition = "2021"
publish = false

[lints]
workspace = true

[dependencies]
tokio      = { workspace = true, features = ["full"] }
reqwest    = { version = "0.12", features = ["json"] }
serde      = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tracing    = { workspace = true }
```

### Step 2: Add to workspace members

In `/Users/will/dev/nunchi/roko/roko/Cargo.toml`, find the `[workspace] members = [...]` list and add `"tests/harness"` alongside the existing `"tests"` entry.

### Step 3: Create `tests/harness/src/lib.rs`

Create `/Users/will/dev/nunchi/roko/roko/tests/harness/src/lib.rs`:

```rust
//! Test harness for multi-component roko integration tests.
//!
//! Spawns roko-serve (and optionally other binaries) as real OS child processes
//! on ephemeral ports. All processes are killed when `RokoTestHarness` is dropped.

pub mod health;
pub mod port;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;

pub use health::{HealthReport, health_check};
pub use port::free_port;

/// Configuration for spawning `roko-serve`.
#[derive(Debug, Default)]
pub struct ServeConfig {
    /// Path to roko.toml for the test workspace. Defaults to a temp dir fixture.
    pub config_path: Option<PathBuf>,
    /// Environment variables to set (e.g. ROKO_SERVE_PORT, API keys).
    pub env: HashMap<String, String>,
    /// Maximum seconds to wait for the health check. Default: 30.
    pub health_timeout_secs: u64,
}

impl ServeConfig {
    /// Create a default config with a 30-second health timeout.
    pub fn new() -> Self {
        Self {
            health_timeout_secs: 30,
            ..Default::default()
        }
    }
}

/// Handle for a spawned `roko-serve` process.
#[derive(Debug)]
pub struct ServerHandle {
    pub port: u16,
    /// Base URL e.g. "http://127.0.0.1:60123"
    pub base_url: String,
}

/// Top-level harness. Owns all spawned child processes.
/// All processes are killed when this value is dropped.
pub struct RokoTestHarness {
    processes: Vec<Child>,
}

impl RokoTestHarness {
    /// Create a new harness with no running processes.
    pub fn new() -> Self {
        Self { processes: Vec::new() }
    }

    /// Spawn `roko-serve` on a random free port.
    ///
    /// Blocks (async) until `GET /health` returns HTTP 200 or
    /// `config.health_timeout_secs` elapses.
    ///
    /// # Errors
    ///
    /// Returns an error if the binary cannot be found, fails to start,
    /// or the health check times out.
    pub async fn spawn_serve(
        &mut self,
        config: ServeConfig,
    ) -> anyhow::Result<ServerHandle> {
        let port = free_port()?;
        let base_url = format!("http://127.0.0.1:{port}");

        // CARGO_BIN_EXE_roko is set by Cargo's test runner when the binary is
        // declared as a dev-dependency. Fall back to searching PATH.
        let binary = std::env::var("CARGO_BIN_EXE_roko")
            .unwrap_or_else(|_| "roko".to_string());

        let timeout = if config.health_timeout_secs == 0 { 30 } else { config.health_timeout_secs };

        let mut cmd = tokio::process::Command::new(&binary);
        cmd.args(["serve"])
            .env("ROKO_SERVE_PORT", port.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(config_path) = &config.config_path {
            cmd.arg("--config").arg(config_path);
        }
        for (key, val) in &config.env {
            cmd.env(key, val);
        }

        let child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!("failed to spawn roko-serve (binary={binary:?}): {e}")
        })?;

        // Safety: convert tokio Child to std Child for storage in self.processes.
        // We only need to kill it on drop; we don't need async I/O on it.
        let std_child = child.into_std()?;
        self.processes.push(std_child);

        let health_url = format!("{base_url}/health");
        let report = health_check(&health_url, timeout).await;
        if !report.ok {
            anyhow::bail!(
                "roko-serve health check failed after {timeout}s at {health_url}: \
                 status={:?}",
                report.status_code
            );
        }

        tracing::info!(port, "roko-serve is healthy");
        Ok(ServerHandle { port, base_url })
    }
}

impl Default for RokoTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RokoTestHarness {
    fn drop(&mut self) {
        for proc in &mut self.processes {
            let _ = proc.kill();
        }
    }
}
```

### Step 4: Create `tests/harness/src/port.rs`

Create `/Users/will/dev/nunchi/roko/roko/tests/harness/src/port.rs`:

```rust
//! Ephemeral port allocation for test processes.

use std::net::TcpListener;

/// Bind to `127.0.0.1:0` to get an OS-assigned free port, then close
/// the listener and return the port number.
///
/// There is a small TOCTOU window between closing the listener and the child
/// process binding. In practice this is negligible on loopback.
///
/// # Errors
///
/// Returns an error if the OS cannot allocate a port.
pub fn free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
```

### Step 5: Create `tests/harness/src/health.rs`

Create `/Users/will/dev/nunchi/roko/roko/tests/harness/src/health.rs`:

```rust
//! Health check polling for test harness.

use std::time::{Duration, Instant};

/// Result of a health check attempt.
#[derive(Debug)]
pub struct HealthReport {
    pub ok: bool,
    pub status_code: Option<u16>,
    pub elapsed_ms: u64,
}

/// Poll `url` until it returns HTTP 200 or `timeout_secs` elapses.
///
/// Retries every 200ms. Returns immediately on success.
pub async fn health_check(url: &str, timeout_secs: u64) -> HealthReport {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let start = Instant::now();
    loop {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return HealthReport {
                    ok: true,
                    status_code: Some(resp.status().as_u16()),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                };
            }
            Ok(resp) => {
                tracing::debug!(
                    url,
                    status = resp.status().as_u16(),
                    "health check not yet passing"
                );
            }
            Err(e) => {
                tracing::debug!(url, error = %e, "health check connection refused");
            }
        }

        if Instant::now() >= deadline {
            return HealthReport {
                ok: false,
                status_code: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
```

### Step 6: Add `roko-test-harness` as a dev-dependency in `tests/Cargo.toml`

Edit `/Users/will/dev/nunchi/roko/roko/tests/Cargo.toml`, adding to the `[dev-dependencies]` block:

```toml
roko-test-harness = { path = "harness" }
anyhow = { workspace = true }
```

### Step 7: Add a usage example to `tests/tests/end_to_end.rs`

Add this test at the bottom of `/Users/will/dev/nunchi/roko/roko/tests/tests/end_to_end.rs`:

```rust
/// Smoke test: spawn roko-serve as a real process and verify /health returns 200.
///
/// This test is marked #[ignore] by default so it does not run in every `cargo
/// test` invocation (requires a built binary). Run explicitly with:
///   cargo test -p roko-tests -- serve_health_smoke --ignored --nocapture
#[tokio::test]
#[ignore]
async fn serve_health_smoke() {
    use roko_test_harness::{RokoTestHarness, ServeConfig};

    let mut harness = RokoTestHarness::new();
    let serve = harness
        .spawn_serve(ServeConfig::new())
        .await
        .expect("roko-serve should start and become healthy");

    let url = format!("{}/health", serve.base_url);
    let report = roko_test_harness::health_check(&url, 5).await;
    assert!(
        report.ok,
        "health check at {url} failed after serve was already healthy: {report:?}"
    );
    // harness dropped here — serve process is killed automatically
}
```

## Acceptance Criteria

1. `cargo build -p roko-test-harness` compiles without errors.
2. `cargo test -p roko-test-harness` passes (the harness crate itself has no tests initially, so this just checks compilation).
3. `RokoTestHarness::new()` can be created and dropped without panicking, even if no processes were spawned.
4. `free_port()` returns a port number > 1024 and the port is not in use (verifiable by attempting a bind at the returned port immediately after the call).
5. When `serve_health_smoke` is run with `--ignored` after building the release binary, `roko-serve` starts on an ephemeral port, `/health` returns 200, and the process is terminated when the harness is dropped.
6. After the harness is dropped, the port is no longer listening (no process holds it).
7. Two parallel runs of `serve_health_smoke` do not conflict: each receives a distinct OS-assigned port from `free_port()`.

## Verification Checklist

- [ ] Run `cargo build -p roko-test-harness` from the workspace root — should compile clean
- [ ] Run `cargo clippy -p roko-test-harness --no-deps -- -D warnings` — should pass with no warnings
- [ ] Run `cargo test --workspace` — should still pass (new crate adds no failing tests)
- [ ] Build the roko binary: `cargo build -p roko-cli --release`
- [ ] Set `CARGO_BIN_EXE_roko=$(pwd)/target/release/roko` in the shell
- [ ] Run `cargo test -p roko-tests -- serve_health_smoke --ignored --nocapture`
- [ ] Confirm output shows "roko-serve is healthy" log line and the test passes
- [ ] After the test exits, run `lsof -i :<port>` (using the port printed in logs) — should return no results

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/tests/harness/Cargo.toml` | Create new file |
| `/Users/will/dev/nunchi/roko/roko/tests/harness/src/lib.rs` | Create new file |
| `/Users/will/dev/nunchi/roko/roko/tests/harness/src/port.rs` | Create new file |
| `/Users/will/dev/nunchi/roko/roko/tests/harness/src/health.rs` | Create new file |
| `/Users/will/dev/nunchi/roko/roko/Cargo.toml` | Add `"tests/harness"` to `[workspace] members` |
| `/Users/will/dev/nunchi/roko/roko/tests/Cargo.toml` | Add `roko-test-harness` and `anyhow` to `[dev-dependencies]` |
| `/Users/will/dev/nunchi/roko/roko/tests/tests/end_to_end.rs` | Add `serve_health_smoke` integration test |
