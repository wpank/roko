# 109 — TUI Real-Time Streaming and Mori Parity

**Priority**: P1 — the standalone `roko dashboard` is architecturally disconnected from the running plan; users see stale or no data by default
**Size**: XL (2-3 weeks)
**Crates**: `roko-cli` (`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/`), `roko-runtime` (`/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/`), `roko-gate` (`/Users/will/dev/nunchi/roko/roko/crates/roko-gate/`)
**Depends on**: 108 (covers quick wins that should be done first)

---

## Background

Roko's TUI feels stuck and unresponsive compared to the mori orchestrator it replaces. Mori
pushed all events through in-memory `mpsc::channel` with ~0ms latency. Every agent output
line, gate progress update, and token count appeared within one 16ms render frame.

The good news: roko ALREADY has this architecture. When `plan run --approval` is used, the
runner publishes all events through `TuiBridge` → `StateHub` → `watch::channel`. The inline TUI
receives them with ~0ms latency and zero file polling. This is the mori-like path.

The problem is that the default user experience uses `roko dashboard` in a separate terminal,
which creates its own isolated in-process `SharedStateHub` with NO connection to the running
`plan run` process. It falls back to polling 20+ JSONL files with a 200ms debounce, and
re-bootstraps the entire state on every filesystem event.

Seven root causes have been identified, ordered by user-visible impact.

## Current State

### RC-1: Event loop uses batching dispatch, not the existing streaming path

**Location:** `crates/roko-cli/src/runner/event_loop.rs` line 10544 → `crates/roko-cli/src/dispatch/factory.rs` line 317 → `crates/roko-cli/src/dispatch_v2.rs` line 1495

The event loop calls:
```rust
let bridge = ctx.factory.spawn_shared_agent_bridge(request, raw_agent_tx);
```

`spawn_shared_agent_bridge` at `factory.rs` line 317 calls
`run_agent_result_bridge_with_tools_and_cli_mcp()` which ends at `dispatch_v2.rs` line 1495:
```rust
let mut result = agent.run(&input, &Context::now()).await;
```

This is the **batching path** — it blocks until the full API response arrives before emitting
any events. A streaming path already exists at `dispatch_v2.rs` line 1320:

```rust
pub async fn run_agent_streaming(
    &self,
    request: AgentDispatchRequest,
    event_tx: mpsc::Sender<AgentRuntimeEvent>,
) -> Result<AgentResult, DispatchV2Error> {
    // ...
    let mut result = created.agent.run_streaming(&input, &Context::now(), chunk_tx).await;
```

This path calls `agent.run_streaming()` and forwards `StreamChunk` events in real time. It is
NEVER called by the event loop today.

### RC-2: Standalone dashboard polls files instead of receiving push events

**Location:** `crates/roko-cli/src/tui/app.rs` lines 535-547

```rust
pub fn new(root: impl AsRef<Path>) -> Self {
    let state_hub = crate::state_hub::SharedStateHub::new_in_process();
    let _ = state_hub.bootstrap_from_workdir(root.as_ref());
    let events_path = root.as_ref().join(".roko").join("events.jsonl");
    let count = state_hub.replay_log_into_snapshot(&events_path);
```

This creates an isolated in-process StateHub. It has NO IPC connection to a running `plan run`.
The `fs_watch.rs` debouncer (`crates/roko-cli/src/tui/fs_watch.rs` line 19:
`const DEBOUNCE_WINDOW: Duration = Duration::from_millis(200)`) triggers full re-bootstrap from
disk on every file change.

The connected path (used only with `--approval`) shares a StateHub via `watch::channel` with
~0ms latency. The standalone path has 200ms+ latency when files are flushed; longer if they
aren't.

### RC-3: Timeout handler has an infinite retry loop with no exit condition for conflicts

**Location:** `crates/roko-cli/src/runner/event_loop.rs` lines 13077-13108, `crates/roko-runtime/src/run_ledger.rs` line 648

