# 110 — Deprecate All JSONL File I/O — StateHub as Single Source of Truth

**Priority**: P1 — architectural debt; 26 parallel JSONL write streams and 20+ reader poll sites fragment state and guarantee stale data in the standalone TUI
**Size**: XL (6-8 weeks, phased)
**Crates**: `roko-runtime`, `roko-fs`, `roko-learn`, `roko-cli` (TUI, runner, commands), `roko-serve`, `roko-neuro`, `roko-agent`, `roko-dreams`, `roko-gate`
**Depends on**: 109 (Phase 1 — inline TUI as default must land first to reduce risk during migration)

---

## Background

Every runtime data producer in roko writes to a separate JSONL file. Every consumer reads from a
separate JSONL file. This creates 26 parallel write streams and 20+ read sites, all operating on
disk, with no coordination and no consistency guarantees.

The inline TUI (`--approval`) already proves that file I/O is unnecessary: it uses pure
StateHub push via `watch::channel` with ~0ms latency and zero file polling. The standalone
`roko dashboard` falls back to polling 20+ JSONL files because there is no IPC bridge between
the running plan process and the dashboard process. The goal of this item is to make the
StateHub path the ONLY path — eliminating the JSONL bypass layer entirely.

The mori reference implementation had zero JSONL hot paths: all events flowed through
`mpsc::channel`, updates appeared within one 16ms render frame, and no file compaction or
rotation was needed. Roko should match this.

This is a large, phased migration. Each phase ships independently and is safe to merge without
completing the next phase.

## Current State

### 26 JSONL Write Streams

| File path | Writer location | Readers | Notes |
|-----------|-----------------|---------|-------|
| `.roko/engrams.jsonl` | `roko-fs` `FileSubstrate` | TUI `SignalCursor` (`tui/cursors.rs:14`), `roko serve` `/api/signals`, `roko status` | High burst rate |
| `.roko/episodes.jsonl` | `EpisodeLogger` in `roko-cli` runner | TUI `EpisodeCursor` (`cursors.rs:84`), dream cycle, tier progression, `roko serve` | Per-task |
| `.roko/events.jsonl` | `StateHub` `EventLogWriter` in `roko-runtime` | TUI `EventLogCursor` (`cursors.rs:126`, FULL RELOAD EVERY TICK), bootstrap replay | Per event |
| `.roko/learn/efficiency.jsonl` | Runner `persist::append_jsonl` | TUI `IncrementalTailer` (`jsonl_tailer.rs:36`), cascade router, `roko serve` routes | Per LLM turn |
| `.roko/learn/c-factor.jsonl` | Runner persist | TUI `IncrementalTailer`, `roko serve` routes | Per task |
| `.roko/learn/costs.jsonl` | `CostsLog` in `roko-learn` | `roko serve` cost routes | Per turn |
| `.roko/learn/routing-decisions.jsonl` | `RoutingLog` | `roko serve` routing routes | Per dispatch |
| `.roko/learn/section-outcomes.jsonl` | `SectionOutcome` | Experiment reconciliation | Per task |
| `.roko/learn/provider-model-outcomes.jsonl` | `ProviderModelOutcome` | Provider health routing | Per dispatch |
| `.roko/learn/task-metrics.jsonl` | `MetricsWriter` | Learning aggregation | Per task |
| `.roko/learn/knowledge-seeds.jsonl` | Neuro admission in `roko-neuro` | Knowledge ingestion | Per episode |
| `.roko/learn/wal.jsonl` | Learning WAL | Cascade router, experiments, thresholds | Per observation |
| `.roko/learn/cascade-router.json` | `CascadeRouter::save()` | Runner startup, TUI, `roko serve` | Periodic |
| `.roko/learn/gate-thresholds.json` | `AdaptiveThresholds` | Gate dispatch, TUI, `roko serve` | Per gate |
| `.roko/learn/experiments.json` | `ExperimentStore` | Runner dispatch, TUI | Periodic |
| `.roko/runtime-events.jsonl` | `JsonlLogger` | Projection builder | Per event |
| `.roko/state/run-ledger.jsonl` | `RunLedger` in `roko-runtime` | Timeout terminal replay | Per attempt |
| `.roko/state/state-snapshot.json` | Runner persist | Bootstrap, resume | Periodic |
| `.roko/custody.jsonl` | Safety audit hooks in `roko-agent` | Custody verification | Per dispatch |
| `.roko/witness.jsonl` | Safety reasoning | Witness DAG queries | Per dispatch |
| `.roko/gate-verdicts.jsonl` | `EpisodeLogger` | Gate history, TUI | Per gate |
| `.roko/metrics/telemetry-observations.jsonl` | `PeriodicObserver` | Telemetry routes | Periodic |
| `.roko/task-outputs/*.txt` | Agent dispatch | TUI `TaskOutputCursors` (`task_outputs.rs`) | Per output line |
| `.roko/dreams/journal.jsonl` | Dream cycle in `roko-dreams` | TUI inspect, dream commands | Per cycle |
| `.roko/dreams/archive.jsonl` | Dream archival | TUI inspect, dream commands | Periodic |
| `.roko/projection-history.jsonl` | `StateHub` in `roko-runtime` | Lens resolution queries | Per projection |

