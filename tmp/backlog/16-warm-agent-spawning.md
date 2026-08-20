# 16 — Warm Agent Spawning

**Priority**: P3 — performance optimization; not blocking correctness
**Size**: M (2–3 days)
**Crates**: `crates/roko-cli/` (dispatch submodule), `crates/roko-agent/`
**Depends on**: None

---

## Background

Roko is a Rust toolkit that executes implementation plans by dispatching tasks to LLM
agents (Claude CLI, Anthropic API, Gemini, etc.), running validation gates, and persisting
results. The plan runner (`roko plan run`) sequences tasks through a lifecycle of phases:
Strategist -> Implementer -> Gate -> Reviewer -> Merge.

Every phase transition involves spawning a new agent: the runner picks a provider, forks
a subprocess (for CLI providers) or opens an HTTP connection (for API providers), performs
an MCP protocol handshake, and waits for the first token from the model. On real hardware,
this cold-start sequence takes 5–15 seconds. For a 10-task plan where each task runs
Implementer -> Gate -> Reviewer, that is 30 cold starts — potentially 5 minutes of pure
idle waiting between tasks, with no LLM work happening.

The warm pool is a pre-spawn optimization: while the gate pipeline is running (which can
take 10–30 seconds for compile, test, and clippy gates), the runner should simultaneously
boot the next-phase agent so it is ready when the gate completes. The data structures and
container semantics for this are already fully implemented and tested. What is missing is
the actual background subprocess spawn.

## Current State

1. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs`** (250 lines) — Contains `WarmPool`, `WarmAgent`, and `WarmPoolStats`. The pool is a per-role LRU container (`HashMap<String, VecDeque<WarmAgent>>`) with TTL eviction, capacity enforcement, `insert`, `take`, and `evict_expired` methods. All 6 container tests pass. `WarmAgent` stores only metadata (`id: String`, `model: String`, `spawned_at: Instant`, `ttl: Duration`) — it does not store a live process handle. The module's own doc comment explicitly states: "This implementation is a *typed*, fully tested LRU container — it does *not* yet pre-spawn real agents."

2. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 10801** — The pre-spawn block runs during gate dispatch. It creates a `WarmAgent` placeholder and inserts it into the pool. The comment at line 10804 reads: "We register a placeholder WarmAgent — real provider spawn happens in the dispatcher when `take()` is called and the slot is promoted." No actual subprocess is forked here.

3. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 4116** — The promotion block runs when a gate passes. It calls `factory.dispatcher().warm_pool().take(next_role)` and logs "warm_pool: promoted pre-spawned agent for next phase" — but the `WarmAgent` it receives has no live handle. The promoted struct is logged and then discarded. The dispatcher subsequently cold-spawns from scratch regardless.

4. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 4395** — The eviction block runs on gate failure. It calls `warm_pool().evict_expired()` which drops `WarmAgent` structs — but since those structs hold no live process, no actual process cleanup is needed yet.

5. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs`** — `Dispatcher` struct (line 137) owns a `warm_pool: WarmPool` field. `warm_pool()` accessor at line 173 returns `&WarmPool`. No `LiveAgentHandle` trait exists anywhere in this crate.

6. **`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/factory.rs` line 135** — `SharedAgentFactory` is constructed with `WarmPool::new(2)` (2 slots per role). The cap is already set; only the actual spawn is missing.

7. **`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli.rs`** (476 lines) — The `ClaudeCliAdapter` creates a `ClaudeCliAgent` by resolving command, working directory, timeout, tools, system prompt, and resource limits — then returns the agent. This is what a real warm spawn needs to invoke.

## Implementation Plan

### Step 1: Define the `LiveAgentHandle` trait

Add a new file `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/live_handle.rs`:

```rust
//! Live agent handles stored in the warm pool.
//!
//! A `LiveAgentHandle` wraps a fully-initialized provider connection. The warm
//! pool stores these handles indexed by `WarmAgent.id` so the dispatcher can
//! hand one off instead of cold-spawning.

use anyhow::Result;

/// A live, initialized provider connection ready to accept work.
pub trait LiveAgentHandle: Send + Sync {
    /// Returns a human-readable label (e.g. "claude-cli:pid=12345").
    fn label(&self) -> &str;
    /// Shut down the underlying process or connection gracefully.
    fn shutdown(&mut self);
}

/// A live handle wrapping a tokio child process.
pub struct CliLiveHandle {
    pub label: String,
    pub child: tokio::process::Child,
}

impl LiveAgentHandle for CliLiveHandle {
    fn label(&self) -> &str {
        &self.label
    }
    fn shutdown(&mut self) {
        let _ = self.child.start_kill();
    }
}
```