The `handle_global_timeout` function (line 13015) loops:
```rust
loop {
    let cancellation = stop_all_agents(...).await;
    if cancellation.all_confirmed() { break; }
    // ... sleep 1 second, repeat ...
}
```

`stop_all_agents` calls `persist_pending_terminal()` which calls `TimeoutTerminalReplay::record()`
(`run_ledger.rs` line 642). If the key already exists with different metadata,
`record()` returns `Err(TimeoutLedgerConflict::ConflictingTerminal { key })` at line 648.

The caller at event_loop.rs does NOT break out of the retry loop on `ConflictingTerminal`.
`all_confirmed()` can never return `true` for the conflicting attempt. The loop runs forever
at 1 iteration/second, producing log spam and wasting CPU.

### RC-4: Efficiency metrics schema mismatch between writer and TUI reader

**Location:** `crates/roko-cli/src/dispatch_v2.rs` line 117 (writer), `crates/roko-cli/src/tui/state.rs` line 1223 (reader)

Two schemas exist for efficiency events:
- `roko_learn::efficiency::AgentEfficiencyEvent` — the schema the TUI reads (used in `state.rs` at line 1223 and `jsonl_tailer.rs`)
- A newer schema written by the dispatch feedback path, with different field names

The TUI's `IncrementalTailer<AgentEfficiencyEvent>` silently drops records that don't match the
old schema. The efficiency panel shows zeros while the header bar may show non-zero tokens from
ephemeral in-memory state.

### RC-5: Gate pipeline runs full workspace compile for focused/markdown tasks

**Location:** `crates/roko-cli/src/runner/event_loop.rs` lines 8529-8536

```rust
fn gate_plan_complexity_for_task(task_def: Option<&TaskDef>) -> PlanComplexity {
    match task_def.map(|task| task.tier.as_str()).unwrap_or("focused") {
        "mechanical" | "fast" => PlanComplexity::Trivial,
        "focused" => PlanComplexity::Simple,   // ← Compile + Lint rungs
```

`PlanComplexity::Simple` → `[Rung::Compile, Rung::Lint]` per `crates/roko-gate/src/rung_selector.rs`
line 237. Even for a task that only writes markdown, `cargo check` + `cargo clippy` run on the
full 35-crate workspace in an isolated worktree with a cold `target/` cache.

### RC-6: Full state re-bootstrap on every file change

**Location:** `crates/roko-cli/src/tui/app.rs` lines 2997-3015

```rust
if got_refresh {
    if let Some(state_hub) = &self._state_hub {
        if self.replay_disk_snapshots {
            let _ = state_hub.bootstrap_from_workdir(&self.workdir);
            let events_path = self.workdir.join(".roko").join("events.jsonl");
            state_hub.replay_log_into_snapshot(&events_path);
        }
    }
}
```

On every filesystem watch event (after 200ms debounce), the standalone dashboard re-reads the
entire state snapshot from disk and replays all events in `events.jsonl`. This is O(n) in the
number of events ever written.

### RC-7: No gate progress visibility in the TUI

**Location:** Gate dispatch in `crates/roko-cli/src/runner/gate_dispatch.rs`

Gate rungs (compile, lint, test) run in a worktree subprocess. Zero progress events are emitted.
The TUI shows "gating" with no indication of which rung is running, how long it has been
running, or what is being compiled. No estimated time remaining.

## Implementation Plan

The fixes are ordered by impact. Complete each before moving to the next, as later fixes depend
on the infrastructure built by earlier ones.

### Phase 1: Fix RC-2 — Make inline TUI the default (1 day)

This gives every user the mori-like experience immediately without any IPC work.

In `crates/roko-cli/src/commands/plan.rs`, in the `PlanCmd::Run` handler (line 713):

```rust
// Before:
if approval {

// After:
let effective_approval = approval || (
    !cli.quiet && !cli.json && std::io::stdout().is_terminal()
);
if effective_approval {
```