### 20 TUI File-Poll Touchpoints

**60fps polling (every TUI render frame):**
- `.roko/engrams.jsonl` via `SignalCursor` — `crates/roko-cli/src/tui/cursors.rs` line 14
- `.roko/episodes.jsonl` via `EpisodeCursor` — `cursors.rs` line 84
- `.roko/learn/efficiency.jsonl` via `IncrementalTailer<AgentEfficiencyEvent>` — `jsonl_tailer.rs` line 36
- `.roko/learn/c-factor.jsonl` via `IncrementalTailer`
- `.roko/task-outputs/*.txt` via `TaskOutputCursors` — `tui/mod.rs` line 58 (re-exported)
- `.roko/events.jsonl` via `EventLogCursor` — `cursors.rs` line 126 (**FULL FILE RELOAD EVERY TICK**)

**200ms debounce (on file change, via `notify_debouncer_full`):**
- `crates/roko-cli/src/tui/fs_watch.rs` line 19: `const DEBOUNCE_WINDOW = Duration::from_millis(200)`
- On change, `app.rs` lines 2997-3015 call `state_hub.bootstrap_from_workdir()` + `replay_log_into_snapshot()` — full re-read

**Stamp-checked (on modification time change):**
- `.roko/learn/cascade-router.json`
- `.roko/learn/experiments.json`
- `.roko/learn/gate-thresholds.json`

**On-demand (view-specific reads):**
- `.roko/neuro/knowledge.jsonl`
- `.roko/dreams/journal.jsonl`
- `.roko/dreams/archive.jsonl`
- `.roko/jobs/*.json`
- `.roko/atelier/*.toml`

**Bootstrap-only (cold start):**
- `.roko/state/state-snapshot.json` — `StateHub.runner_projection`
- `.roko/events.jsonl` — StateHub initialization via `replay_log_into_snapshot`

### Target Architecture

```
Producer (runner, learning, gates, agents, safety)
    │
    ▼ state_hub.publish(DashboardEvent)
SharedStateHub (single owner per process)
    │
    ├──▶ watch::channel → Inline TUI (~0ms, no file I/O)
    ├──▶ broadcast → WebSocket/SSE subscribers (~0ms)
    ├──▶ Unix socket → Standalone dashboard (~1ms)
    ├──▶ REST snapshot → HTTP API queries
    │
    └──▶ Durable checkpoint (async, batched, periodic)
         └── .roko/state/statehub.db (SQLite)
             SQLite WAL mode: concurrent reads during writes
             Retention: configurable, default 7 days
             Indices on (type, timestamp), (plan_id, task_id)
```

## Implementation Plan

