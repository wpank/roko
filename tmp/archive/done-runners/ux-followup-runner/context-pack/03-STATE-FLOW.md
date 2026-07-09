# State Flow — StateHub / DashboardSnapshot / EventBus

Required reading for every batch that touches the TUI, the HTTP surface, or
the orchestrator event emission path. This file is the reference for UX05-UX22
(TUI streaming + observability) and UX12-UX14 (state mgmt).

## The three channels

```
┌────────────────────┐     push     ┌────────────────┐
│ orchestrator /     │ ───────────► │   StateHub     │
│ gate pipeline /    │              │   (roko-core)  │
│ conductor          │              └───────┬────────┘
└────────────────────┘                      │  watch::Sender
                                            ▼
┌────────────────────┐  watch::Rx   ┌────────────────┐
│ TUI App            │ ◄─────────── │  DashboardSnap │
│ (roko-cli/tui)     │              │    (Arc<…>)    │
└─────────┬──────────┘              └────────────────┘
          │ render                           ▲
          ▼                                  │
     [rendered UI]              HTTP /api/*  │
                                             │
                                   ┌─────────┴────────┐
                                   │  roko-serve      │
                                   │  routes/*.rs     │
                                   └──────────────────┘
```

## Components

### StateHub (`crates/roko-core/src/state_hub.rs`)

- Single owner of the authoritative `DashboardSnapshot`.
- Exposes a `tokio::sync::watch::Receiver<DashboardSnapshot>` via
  `fn snapshot(&self) -> watch::Receiver<DashboardSnapshot>`.
- Subscribers poll `rx.has_changed()` and call `rx.borrow_and_update()` to
  get a zero-copy `Ref`.
- Mutations go through `StateHub::apply(event: DashboardEvent)`.

### DashboardSnapshot

- Struct aggregating everything the dashboard renders: plans, agents,
  gate verdicts, episodes, efficiency, experiments, cascade state,
  conductor diagnoses, approval requests, errors.
- Cheaply clonable (`Arc`-wrapped fields where relevant).

### DashboardEvent

- `enum DashboardEvent { PlanAdded(..), TaskStarted(..), GateVerdict(..), ... }`.
- Emitted from orchestrator on every state transition.

### EventBus (`crates/roko-runtime/src/event_bus.rs`)

- `RokoEvent` broadcast channel for cross-crate fan-out.
- UX02 will add `RokoEvent::PrdPublished { slug, path }`.
- UX01 will add `RokoEvent::PlanRevision { plan_id, reason, failures: Vec<GateFailure> }`.
- Receivers subscribe via `event_bus.subscribe()` and get a `broadcast::Receiver`.

## TUI consumption path

The TUI is supposed to run in two modes:

1. **Connected** (what already works):
   ```rust
   let snapshot_rx = state_hub.snapshot();
   app.snapshot_rx = Some(snapshot_rx);
   ```
   In the event loop:
   ```rust
   if let Some(rx) = &mut app.snapshot_rx {
       if rx.has_changed().unwrap_or(false) {
           let snapshot = rx.borrow_and_update();
           app.tui_state.update_from_dashboard_snapshot(&snapshot);
       }
   }
   ```

2. **Standalone** (the bug surface UX05 fixes):
   - Currently: `snapshot_rx = None` → falls through to a 500 ms polling
     thread that walks `.roko/` (`app.rs:526-549`) + a per-tick sync refresh
     branch at `app.rs:357-361`.
   - Target: spawn an in-process `SharedStateHub`, hand the TUI its
     `watch::Receiver`, and delete the polling branches entirely.

## Incremental tail pattern (UX07, UX08)

JSONL files in `.roko/` should be tailed incrementally:

```rust
struct JsonlCursor {
    path: PathBuf,
    offset: u64,
    last_line_number: usize,
}

impl JsonlCursor {
    fn read_new_lines(&mut self) -> std::io::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        self.offset = file.metadata()?.len();
        Ok(buf.lines().map(String::from).collect())
    }
}
```

Fire the cursor on a `notify` event (UX06 introduces the watcher).

## File paths the TUI currently polls (target for UX06-UX10)

| Path | Consumer | Batch |
|------|----------|-------|
| `.roko/state/executor.json` | `DashboardData::refresh_sync` | UX05/UX06 |
| `.roko/signals.jsonl` | `load_signal_state` | UX07 |
| `.roko/episodes.jsonl` | `load_episodes_from_path` | UX07 |
| `.roko/state/events.json` | `load_event_log` | UX07 |
| `.roko/task-outputs/` (dir) | `load_task_outputs` | UX08 |
| `.roko/learn/efficiency.jsonl` | `read_efficiency_events_sync` | UX06 |
| `.roko/learn/experiments.json` | `load_json_opt::<ExperimentStore>` | UX06 |
| `.roko/learn/gate-thresholds.json` | `load_json_opt::<AdaptiveThresholds>` | UX06 |
| `.roko/learn/cascade-router.json` | `load_json_opt::<CascadeRouterState>` | UX06 |
| `.roko/learn/c-factor.jsonl` | `load_latest_jsonl_value::<CFactor>` | UX06 |
| `.git/HEAD` + `.git/refs/heads/*` | `git_view::collect_git_data` | UX10 |

## Verdicts substrate (UX15)

`run_gate_rung` at `crates/roko-cli/src/orchestrate.rs:11170-11188` writes each
verdict via `FileSubstrate::put` as a signal with `Kind::GateVerdict`. No
readers currently consume these signals — UX15 adds the reader + Gate-trend
widget.

## DashboardSnapshot fields — extension targets

Fields to **add** as part of this runner:

| Field | Added by | Reason |
|-------|---------|--------|
| `diagnoses: Vec<DiagnosisSummary>` | UX16 | conductor diagnosis panel |
| `experiment_winners: Vec<ExperimentWinner>` | UX19 | Learning tab widget |
| `agent_topology: AgentTopology` | UX20 | topology widget |
| `cfactor_trend: TrendBuckets` | UX22 | c-factor trend |
| `gate_trends: HashMap<String, TrendBuckets>` | UX15 | per-gate pass/fail |
| `efficiency_trend: TrendBuckets` | UX17 | Learning sparkline |

All additions should:
- be cheap to clone (use `Arc<Vec<_>>` or similar)
- have a `Default` impl
- stay append-only on the serde shape (add `#[serde(default)]`)

## Snapshot persistence (UX12, UX13)

`ExecutorSnapshot` (struct in `crates/roko-cli/src/orchestrate.rs`) is
serialized whole to `.roko/state/executor.json` via `save_snapshot_atomic`
(line ~646). There is no `schema_version` field — UX12 adds it. UX13 then
validates plan-discovery consistency at resume time.

## Events that every batch should consider emitting

Any state-changing operation should emit a `DashboardEvent` (for the TUI) and,
if cross-crate, a `RokoEvent` (for other subscribers). If your batch adds a
new entity (diagnosis, experiment winner, topology), add a matching event
variant so the TUI updates without a file-system trip.