### Step 2: Add `WarmHandleRegistry` to the dispatch module

Add a new file `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_handle_registry.rs`:

```rust
//! Registry mapping WarmAgent ids to live provider handles.

use std::collections::HashMap;
use std::sync::Mutex;
use super::live_handle::LiveAgentHandle;

#[derive(Default)]
pub struct WarmHandleRegistry {
    inner: Mutex<HashMap<String, Box<dyn LiveAgentHandle>>>,
}

impl WarmHandleRegistry {
    pub fn insert(&self, id: String, handle: Box<dyn LiveAgentHandle>) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(id, handle);
    }

    /// Remove and return a live handle by WarmAgent id.
    pub fn take(&self, id: &str) -> Option<Box<dyn LiveAgentHandle>> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(id)
    }

    /// Shut down and remove all handles for the given ids.
    pub fn shutdown_ids(&self, ids: &[String]) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for id in ids {
            if let Some(mut handle) = guard.remove(id) {
                handle.shutdown();
            }
        }
    }
}
```

### Step 3: Add both modules to `dispatch/mod.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs`, add:

```rust
pub mod live_handle;
pub mod warm_handle_registry;

pub use live_handle::LiveAgentHandle;
pub use warm_handle_registry::WarmHandleRegistry;
```

Add `warm_handle_registry: WarmHandleRegistry` to the `Dispatcher` struct and initialize it in `Dispatcher::new`. Add a `warm_handle_registry()` accessor returning `&WarmHandleRegistry`.

### Step 4: Replace the placeholder spawn with a real background spawn

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, the pre-spawn block starts at line 10801. Replace the placeholder `WarmAgent` insertion block with:

```rust
// WarmPool: background-spawn the next-phase agent while the gate runs.
{
    let current_role = task_def
        .and_then(|td| td.role.as_deref())
        .unwrap_or("implementer");
    let next_role = next_warm_role(current_role);
    let warm_id = format!("{plan_id}:{task_id}:warm:{next_role}");
    let warm_agent = crate::dispatch::warm_pool::WarmAgent {
        id: warm_id.clone(),
        model: ctx.config.model.clone(),
        spawned_at: Instant::now(),
        ttl: Duration::from_secs(300),
    };
    if let Some(evicted) = ctx.factory.dispatcher().warm_pool().insert(next_role, warm_agent) {
        // Shut down the evicted handle if one exists.
        ctx.factory.dispatcher().warm_handle_registry().shutdown_ids(&[evicted.id]);
        debug!(evicted_id = %evicted.id, "warm_pool: evicted overflow agent on pre-spawn");
    }
    // Spawn the real provider process in the background. Gate runs concurrently.
    let registry = ctx.factory.dispatcher().warm_handle_registry_arc(); // Arc clone
    let config_clone = ctx.config.clone();
    let workdir_clone = ctx.config.workdir.clone();
    tokio::spawn(async move {
        match spawn_warm_cli_handle(&config_clone, &workdir_clone, next_role, &warm_id).await {
            Ok(handle) => {
                registry.insert(warm_id.clone(), handle);
                debug!(warm_agent_id = %warm_id, "warm_pool: live handle stored");
            }
            Err(e) => {
                debug!(error = %e, warm_agent_id = %warm_id,
                    "warm_pool: background spawn failed, will cold-spawn on promote");
            }
        }
    });
}
```

Add the `spawn_warm_cli_handle` helper function to `event_loop.rs` (or extract to `dispatch_model.rs` if that module has been created):

```rust
async fn spawn_warm_cli_handle(
    config: &RunConfig,
    workdir: &Path,
    role: &str,
    warm_id: &str,
) -> Result<Box<dyn crate::dispatch::LiveAgentHandle>> {
    use tokio::process::Command;
    // Resolve provider command from config. This mirrors what cold-spawn does.
    let command = config.model.clone(); // simplified; use actual provider resolution
    let mut cmd = Command::new(&command);
    cmd.arg("--no-interactive") // provider-specific idle flag
        .current_dir(workdir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = cmd.spawn()?;
    let label = format!("{role}:pid={}", child.id().unwrap_or(0));
    Ok(Box::new(crate::dispatch::live_handle::CliLiveHandle {
        label: label.clone(),
        child,
    }))
}
```