### Phase 1: Make Inline TUI the Default (1 day) — DO THIS FIRST

**This phase is covered in item 109 Phase 1.** It must land before this migration begins.

Once the inline TUI is the default for interactive sessions, every user gets StateHub push
semantics immediately. The JSONL bypass paths become fallback-only, not the primary path.

### Phase 2: Cross-Process IPC for Standalone Dashboard (1 week)

Enable `roko dashboard` to connect to a running `plan run`'s StateHub.

**Changes:**

In `crates/roko-runtime/src/state_hub.rs` (or the StateHub implementation file), add:

```rust
/// Write a socket path to .roko/state/hub.sock and start listening.
/// Sends a full snapshot to each new connection, then streams DashboardEvents.
pub async fn start_ipc_server(&self, sock_path: PathBuf) -> Result<()> {
    // Remove stale socket
    let _ = tokio::fs::remove_file(&sock_path).await;
    let listener = tokio::net::UnixListener::bind(&sock_path)?;
    let hub = self.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let hub = hub.clone();
                tokio::spawn(async move {
                    // Send snapshot + stream events as length-prefixed JSON frames
                    hub.stream_to_client(stream).await;
                });
            }
        }
    });
    Ok(())
}
```

In `crates/roko-cli/src/commands/plan.rs`, after the StateHub is created (before the event
loop), start the IPC server:

```rust
let sock_path = wd.join(".roko").join("state").join("hub.sock");
state_hub.start_ipc_server(sock_path).await?;
```

In `crates/roko-cli/src/tui/app.rs`, in `App::new()` (line 538), check for the socket before
falling back to file bootstrap:

```rust
let sock_path = root.as_ref().join(".roko").join("state").join("hub.sock");
if sock_path.exists() {
    return Self::new_connected_via_ipc(&sock_path, root);
}
// Existing file-bootstrap path as fallback
```

### Phase 3: Consolidate Producers into StateHub (2-3 weeks)

Migrate all JSONL writers to publish through StateHub instead. Do in this order (smallest
blast radius first):

**3a. `EpisodeLogger` (affects runner, dreams, tier progression)**

In `roko-cli` runner, wherever `EpisodeLogger::record()` is called, replace with:
```rust
state_hub.publish(DashboardEvent::EpisodeCompleted { episode });
```

The StateHub's durable backend (Phase 4) will persist it. Add `EpisodeCompleted` to
`DashboardEvent` enum in `roko-runtime` or `roko-core`.

**3b. Runner `persist::append_jsonl` calls (efficiency, c-factor, task-metrics)**

In `crates/roko-cli/src/runner/persist.rs` (or wherever `append_jsonl_line_sync` is called for
these files), replace each call with `state_hub.publish(DashboardEvent::EfficiencyRecorded { ... })`.

**3c. Learning writers (`CostsLog`, `RoutingLog`, `SectionOutcome`, etc.)**

In `roko-learn`, add a `publish_fn: Option<Box<dyn Fn(DashboardEvent) + Send>>` to each writer
struct, or pass a `StateHub` handle at construction. Convert each `append_jsonl` call to
`self.hub.publish(...)`.

**3d. `TaskOutputCursors` writes**

In `crates/roko-cli/src/tui/task_outputs.rs` (or wherever task output is written to
`.roko/task-outputs/*.txt`), replace with:
```rust
state_hub.publish(DashboardEvent::AgentOutputDelta { agent_id, line });
```

**3e. Safety audit (custody, witness)**

In `roko-agent` safety hooks, replace file writes with:
```rust
state_hub.publish(DashboardEvent::CustodyAuditRecorded { ... });
```

**3f. `FileSubstrate` (engrams) — largest change**

In `roko-fs`, add a `StateHub` handle to `FileSubstrate`. On `write()`, also call:
```rust
self.hub.publish(DashboardEvent::SignalStored { signal });
```

Eventually the file write can be removed after the durable StateHub backend lands.

