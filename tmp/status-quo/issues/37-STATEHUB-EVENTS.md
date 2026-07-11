# StateHub and Event Bus Issues

## High

### Broadcast channel overflow silently drops events
- `event_bus.rs:252`: Capacity 1024. Under high load (token-per-event streaming), ring fills in seconds.
- SSE handler (`sse.rs:68`) continues on `Lagged(n)` — silently skips events.
- Events not recoverable if ring has evicted them before reconnect.

### `apply_snapshot` bypasses broadcast — SSE/WS never see updates
- `state_hub.rs:172-174`: Replaces snapshot with no event broadcast.
- Called from TUI for gate trends (`app.rs:2600,2634`) and topology (`app.rs:3098`).
- SSE clients never notified of these changes. Also applies to bootstrap from workdir.

### TUI read-modify-write races with runner event publishes
- TUI (main thread): `current_snapshot()` → modify → `apply_snapshot()`.
- Runner (async): `publish(event)` → `apply()` concurrently.
- TUI's `apply_snapshot` can overwrite runner's interim events. Not atomic.

## Medium

### `publish_batch` broadcasts events before snapshot settles
- `state_hub.rs:157-169`: `event_bus.emit(event)` fires inside `send_modify` closure before `snap.apply()` completes. HTTP consumers reading snapshot see intermediate state.

### Missing DashboardEvent variants from runner path
- `TaskPhaseChanged`: Defined in `tui_bridge.rs:49-62` but never called from `event_loop.rs`.
- `EpisodeRecorded`: Only emitted from `orchestrate.rs` and `run.rs`, not runner path.
- `Diagnosis`: Only orchestrate.rs.
- Duplicate `AgentCompleted` can drive `agents_active` below actual count.

### `cost_usd_total` double-counted after bootstrap
- `dashboard_snapshot.rs:2619,1132`: Bootstrap sets total from historical efficiency.jsonl. Live `EfficiencyEvent` increments again → double-count.

### Token attribution lost under parallel agents
- `dashboard_snapshot.rs:1110-1135`: `find_agent_key_for_task` returns `None` when multiple agents active → per-agent cost/tokens silently dropped.

## Low

### `Vec::remove(0)` for gates and errors — O(n)
- `dashboard_snapshot.rs:1072-1075,1205-1208`: Inside `send_modify` write lock. Should be `VecDeque`.