Note: The exact provider command resolution should mirror the logic in `crates/roko-agent/src/provider/claude_cli.rs` — read the `provider.command` field from `RunConfig` rather than using the model slug directly.

### Step 5: Use the live handle on promotion

In event_loop.rs at the promotion block (line 4127), after `take()` returns a `WarmAgent`, look up the live handle:

```rust
let promoted = factory.dispatcher().warm_pool().take(next_role);
if let Some(warm) = promoted {
    let handle = factory.dispatcher().warm_handle_registry().take(&warm.id);
    if let Some(_live) = handle {
        debug!(warm_agent_id = %warm.id, role = next_role, "warm_pool: promoted live handle");
        // Pass live handle to dispatcher. For now: log and let dispatcher reuse pid.
        // Full dispatcher integration is a follow-up; this step verifies the plumbing.
    } else {
        debug!(warm_agent_id = %warm.id, "warm_pool: handle not ready, cold-spawning");
    }
} else {
    debug!(role = next_role, "warm_pool: no warm agent, cold-spawning");
}
```

### Step 6: Shut down handles on eviction

In event_loop.rs at the eviction block (line 4398), call `shutdown_ids` on evicted handle ids:

```rust
let evicted = factory.dispatcher().warm_pool().evict_expired();
if !evicted.is_empty() {
    let evicted_ids: Vec<String> = evicted.iter().map(|a| a.id.clone()).collect();
    factory.dispatcher().warm_handle_registry().shutdown_ids(&evicted_ids);
    debug!(count = evicted.len(), "warm_pool: evicted stale agents and shut down handles");
}
```

### Step 7: Add a `live_handles` field to `WarmPoolStats`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs`, add to `WarmPoolStats`:

```rust
pub struct WarmPoolStats {
    pub size: usize,
    pub roles_with_warm_agents: usize,
    pub max_per_role: usize,
    pub live_handles: usize,  // Add this field
}
```

Populate `live_handles` in `WarmPool::stats()` by accepting the count from `WarmHandleRegistry::len()` as a parameter, or by querying a shared registry.

## Acceptance Criteria

1. When a gate starts, a `tokio::spawn` initiates a real provider subprocess for the next-phase role and (on success) stores a live handle in the registry. A `debug!("warm_pool: live handle stored ...")` log line is emitted.

2. When a gate passes, `warm_handle_registry().take(warm_id)` returns a live handle (not `None`) in the happy path. The log line "warm_pool: promoted live handle" appears in debug output during integration tests.

3. If the background spawn fails or is still in flight when promotion is attempted, the runner falls through to standard cold-spawn without error. No panic, no user-visible regression.

4. When agents are evicted (gate failure or TTL expiry), `handle.shutdown()` is called on each evicted handle. After a plan run that exercises the gate-failure path, `ps aux` shows no leaked Claude CLI subprocesses.

5. All 6 existing `WarmPool` container tests in `warm_pool.rs` continue to pass without modification.

6. `cargo test -p roko-cli` passes.

## Verification Checklist

- [ ] Run `cargo build -p roko-cli` — zero errors
- [ ] Run `cargo test -p roko-cli -- warm_pool` — all 6 container tests pass
- [ ] Run `cargo clippy -p roko-cli --no-deps -- -D warnings` — clean
- [ ] Run a plan with `RUST_LOG=debug cargo run -p roko-cli -- plan run plans/ --engine runner-v2` and check logs for "warm_pool: live handle stored"
- [ ] Verify that on gate failure the log shows "warm_pool: evicted stale agents and shut down handles"
- [ ] Verify that on gate pass the log shows either "warm_pool: promoted live handle" or "warm_pool: handle not ready, cold-spawning" (fallback path)
- [ ] Check `ps aux | grep claude` before and after a plan run to confirm no leaked subprocesses

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/live_handle.rs` | New file: `LiveAgentHandle` trait + `CliLiveHandle` struct |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_handle_registry.rs` | New file: `WarmHandleRegistry` — HashMap of live handles by id |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs` | Declare new modules; add `warm_handle_registry: WarmHandleRegistry` to `Dispatcher`; add Arc accessor |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs` | Add `live_handles: usize` to `WarmPoolStats` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 10801 | Replace placeholder insert with `tokio::spawn` that calls `spawn_warm_cli_handle` and stores result in registry |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 4127 | Promote: look up handle by id in registry; log whether live or fallback |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` line 4398 | Evict: call `warm_handle_registry().shutdown_ids(evicted_ids)` |