**3g. Telemetry (`PeriodicObserver`)**

In `roko-cli` or `roko-runtime`, where the periodic observer writes
`telemetry-observations.jsonl`, replace with `state_hub.publish(DashboardEvent::TelemetryObservation { ... })`.

### Phase 4: Add SQLite Durable Backend (1-2 weeks)

StateHub needs persistent storage for cold start and history queries. SQLite is the recommended
option: ACID guarantees, WAL mode for concurrent reads, single file, bounded retention.

**New file:** `crates/roko-runtime/src/state_durable.rs`

```rust
pub struct StateDurableStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl StateDurableStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                ts INTEGER NOT NULL,
                kind TEXT NOT NULL,
                plan_id TEXT,
                task_id TEXT,
                data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS events_ts ON events(ts);
            CREATE INDEX IF NOT EXISTS events_plan_task ON events(plan_id, task_id);
            CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                agent_id TEXT,
                task_id TEXT,
                plan_id TEXT,
                model TEXT,
                tokens_in INTEGER,
                tokens_out INTEGER,
                cost_usd REAL,
                outcome TEXT,
                ts INTEGER
            );
            CREATE TABLE IF NOT EXISTS snapshots (
                generation INTEGER PRIMARY KEY,
                data TEXT NOT NULL,
                created_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS learning (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER
            );
        ")?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    /// Persist an event. Called by StateHub on every publish().
    pub fn record_event(&self, kind: &str, plan_id: Option<&str>, task_id: Option<&str>, data: &str) {
        // ... INSERT INTO events ...
    }

    /// Prune events older than retention_days.
    pub fn prune_old(&self, retention_days: u64) {
        // ... DELETE FROM events WHERE ts < now - retention_days * 86400000 ...
    }
}
```

In `SharedStateHub::publish()`, after delivering to all `watch::channel` subscribers, call
`self.durable.record_event(...)` asynchronously (spawn a task or use a background queue to avoid
blocking the publish path).

### Phase 5: Consolidate Readers into StateHub Queries (2-3 weeks)

Once producers write to StateHub and the durable backend exists, every JSONL reader can become a
StateHub query.

**Migration order:**

1. **TUI cursors** (`SignalCursor`, `EpisodeCursor`, `EventLogCursor`) in `cursors.rs` → read from
   `snapshot_rx` (already available in the inline TUI path; now expose via IPC for standalone)
2. **`IncrementalTailer`** in `jsonl_tailer.rs` → replace with `snapshot_rx` drain
3. **`roko serve` routes** that read JSONL files → query StateHub durable store via `store.query_events(...)`
4. **`roko learn` CLI commands** → query durable store
5. **`roko status` command** → query StateHub snapshot
6. **Dream cycle** (`roko-dreams`) → query durable store for episode history
7. **Cascade router load** → read from StateHub `learning` projection (loaded from `learning` table)
8. **Gate thresholds load** → read from StateHub `learning` projection

### Phase 6: Delete JSONL Infrastructure (1 week)

After all readers and writers are migrated, delete the JSONL infrastructure:

- Delete `crates/roko-cli/src/tui/cursors.rs` (`SignalCursor`, `EpisodeCursor`, `EventLogCursor`)
- Delete `crates/roko-cli/src/tui/jsonl_tailer.rs` (`IncrementalTailer`)
- Delete `crates/roko-cli/src/tui/task_outputs.rs` (or repurpose as StateHub-backed view)
- Delete `crates/roko-cli/src/tui/fs_watch.rs` (filesystem watcher for JSONL changes)
- Remove all `.jsonl` write calls from `roko-cli/src/runner/persist.rs`
- Remove `EpisodeLogger` file-write path (keep the in-memory struct for event construction)
- Remove `JsonlLogger` in `roko-runtime`
- Remove `log_rotation.rs` and `append_jsonl_line_sync()` helpers
- Remove `DashboardData::load_best_effort()` (the file-polling fallback)
- Set `replay_disk_snapshots = false` permanently (remove the flag)
- Clean up `FileSubstrate` — keep it as a cold-archive interface but remove hot-path writes

