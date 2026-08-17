# 05 — TUI Event Parity (final closeout)

> **Source plan**: `tmp/ux/ux-followup/12-tui-event-parity.md`. Items 70,
> 71, 72, 73, 74, 76, 78 are still open (per the 2026-04-20 re-audit, the
> notify-watcher infrastructure is in but several panels still
> re-parse whole files on each refresh).
>
> **Status as of 2026-05-01**:
> - `crates/roko-cli/src/tui/fs_watch.rs` exists (notify-based, debounced)
>   and is wired in `app.rs:602`.
> - `crates/roko-cli/src/tui/git_watch.rs` exists (item 75 done).
> - `crates/roko-cli/src/tui/jsonl_tailer.rs::IncrementalTailer<T>` exists
>   *and is only used* for `efficiency.jsonl` and `c-factor.jsonl` (lines
>   402, 404, 521, 524 of `dashboard.rs`).
> - The agent status panel still reads from `executor.json` snapshot.
> - `signals.jsonl`, `episodes.jsonl`, `task-outputs/`, `events.json`,
>   `learn/*` are still re-parsed on every refresh tick via `file_stamp`
>   (`dashboard.rs` lines 414, 433-492, 559-586, etc).
> - `OnceLock<HashMap<PathBuf, ...>>` generation counter is still
>   in-process-only.
>
> **Effort**: 3-4 days.
>
> **Risk**: Low. Each item is local to one module. Behaviour change is
> "less CPU, fresher data"; UX surface unchanged.

---

## What this plan accomplishes

Eliminate the remaining "re-parse whole file on stamp change" sites in
the TUI, replacing them with `IncrementalTailer<T>` (already proven for
efficiency / c-factor data) or a one-time directory subscription on
`fs_watch`.

After this plan:

- Item 70: Agent panel renders from a per-agent `/stream` WS subscription
  (fed by aggregator multiplex).
- Item 71: Gate signals tail incrementally from `.roko/signals.jsonl`.
- Item 72: Task outputs are watched per-file via `fs_watch`; only changed
  files get re-tailed.
- Item 73: Episode log incrementally tailed.
- Item 74: Event log incrementally tailed.
- Item 76: Learning trio (efficiency, experiments, gate-thresholds) all
  use `IncrementalTailer` (efficiency already does).
- Item 78: Generation counter persists to `.roko/state/dashboard-gen.json`.

The "polling is a bug" closeout is finally complete.

## Why this matters

Each polling site causes O(N) work on each refresh tick where N is the
size of an append-only log. For a long-running session, episodes can
exceed 100 MB; re-parsing every refresh is the single largest CPU sink in
the TUI. The TUI also feels laggy — a new event takes the polling
interval (500 ms) plus the parse latency to surface. Incremental reads
get this to <50 ms end-to-end.

---

## Required reading

```
crates/roko-cli/src/tui/jsonl_tailer.rs           (template)
crates/roko-cli/src/tui/jsonl_cursor.rs
crates/roko-cli/src/tui/fs_watch.rs
crates/roko-cli/src/tui/git_watch.rs
crates/roko-cli/src/tui/dashboard.rs              (the polling sites)
crates/roko-cli/src/tui/app.rs                    (refresh dispatch)
crates/roko-cli/src/tui/views/agents_view.rs      (item 70 target)
crates/roko-cli/src/tui/state.rs                  (TuiState; per-agent fields)
crates/roko-cli/src/tui/ws_client.rs              (existing WS client; for item 70)
crates/roko-cli/src/tui/verdicts.rs               (already-built incremental substrate reader)
crates/roko-serve/src/routes/aggregator.rs        (the /api/ws multiplex consumer)
tmp/ux/ux-followup/12-tui-event-parity.md
```

---

## Deliverables (per item)

### Item 70 — Agent panel via aggregator `/api/ws`

1. New `tui/agent_streams.rs`. Subscribes to
   `ws://{aggregator_host}/api/ws` when the Agents tab activates and
   pushes incoming events into `TuiState::agent_streams: HashMap<AgentId, RingBuffer<StreamEvent>>`.
2. `RingBuffer` capped at 200 events per agent, drop-oldest.
3. WS reconnects on close with exponential back-off (250 ms → 1 s → 4 s,
   cap at 10 s).
4. Closes WS when the user leaves the Agents tab. Save state for the
   current selection to avoid blink on revisit.
5. Render the live tail in the agent detail panel as the latest 10 events
   in monospace, oldest at top.

### Item 71 — Gate signals incremental tail

1. Replace `load_signal_state(&signals_path)` (line 414 of
   `dashboard.rs`) with `IncrementalTailer<SignalEntry>`.
2. The aggregator helpers `gate_signal_summaries` and `signal_gate_results`
   become incremental reducers — accept the existing accumulator and the
   new slice; return updated accumulator.
3. Move signal summaries / `signals_state` to live alongside the tailer
   inside `DashboardData` (already partly there; complete the move).

### Item 72 — Task outputs watched

1. Use `fs_watch` (already running) to receive change events. Subscribe
   to `.roko/task-outputs/` with `RecursiveMode::NonRecursive`.
2. Per file, maintain a `JsonlCursor`. On a watch event, advance only
   the cursor for the changed file. New file event creates a new cursor.
3. Cap to last N=20 task output files watched concurrently; older files
   become read-once snapshots displayed but not tailed.

### Item 73 — Episode log incremental

1. Replace `refresh_episodes(&episodes_path, stamp)` (line 561 of
   `dashboard.rs`) with `IncrementalTailer<Episode>`.
2. Keep the existing `episodes_state` shape but populate it from the new
   tailer's items.
3. On tail size > 10 000, drop the oldest 1 000 entries to keep memory
   bounded. The dashboard view only needs the recent window.