Add `use std::io::IsTerminal;` at the top of the file if not present. Add `--no-tui` to
`PlanCmd::Run` in `main.rs` to allow opting out. This eliminates the standalone dashboard
polling path for all interactive sessions.

Also: add `alias = "tui"` to the `approval` field definition in `main.rs` line 1506 so
`--tui` works.

### Phase 2: Fix RC-3 — Infinite timeout retry loop (0.5 days)

In `crates/roko-cli/src/runner/event_loop.rs`, in `handle_global_timeout` (line 13077), track
the number of retry attempts and break on `ConflictingTerminal`:

```rust
let mut retry_count = 0;
const MAX_TIMEOUT_RETRIES: u32 = 5;
loop {
    let cancellation = stop_all_agents(...).await;
    if cancellation.all_confirmed() || retry_count >= MAX_TIMEOUT_RETRIES {
        break;
    }
    retry_count += 1;
    // ... sleep, repeat
}
```

Also: in `stop_all_agents` or `persist_pending_terminal`, handle `ConflictingTerminal` by
logging once at `WARN` level and treating the attempt as confirmed (the terminal is already
recorded, just with different metadata):

```rust
Err(TimeoutLedgerConflict::ConflictingTerminal { key }) => {
    tracing::warn!(?key, "timeout terminal conflict — treating as confirmed");
    // mark attempt confirmed so all_confirmed() can return true
}
```

### Phase 3: Fix RC-4 — Efficiency schema mismatch (1 day)

Check the exact schema being written by the dispatch feedback path
(`crates/roko-cli/src/dispatch_v2.rs` around line 117). Compare it to
`roko_learn::efficiency::AgentEfficiencyEvent` (the struct in `roko-learn`).

Two options:
1. **Unify schemas**: Update the dispatch writer to write `AgentEfficiencyEvent`. This is
   preferred if the structs are close in shape.
2. **Dual-format reader**: Make `IncrementalTailer` try both schemas and accept either.

In `crates/roko-cli/src/tui/state.rs`, line 1223, the TUI stores efficiency events as
`Vec<roko_learn::efficiency::AgentEfficiencyEvent>`. After unifying, verify that the efficiency
panel (token count, cost, success rate) shows live data.

### Phase 4: Fix RC-1 — Wire streaming dispatch in the event loop (2 days)

This is the core streaming fix. The event loop needs to call `run_agent_streaming()` instead of
the batching path.

In `crates/roko-cli/src/dispatch/factory.rs`, add a `spawn_streaming_agent_bridge` method
alongside the existing `spawn_shared_agent_bridge` (line 317):

```rust
pub fn spawn_streaming_agent_bridge(
    &self,
    request: AgentDispatchRequest,
    event_tx: mpsc::Sender<AgentRuntimeEvent>,
) -> AgentBridgeHandle {
    // ... similar to spawn_shared_agent_bridge but calls run_agent_streaming() instead of
    // run_agent_result_bridge_with_tools_and_cli_mcp()
}
```

In `crates/roko-cli/src/runner/event_loop.rs` at line 10544, replace:
```rust
let bridge = ctx.factory.spawn_shared_agent_bridge(request, raw_agent_tx);
```
with:
```rust
let bridge = ctx.factory.spawn_streaming_agent_bridge(request, raw_agent_tx);
```

The `run_agent_streaming` path already exists in `dispatch_v2.rs` lines 1315-1360. The main
work is ensuring the tool loop and MCP handling are preserved. Check whether
`run_agent_streaming` includes the full tool-loop iteration or just single-shot streaming; if
it's single-shot, the tool loop needs to be wrapped around it.

**Prerequisite**: agents that don't support streaming must fall back gracefully. Add a check:
```rust
if agent.supports_streaming() {
    factory.spawn_streaming_agent_bridge(request, event_tx)
} else {
    factory.spawn_shared_agent_bridge(request, event_tx)
}
```

### Phase 5: Fix RC-2 — Cross-process IPC for standalone dashboard (1 week)