## Acceptance Criteria

1. `grep -rn '\.jsonl' crates/ --include='*.rs' | grep -v test | grep -v doc | grep -v target` — zero results (no production JSONL usage outside tests).
2. `ls .roko/*.jsonl .roko/learn/*.jsonl .roko/state/*.jsonl 2>/dev/null` — returns nothing for a fresh run.
3. Standalone `roko dashboard` shows identical real-time updates to the inline TUI within 2ms.
4. Cold start from `.roko/state/statehub.db` completes in under 100ms for a workspace with 100+ episodes.
5. Zero file polling in the render loop — pure `watch::channel` drain.
6. Agent output streams per-token to TUI (not batched after API completion) — requires RC-1 from item 109.
7. `cargo test --workspace` passes.
8. `roko doctor` reports "StateHub: SQLite backend, N events, M episodes" instead of listing JSONL files.

## Verification Checklist

- [ ] Complete Phase 1 (inline TUI default) from item 109 first
- [ ] Phase 2: run `roko plan run plans/ --engine runner-v2` then `roko dashboard` in second terminal → live updates appear
- [ ] Phase 3: run a full plan → verify no JSONL writes for efficiency/episodes (check with `inotifywait -r .roko/` or equivalent on macOS: `fswatch .roko/`)
- [ ] Phase 4: verify `.roko/state/statehub.db` exists and contains events table with rows after a plan run
- [ ] Phase 5: `roko learn episodes` and `roko status` work from the durable store with no `cat .roko/*.jsonl` calls
- [ ] Phase 6: `grep -rn '\.jsonl' crates/ --include='*.rs' | grep -v test | wc -l` returns 0
- [ ] `cargo test --workspace 2>&1 | tail -5` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes

## Files to Modify (by phase)

| Phase | Files | Change |
|-------|-------|--------|
| 2 | `crates/roko-runtime/src/state_hub.rs` | Add `start_ipc_server(sock_path)` method |
| 2 | `crates/roko-cli/src/commands/plan.rs` | Start IPC server after StateHub init |
| 2 | `crates/roko-cli/src/tui/app.rs` | Check for socket in `App::new()` (line 538) before file bootstrap |
| 3 | `crates/roko-cli/src/runner/persist.rs` | Replace `append_jsonl` calls with `state_hub.publish()` |
| 3 | `crates/roko-learn/src/` | Add StateHub handle to all JSONL writer structs; replace file writes with `publish()` |
| 3 | `crates/roko-agent/src/safety/` | Replace custody/witness file writes with `state_hub.publish()` |
| 3 | `crates/roko-fs/src/` | Add StateHub handle to `FileSubstrate`; dual-write during transition |
| 4 | `crates/roko-runtime/src/state_durable.rs` | New file: `StateDurableStore` with SQLite backend |
| 4 | `crates/roko-runtime/src/state_hub.rs` | Wire `StateDurableStore` into `publish()` path |
| 5 | `crates/roko-cli/src/tui/cursors.rs` | Replace file reads with `snapshot_rx` drain |
| 5 | `crates/roko-cli/src/tui/jsonl_tailer.rs` | Replace `IncrementalTailer` tick with snapshot drain |
| 5 | `crates/roko-serve/src/routes/` | Replace JSONL file reads with `store.query_events()` calls |
| 6 | `crates/roko-cli/src/tui/cursors.rs` | Delete file |
| 6 | `crates/roko-cli/src/tui/jsonl_tailer.rs` | Delete file |
| 6 | `crates/roko-cli/src/tui/fs_watch.rs` | Delete file (or keep for other uses) |
| 6 | `crates/roko-cli/src/runner/persist.rs` | Remove all JSONL write functions |
| 6 | `crates/roko-cli/src/tui/dashboard.rs` | Remove `DashboardData::load_best_effort()` |
