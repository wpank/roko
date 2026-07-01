# B4: Server state persistence across restarts

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` — all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` — always present, no Option wrapping

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## What this task does

Persist selected server state to disk so that `roko serve` can resume after a restart. The server currently loses all in-memory state (discovered agents, deployments, template run records) on shutdown. This task adds a snapshot file at `.roko/state/server-state.json` with:

- Atomic writes (write to `.tmp`, then rename)
- Auto-save every 30 seconds using the cancellation token
- Restore on startup with graceful handling of missing or corrupt snapshots
- A final save on graceful shutdown

---

## Critical notes about AppState

Read the full `AppState` struct at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` before making any changes.

Key points:
- `AppState` fields use `tokio::sync::RwLock` (NOT `std::sync::RwLock`) — all locks are async: `.read().await`, `.write().await`
- Some fields use `parking_lot::Mutex` (e.g., `affect_engine`) — these are sync: `.lock()`
- Some fields are not serializable (handles, `Arc<dyn>`, channels) — skip them entirely in the snapshot
- The snapshot includes only data worth restoring. Ephemeral state (active runs, handles) starts fresh on every restart.

---

## Fields worth persisting

| Field | Type | Persist? | Reason |
|-------|------|----------|--------|
| `discovered_agents` | `RwLock<HashMap<String, DiscoveredAgent>>` | Yes | Agent registry survives restarts |
| `template_runs` | `RwLock<HashMap<String, Vec<TemplateRunRecord>>>` | Yes | Run history |
| `deployments` | `RwLock<HashMap<String, Deployment>>` | Conditional | Only if `Deployment` derives `Serialize` — check first |
| `active_runs` | `RwLock<HashMap<String, RunHandle>>` | No | Contains `JoinHandle`, not serializable |
| `active_plans` | `RwLock<HashMap<String, PlanHandle>>` | No | Contains `JoinHandle` |
| `operations` | `RwLock<HashMap<String, OperationHandle>>` | No | Contains `JoinHandle` |
| `event_bus` | `EventBus<ServerEvent>` | No | Ephemeral |
| `supervisor` | `Arc<ProcessSupervisor>` | No | Runtime only |

---

## Steps

### Step 1 — Read AppState and related types

Before writing any code, read:

```
/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs
```

Also check whether `Deployment` derives `Serialize + Deserialize`:
```bash
grep -n "struct Deployment\|#\[derive" crates/roko-serve/src/deploy.rs 2>/dev/null \
  || grep -rn "struct Deployment" crates/roko-serve/src/ --include='*.rs'
```

If `Deployment` does not derive serde traits, exclude it from the snapshot and leave a comment marking it as a future wire-up.

### Step 2 — Add snapshot struct and methods to state.rs

After the `impl AppState` block (search for its closing `}`), add:

```rust
// ---------------------------------------------------------------------------
// Server state persistence
// ---------------------------------------------------------------------------

/// Serializable subset of [`AppState`] persisted across server restarts.
///
/// Only fields that are both (a) worth restoring and (b) serializable are
/// included. Ephemeral state (active runs, handles, channels) is always
/// initialized fresh on startup.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerStateSnapshot {
    /// When the snapshot was written.
    pub saved_at: chrono::DateTime<chrono::Utc>,
    /// Discovered agent registry.
    #[serde(default)]
    pub discovered_agents: HashMap<String, DiscoveredAgent>,
    /// Template run history.
    #[serde(default)]
    pub template_runs: HashMap<String, Vec<TemplateRunRecord>>,
    // NOTE: `deployments` omitted until `Deployment` derives Serialize.
}

impl AppState {
    /// Absolute path to the server state snapshot file.
    fn snapshot_path(&self) -> PathBuf {
        self.layout.root().join("state").join("server-state.json")
    }

    /// Atomically capture current state to the snapshot file.
    ///
    /// Writes to a `.tmp` file first, then renames into place. A crash
    /// during write cannot corrupt the existing snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, the file cannot
    /// be written, or serialization fails. Callers should log and continue
    /// rather than treating this as fatal.
    pub async fn save_snapshot(&self) -> anyhow::Result<()> {
        let snapshot = ServerStateSnapshot {
            saved_at: chrono::Utc::now(),
            discovered_agents: self.discovered_agents.read().await.clone(),
            template_runs: self.template_runs.read().await.clone(),
        };

        let path = self.snapshot_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(&snapshot)?;
        let tmp = path.with_extension("json.tmp");
        tokio::fs::write(&tmp, &json).await?;
        tokio::fs::rename(&tmp, &path).await?;

        tracing::debug!(path = %path.display(), "server state snapshot saved");
        Ok(())
    }