This eliminates the need for the standalone dashboard to poll files. After Phase 1 (default
inline TUI), this is lower priority but needed for the `roko dashboard` use case.

**Option A: Unix socket (recommended)**

When a `plan run` starts, write the socket path to `.roko/state/hub.sock`. When `roko dashboard`
starts, check for this file and connect to the socket. The running process listens on the socket
and streams `DashboardEvent` JSON frames to connected clients.

In `crates/roko-runtime/src/state_hub.rs` (or wherever `SharedStateHub` is defined), add:
```rust
pub async fn listen_on_socket(&self, path: &Path) -> Result<()> {
    let listener = tokio::net::UnixListener::bind(path)?;
    // For each connection, send a snapshot followed by a stream of events
    // Use length-prefixed JSON frames
}
```

In `crates/roko-cli/src/tui/app.rs`, check for `.roko/state/hub.sock` at startup:
```rust
if sock_path.exists() {
    Self::new_connected_via_ipc(&sock_path)
} else {
    Self::new(root)  // existing file-poll fallback
}
```

**Option B: Use `roko-serve` SSE endpoint (simpler)**

If `roko serve` is running, the standalone dashboard can connect to its SSE endpoint at
`/api/events/stream` (or similar). No new socket infrastructure needed. The downside is that
this requires `roko serve` to be running.

### Phase 6: Fix RC-5 — Skip cargo gates for non-Rust changes (1 day)

(This is also in item 108, fix 4. Included here for completeness in the full streaming/perf
context.)

In `gate_plan_complexity_for_task` (event_loop.rs line 8529), check for Rust file changes before
defaulting to Compile+Lint gates. See item 108 for the implementation details.

### Phase 7: Fix RC-6 — Incremental state updates instead of full re-bootstrap (2 days)

Only relevant if the standalone dashboard survives after Phase 1 (inline TUI default) and Phase
5 (IPC). If RC-2 is fixed via IPC, this is moot — the IPC path is already incremental.

If the file-polling path must remain as a fallback: cache a SHA-256 hash of
`state-snapshot.json`. On filesystem event, read the hash first (cheap) and only re-bootstrap
if the hash changed.

```rust
let new_hash = sha256_of_file(&snapshot_path)?;
if new_hash != self.last_snapshot_hash {
    state_hub.bootstrap_from_workdir(&self.workdir);
    self.last_snapshot_hash = new_hash;
}
```

Also: add an event cursor that tracks the offset in `events.jsonl` and only reads new lines
rather than replaying from the start.

### Phase 8: Fix RC-7 — Gate progress visibility (1 day)

In `crates/roko-cli/src/runner/gate_dispatch.rs`, before executing each gate rung, push a
`DashboardEvent::GateProgress { task_id, plan_id, rung_name, elapsed_ms }` event through the
TUI bridge. In the TUI, display the current rung name and elapsed time on the task status card
when state is "gating."

## Mori vs Roko Architecture Reference

| Aspect | Mori | Roko (inline TUI `--approval`) | Roko (standalone `dashboard`) |
|--------|------|-------------------------------|-------------------------------|
| Event delivery | `mpsc::channel` push (~0ms) | `watch::channel` push (~0ms) | File polling (200ms debounce) |
| Output streaming | Per-token via channel | Batching (not yet streaming) | Via file, if flushed |
| State updates | Incremental append | Incremental via StateHub | Full re-bootstrap |
| Gate progress | Dedicated progress channel | No visibility | No visibility |
| Data layers | 1 (in-memory state) | 3 (DashboardData→StateHub→TuiState) | 4 (disk→DashboardData→StateHub→TuiState) |

The inline TUI path (`--approval`) is already close to mori. Making it the default (Phase 1) is
the highest-leverage action.

## 20 File-Poll Touchpoints to Eliminate

All used ONLY by standalone `roko dashboard` (not inline TUI). Eliminating them is Phase 5+.