### Item 74 — Event log incremental

1. `events.json` is a *single JSON array*, not JSONL. Two options:
   - **A.** Convert `roko-runtime::event_bus` to write JSONL instead of
     a JSON array. This is the better long-term move; it is also
     coupled across the runtime and warrants its own micro-task.
   - **B.** For this plan: keep the array shape, but add a `JsonArrayTailer`
     adjacent to `JsonlTailer` that incrementally parses array items by
     scanning for `},\n  {` boundaries. ~100 LOC, less invasive.

   Choose **A** if `event_bus` is touched elsewhere this sprint; otherwise
   **B**. Document the choice.

### Item 76 — Learning trio incremental

1. `experiments.jsonl` and `gate-thresholds.json` get `IncrementalTailer`
   (or, for the latter, a single-snapshot reader updated on `fs_watch`
   notifications).
2. `cascade-router.json` is a snapshot file (not append-only) — replace
   the stamp poll with an `fs_watch` listener that reloads on change.
3. Touch up `dashboard.rs:565-586` so the four files share the same
   pattern.

### Item 78 — Generation counter durable

1. New `crates/roko-cli/src/tui/dashboard_gen_persist.rs`.
2. On startup, read `.roko/state/dashboard-gen.json` if present; populate
   the in-memory `OnceLock<...>` from it.
3. On every generation increment, write the file atomically (write to
   `.roko/state/dashboard-gen.json.tmp`, fsync, rename).
4. Cap entries at 1 000; drop the least-recently-updated when exceeded.

---

## Step-by-step

For each numbered item below, follow this rhythm:

1. Read the existing site identified in `12-tui-event-parity.md`.
2. Read the analogous already-converted site (`efficiency_tailer` is the
   gold reference — `dashboard.rs:402,521`).
3. Write the new field on `DashboardData` and initialize it in
   `DashboardData::new`.
4. Replace the stamp/poll branch with a `tick()` call.
5. Update the consumer (UI panel) to read from the tailer's `items()`.
6. Add a unit test in the same file's `mod tests` block.

A representative diff for item 73:

```rust
// dashboard.rs (sketch)

pub struct DashboardData {
    // BEFORE
    // episodes_state: EpisodesState,
    // episodes_stamp: Option<FileStamp>,

    // AFTER
    episodes_tailer: super::jsonl_tailer::IncrementalTailer<Episode>,
}

// In tick():
//   BEFORE
//     let stamp = file_stamp(&episodes_path);
//     if stamp != self.episodes_stamp {
//         self.refresh_episodes(&episodes_path, stamp);
//         generation_changed = true;
//     }
//   AFTER
let added = self.episodes_tailer.tick().unwrap_or(0);
if added > 0 {
    generation_changed = true;
}
```

The consumer (e.g. `views/episodes_view.rs`) calls
`data.episodes_tailer.items()` instead of `data.episodes_state.entries`.

### Order of work (dependency-light, in any order)

| Item | LOC delta | Notes |
|------|----------|-------|
| 76 (learning trio) | ~80 | Reuse efficiency_tailer pattern. Easiest. |
| 73 (episodes) | ~100 | Pure JSONL; plug in IncrementalTailer. |
| 71 (signals) | ~150 | Aggregator-style reducer needs care. |
| 72 (task outputs) | ~200 | Per-file cursor map; coordinate with `fs_watch`. |
| 74 (events) | ~120 | Choice between A and B above. |
| 78 (gen counter) | ~80 | Atomic file write. |
| 70 (agent stream WS) | ~250 | The biggest single item. Cross-cuts WS client. |

Land each as its own commit; revert is trivial if a panel regresses.

---

## Anti-patterns to avoid

- **Don't add fresh polling alongside the new tailers.** The whole point
  is to delete `file_stamp` calls. If you find yourself writing
  `if stamp_changed { ... } else { tailer.tick(); }`, reread item 71 in
  `12-tui-event-parity.md`.
- **Don't unbound the in-memory tailer Vec.** All long-running
  consumers must drop oldest on a cap. 10 000 episodes × ~2 KB =
  20 MB; that's the budget.
- **Don't run the WS client on the render thread.** Spawn a
  `tokio::task::spawn` and forward via `tokio::sync::mpsc` to the TUI
  state. The render loop is sync and must remain so for ratatui's
  draw model.
- **Don't read `.roko/state/dashboard-gen.json` on every render.**
  Read once at startup; write on increments. The whole point of
  persisting is to avoid the per-render hit.
- **Don't add new `std::thread::sleep(...)` polling threads.** All
  watchers go through `notify` via `fs_watch.rs`.
- **Don't break truncation handling.** `IncrementalTailer::tick`
  already handles file truncation (line 95-114). Don't paper over it
  in your new site; reuse the cursor.
- **Don't skip the unit test.** Each item gets a test in
  `dashboard.rs::tests` (or a sibling test file). The existing
  `IncrementalTailer` tests are the template.

## Done when

1. `rg 'file_stamp\\(' crates/roko-cli/src/tui/` returns 0 hits inside
   `dashboard.rs::tick` (file_stamp may remain elsewhere as a utility).
2. `rg 'std::thread::sleep' crates/roko-cli/src/tui/` returns 0 hits.
3. The TUI Agents tab shows live agent stream events within ~200 ms of
   them landing on the aggregator WS.
4. `cargo test -p roko-cli --lib tui` passes (including the new tests).
5. After running for 30 minutes with a generating workload, peak memory
   does not exceed 1.5× the steady-state memory at the 30 s mark
   (catches the unbounded growth bug).
6. `.roko/state/dashboard-gen.json` is present after a TUI session and
   contains the latest counter.
7. `tmp/ux/ux-followup/12-tui-event-parity.md` items 70-78 marked DONE
   with this plan referenced.