    /// Restore state from a previous snapshot, if one exists.
    ///
    /// - Returns `Ok(true)` if a snapshot was found and restored.
    /// - Returns `Ok(false)` if no snapshot file exists.
    /// - Returns `Err` only if the file exists but cannot be read (e.g., I/O error).
    ///
    /// A corrupt snapshot (invalid JSON) is treated as if no snapshot exists:
    /// a warning is logged and `Ok(false)` is returned so the server starts
    /// cleanly.
    ///
    /// Restored data is merged into the current state using
    /// `entry().or_insert()`, so any state already present (from a racing
    /// in-flight write) is not overwritten.
    pub async fn restore_snapshot(&self) -> anyhow::Result<bool> {
        let path = self.snapshot_path();
        if !path.exists() {
            tracing::debug!(path = %path.display(), "no server state snapshot found — starting fresh");
            return Ok(false);
        }

        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "failed to read server state snapshot — starting fresh"
                );
                return Ok(false);
            }
        };

        let snapshot: ServerStateSnapshot = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "server state snapshot is corrupt — starting fresh"
                );
                return Ok(false);
            }
        };

        // Merge discovered agents (do not overwrite in-flight registrations).
        {
            let mut agents = self.discovered_agents.write().await;
            for (id, agent) in snapshot.discovered_agents {
                agents.entry(id).or_insert(agent);
            }
        }

        // Merge template runs.
        {
            let mut runs = self.template_runs.write().await;
            for (name, records) in snapshot.template_runs {
                runs.entry(name).or_insert(records);
            }
        }

        tracing::info!(
            path = %path.display(),
            saved_at = %snapshot.saved_at,
            "server state snapshot restored"
        );
        Ok(true)
    }
}
```

### Step 3 — Add the auto-save background task

After the `impl AppState` block that contains `save_snapshot` and `restore_snapshot`, add the standalone function:

```rust
/// Spawn a background task that saves server state on every `interval`.
///
/// The task respects the cancellation token on `state.cancel`:
/// - On each tick, if the token is cancelled, save one final snapshot then exit.
/// - If the periodic save fails, log a warning and continue — do not exit.
///
/// Returns the [`JoinHandle`] so the caller can await it during shutdown.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use std::time::Duration;
/// use roko_serve::state::{AppState, spawn_auto_save};
///
/// async fn start(state: Arc<AppState>) {
///     let _handle = spawn_auto_save(Arc::clone(&state), Duration::from_secs(30));
///     // Server runs …
///     // On shutdown, the handle is dropped; the final save happens in AppState::shutdown().
/// }
/// ```
pub fn spawn_auto_save(
    state: Arc<AppState>,
    interval: std::time::Duration,
) -> tokio::task::JoinHandle<()> {
    let cancel = state.cancel.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip missed ticks rather than rushing to catch up. If the server
        // is under load and a save takes longer than `interval`, the next
        // save is simply delayed to the next scheduled tick.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = state.save_snapshot().await {
                        tracing::warn!(error = %e, "periodic snapshot save failed");
                    }
                }
                _ = cancel.cancelled() => {
                    // Cancellation: perform a final save before exiting.
                    if let Err(e) = state.save_snapshot().await {
                        tracing::warn!(error = %e, "final snapshot save on shutdown failed");
                    }
                    break;
                }
            }
        }
    })
}
```

### Step 4 — Save on graceful shutdown

Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` and find `AppState::shutdown()`. Update it to save before cancelling:

```rust
pub async fn shutdown(&self) {
    tracing::info!("server shutdown initiated");

    // Persist state before cancelling so the auto-save task also exits cleanly.
    if let Err(e) = self.save_snapshot().await {
        tracing::warn!(error = %e, "snapshot save on shutdown failed");
    }

    self.cancel.cancel();
    self.supervisor.shutdown_all().await;
    self.event_bus.publish(ServerEvent::ServerShutdown);
}
```

### Step 5 — Wire restore and auto-save into server startup

Find where `AppState` is created and the axum server is bound. Search:

```bash
grep -rn "AppState::new\|AppState::new_with" \
  crates/roko-serve/src/ crates/roko-cli/src/ --include='*.rs' | grep -v target/
```

After the `AppState` is created and wrapped in `Arc`, add:

```rust
// Restore previous server state, if a snapshot exists.
// A corrupt or missing snapshot is logged but does not prevent startup.
if let Err(e) = state.restore_snapshot().await {
    tracing::warn!(error = %e, "failed to restore server state snapshot");
}

// Persist server state every 30 seconds.
// The handle is intentionally dropped here — the task will self-terminate
// via the cancellation token when AppState::shutdown() is called.
let _auto_save_handle = roko_serve::state::spawn_auto_save(
    Arc::clone(&state),
    std::time::Duration::from_secs(30),
);
```

### Step 6 — Add unit tests

In the existing `#[cfg(test)] mod tests` block at the bottom of `state.rs`, add:

```rust
    #[tokio::test]
    async fn snapshot_roundtrip() {
        let tempdir = tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));

        // Register an agent so there is something to persist.
        state
            .upsert_discovered_agent(AgentRegistrationRecord {
                agent_id: "test-agent".into(),
                label: Some("Test".into()),
                ..Default::default()
            })
            .await;

        // Save.
        state.save_snapshot().await.expect("save snapshot");

        // Verify the file was created.
        let snap_path = state.layout.root().join("state").join("server-state.json");
        assert!(snap_path.exists(), "snapshot file must exist after save");

        // Verify no .tmp file is left behind.
        let tmp_path = snap_path.with_extension("json.tmp");
        assert!(!tmp_path.exists(), ".tmp file must be cleaned up after atomic write");

        // Create a fresh state and restore.
        let state2 = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));

        let restored = state2.restore_snapshot().await.expect("restore snapshot");
        assert!(restored, "restore must return true when snapshot exists");

        let agents = state2.list_discovered_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "test-agent");
    }

    #[tokio::test]
    async fn restore_returns_false_when_no_snapshot() {
        let tempdir = tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));

        let restored = state.restore_snapshot().await.expect("restore");
        assert!(!restored, "must return false when no snapshot file exists");
    }

    #[tokio::test]
    async fn restore_handles_corrupt_snapshot_gracefully() {
        let tempdir = tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));

        // Create a corrupt snapshot file.
        let snap_dir = tempdir.path().join("state");
        std::fs::create_dir_all(&snap_dir).unwrap();
        std::fs::write(snap_dir.join("server-state.json"), b"NOT JSON").unwrap();

        // Restore must not panic or return Err — it logs a warning and returns Ok(false).
        let restored = state.restore_snapshot().await.expect("should not return Err");
        assert!(!restored, "corrupt snapshot must be treated as missing");
    }

    #[tokio::test]
    async fn snapshot_does_not_overwrite_in_flight_state() {
        let tempdir = tempdir().expect("tempdir");
        let state = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));

        // Register agent-A and save.
        state
            .upsert_discovered_agent(AgentRegistrationRecord {
                agent_id: "agent-a".into(),
                ..Default::default()
            })
            .await;
        state.save_snapshot().await.unwrap();

        // Create a new state instance, register agent-B before restoring.
        let state2 = Arc::new(AppState::new(
            tempdir.path().to_path_buf(),
            Arc::new(NoOpRuntime),
            RokoConfig::default(),
            Arc::new(ManualBackend::default()),
        ));
        state2
            .upsert_discovered_agent(AgentRegistrationRecord {
                agent_id: "agent-b".into(),
                ..Default::default()
            })
            .await;

        // Restore: agent-a from snapshot, agent-b from in-memory registration.
        state2.restore_snapshot().await.unwrap();

        let agents = state2.list_discovered_agents().await;
        let ids: std::collections::HashSet<String> = agents.into_iter().map(|a| a.agent_id).collect();
        assert!(ids.contains("agent-a"), "agent-a must be restored from snapshot");
        assert!(ids.contains("agent-b"), "agent-b must not be overwritten by restore");
    }
```

---

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile
cargo check -p roko-serve 2>&1 | head -30

# Run the new state tests
cargo test -p roko-serve -- state:: --nocapture

# Clippy
cargo clippy -p roko-serve --no-deps -- -D warnings 2>&1 | head -20

# Format
cargo +nightly fmt --all -- --check
```

Manual smoke test:

```bash
# Start the server
cargo run -p roko-cli -- serve &
PID=$!

# Wait for the auto-save to fire (30s), then inspect the snapshot.
sleep 35
cat .roko/state/server-state.json | python3 -m json.tool | head -10

# Stop the server.
kill $PID
wait $PID

# Restart — state should restore.
cargo run -p roko-cli -- serve &
# Check the log for "server state snapshot restored".
```

Expected:
- Snapshot file appears at `.roko/state/server-state.json` within 30 seconds of server start.
- On restart, the log contains `server state snapshot restored`.
- No `.tmp` file remains after any save.
- A corrupt snapshot is skipped with a warning, not a panic.