**High-frequency (every TUI frame):**
- `.roko/engrams.jsonl` via `SignalCursor` (`crates/roko-cli/src/tui/cursors.rs` line 14)
- `.roko/episodes.jsonl` via `EpisodeCursor` (`cursors.rs` line 84)
- `.roko/learn/efficiency.jsonl` via `IncrementalTailer` (`jsonl_tailer.rs` line 36)
- `.roko/learn/c-factor.jsonl` via `IncrementalTailer`
- `.roko/task-outputs/*.txt` via `TaskOutputCursors` (`task_outputs.rs`)
- `.roko/events.jsonl` via `EventLogCursor` (`cursors.rs` line 126) — FULL RELOAD EVERY TICK

**Medium-frequency (stamp-checked on modification):**
- `.roko/learn/cascade-router.json`
- `.roko/learn/experiments.json`
- `.roko/learn/gate-thresholds.json`

## Acceptance Criteria

1. `roko plan run plans/` (no `--approval`) in an interactive terminal shows the inline TUI automatically — Phase 1.
2. Timeout cancellation loop in `handle_global_timeout` terminates within 5 retries or on `ConflictingTerminal` — Phase 2.
3. Efficiency panel shows correct token counts from live dispatch (not zeros) — Phase 3.
4. During an OpenAI-compat API call, streaming text chunks appear in the TUI as they arrive — Phase 4.
5. `roko dashboard` in a second terminal connects to the running `plan run` via Unix socket and shows live updates — Phase 5.
6. A markdown-only task does not trigger `cargo check` or `cargo clippy` gates — Phase 6.
7. All existing tests pass: `cargo test -p roko-cli -p roko-runtime -p roko-gate`.

## Verification Checklist

- [ ] `cargo run -p roko-cli --bin roko -- plan run plans/ --engine runner-v2` (no `--approval`) → TUI appears
- [ ] Run with z.ai provider → streaming text chunks appear during API call (not after completion)
- [ ] `grep -n 'all_confirmed\|ConflictingTerminal' crates/roko-cli/src/runner/event_loop.rs` → confirm max retry bound
- [ ] Efficiency panel shows non-zero tokens for a completed dispatch-v2 run
- [ ] `ls .roko/state/hub.sock` → socket file exists during `plan run`
- [ ] Second terminal `roko dashboard` → shows live agent output
- [ ] `cargo test --workspace 2>&1 | tail -5` → passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/plan.rs` | Default `approval = true` when stdout is TTY (line 713); add `--no-tui` flag |
| `crates/roko-cli/src/main.rs` | Add `no_tui: bool` to `PlanCmd::Run`; add `alias = "tui"` to `approval` field (line 1506) |
| `crates/roko-cli/src/runner/event_loop.rs` | Add max retry + `ConflictingTerminal` break in `handle_global_timeout` (line 13077); switch dispatch call at line 10544 from `spawn_shared_agent_bridge` to `spawn_streaming_agent_bridge` |
| `crates/roko-cli/src/dispatch/factory.rs` | Add `spawn_streaming_agent_bridge` method (alongside line 317) |
| `crates/roko-cli/src/dispatch_v2.rs` | Verify `run_agent_streaming` (line 1320) includes tool loop iteration; fix efficiency schema mismatch (line 117) |
| `crates/roko-cli/src/tui/app.rs` | Check for Unix socket at startup (lines 535-548); skip re-bootstrap if snapshot hash unchanged (lines 2997-3015) |
| `crates/roko-cli/src/tui/state.rs` | Unify efficiency event schema (line 1223) |
| `crates/roko-cli/src/tui/fs_watch.rs` | Extend with fallback path description (line 19 debounce constant) |
| `crates/roko-runtime/src/run_ledger.rs` | Handle `ConflictingTerminal` gracefully in timeout replay (line 648) |
| `crates/roko-gate/src/rung_selector.rs` | Skip Compile+Lint rungs for non-Rust change sets |
| `crates/roko-cli/src/runner/gate_dispatch.rs` | Emit `GateProgress` events before each rung execution |
