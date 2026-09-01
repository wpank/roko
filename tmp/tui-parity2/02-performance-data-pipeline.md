# Performance Audit: TUI Data Pipeline

**Observed**: ~2.1 FPS
**Target**: 60+ FPS
**Root cause**: a combination of per-frame CPU waste across data update, rendering, and
post-processing layers. No single issue dominates; fixing the top five together should
reach the target.

All paths reference:
- **state.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` (6,225 lines)
- **app.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` (5,314 lines)
- **postfx.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx.rs` (1,055 lines)
- **postfx_pipeline.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/postfx_pipeline.rs` (184 lines)
- **dashboard.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs`
- **event.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/event.rs` (80 lines)
- **logs_view.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/logs_view.rs`
- **theme.rs** = `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs`

---

## Finding 1: Async `run()` draws unconditionally every frame (CRITICAL)

**File**: app.rs, lines 541-569

```rust
pub async fn run(terminal: &mut Terminal<...>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;          // <-- EVERY iteration
        if crossterm::event::poll(app.refresh_rate)? {
            // handle events
        }
        app.drain_snapshot_channel();
        // ...
    }
}
```

The async `run()` variant calls `terminal.draw()` on every single loop iteration, regardless
of whether any state has changed. With a default `refresh_rate` of 250ms, this caps the
theoretical rate at 4 FPS for the async path, but the draw itself is expensive enough to
further reduce it.

By contrast, the sync `main_loop()` (app.rs:924-1000) correctly tracks a `redraw` flag and
only calls `terminal.draw()` when state has actually changed:

```rust
let mut redraw = snapshot_changed || sys_changed || approval_pending;
// ...tick handling sets redraw...
if redraw {
    terminal.draw(|frame| self.draw(frame))?;
}
```

**Estimated cost**: If the async path is active, this wastes 100% of frames that had no
state change. Even the sync path redraws more often than necessary (see Finding 4).

**Fix**: Port the async `run()` to use the same `redraw` flag pattern as `main_loop()`. Only
redraw when a key event, snapshot change, sys metrics change, or tick-driven animation is
active.

---

## Finding 2: `DashboardSnapshot.clone()` on every snapshot channel drain (CRITICAL)

**File**: app.rs, line 3493

```rust
fn drain_snapshot_channel(&mut self) {
    // ...
    let snapshot = rx.borrow_and_update().clone();   // <-- full deep clone
    apply_dashboard_snapshot(&mut self.tui_state, ..., &snapshot);
}
```

`DashboardSnapshot` (defined in `roko-core/src/dashboard_snapshot.rs:1013`) is a large struct
containing:

- `plans: HashMap<String, PlanState>`
- `tasks: HashMap<String, TaskState>`
- `agents: HashMap<String, AgentState>`
- `gates: Vec<GateVerdictView>` (up to 256)
- `diagnoses: VecDeque<DiagnosisSummary>` (up to 50)
- `episodes: VecDeque<EpisodeSummary>` (up to 128)
- `errors: Vec<ErrorEntry>` (up to 64)
- `event_log: VecDeque<DashboardEventLogEntry>` (up to 200)
- `task_outputs: HashMap<String, VecDeque<String>>`
- `gate_output_lines: VecDeque<String>` (up to 500)
- `token_event_ring: VecDeque<u64>` (up to 120)
- `marketplace_jobs`, `atelier_prds`, `atelier_tasks`, `knowledge_entries`, etc.

Plus opaque JSON blobs (`cascade_router_json`, `gate_thresholds_json`).

The `watch::Receiver::borrow_and_update()` returns a `Ref` that borrows the inner value.
The `.clone()` is needed because `apply_dashboard_snapshot` takes `&mut self` on the same
`App`, so the borrow cannot be held. But this deep-clones the entire snapshot on every
state-hub publication.

**Estimated cost**: With active plans/agents, this is easily several hundred microseconds to
low milliseconds per clone, called on every snapshot tick and also redundantly across
multiple call sites (the sync loop calls `drain_snapshot_channel` up to three times per
iteration: lines 939, 949, 975).

**Fix**:
1. Use `Arc<DashboardSnapshot>` in the watch channel so "cloning" is just an Arc bump.
2. Deduplicate the triple `drain_snapshot_channel` calls in the sync loop -- call it once
   at the top of the tick, not three times.

---

## Finding 3: Bulk `.clone()` cascade in `update_from_snapshot()` (HIGH)

**File**: state.rs, lines 2755-2792

After the snapshot clone, `update_from_snapshot()` performs a second wave of deep clones
from `DashboardData` into `TuiState`:

```rust
self.efficiency_summary = data.efficiency.clone();              // line 2755
self.efficiency_events = data.efficiency_events.clone();         // line 2756 (Vec<AgentEfficiencyEvent>)
self.efficiency_trend = data.efficiency_trend.clone();           // line 2757
self.cfactor_trend_buckets = data.cfactor_trend.clone();         // line 2758
self.cascade_router = data.cascade_router.clone();               // line 2759
self.recent_signals = data.recent_signals.clone();               // line 2760
self.current_plan_execution = data.current_plan_execution.clone(); // line 2761
self.conductor_alerts = data.conductor_alerts.clone();           // line 2762
self.cfactor = data.cfactor.clone();                             // line 2763
self.gate_results_page = data.gate_results_page.clone();         // line 2764
self.experiments = data.experiments.clone();                      // line 2765
self.task_output_tails = data.task_outputs().clone();             // line 2766
self.git_diff = data.git_diff.clone();                           // line 2767
self.plan_summaries = data.plans.clone();                        // line 2768
self.agent_summaries = data.agents.clone();                      // line 2769
self.active_task_summaries = data.active_tasks.clone();          // line 2770
self.gate_result_summaries = data.gate_results.clone();          // line 2771
self.episodes_cache = data.episodes().to_vec();                  // line 2772
self.knowledge_entries = data.knowledge_entries.clone();          // line 2776
self.marketplace_jobs = data.marketplace_jobs.clone();            // line 2790
self.atelier_prds = data.atelier_prds.clone();                   // line 2791
self.atelier_tasks_by_slug = data.atelier_tasks_by_slug.clone(); // line 2792
```

That is 22 separate deep clones of Vec/HashMap collections. The same pattern repeats in
`update_from_dashboard_snapshot()` (lines 2811-3396) with a similar volume.

The connected-mode path (`update_from_dashboard_snapshot`, line 2811) also builds **eight
temporary HashMaps** just to preserve navigation state across updates (lines 2836-2873):

```rust
let prev_plan_order: HashMap<String, usize> = ...collect();      // line 2836
let prev_agent_order: HashMap<String, usize> = ...collect();     // line 2842
let prev_plan_expanded: HashMap<String, bool> = ...collect();    // line 2848
let prev_plan_elapsed: HashMap<String, f64> = ...collect();      // line 2853
let prev_plan_wave: HashMap<String, Option<usize>> = ...collect(); // line 2858
let prev_task_elapsed: HashMap<String, f64> = ...collect();      // line 2863
let prev_agent_rows: HashMap<String, AgentRow> = ...collect();   // line 2868
let prev_route_metrics = self.route_metrics.clone();             // line 2874
```

**Estimated cost**: With 10 plans, 5 agents, 100 efficiency events, and 50 signals, this
is several hundred microseconds of pure allocation + memcpy per update.

**Fix**:
1. Wrap large shareable collections in `Arc<Vec<T>>` and only clone the Arc pointer.
2. Use generation-stamped diffing: skip cloning fields that haven't changed since the
   last update. `DashboardData` already has a `generation` counter; propagate sub-field
   generations.
3. For the eight navigation-preservation HashMaps: keep them as persistent fields on
   `TuiState` and update incrementally, rather than rebuilding from scratch each time.

---

## Finding 4: `build_unified_log_cache()` called on every snapshot update (HIGH)

**File**: state.rs, lines 851-1034; called at line 2773 and line 3396

```rust
pub fn refresh_cached_unified_log(&mut self) {
    self.cached_unified_log = build_unified_log_cache(self);   // line 3407
}
```

Called at the end of both `update_from_snapshot()` (line 2773) and
`update_from_dashboard_snapshot()` (line 3396).

`build_unified_log_cache` is O(N log N) where N = total log entries across five sources:
- `recent_signals` (line 855)
- `episodes_cache` (line 890)
- `efficiency_events` (line 942)
- `gate_results_page.failure_rows` (line 982)
- `event_log` (line 999)

It:
1. Inserts every entry into a `BTreeMap<(i64, usize), LogEntry>` for sorting (line 852)
2. Calls `format!()` to build the `source`, `message`, and `timestamp` for every entry
3. Parses RFC3339 timestamps via `chrono::DateTime::parse_from_rfc3339` for every
   efficiency event (line 943)
4. Collects into `Vec<LogEntry>` (line 1025)
5. Truncates to 10,000 entries (line 1024)

With 200 signals + 128 episodes + 100 efficiency events + 50 gate failures + 200 event
log entries = 678 entries, this is building a BTreeMap, performing ~678 string
format/allocations, and sorting -- every time any snapshot field changes.

The function is also called on the **Logs tab render path** indirectly: `unified_log_entries()`
(line 3401) returns `&self.cached_unified_log`, which is correct (cached). But the cache
is rebuilt every snapshot update regardless of whether the Logs tab is even visible.

**Estimated cost**: ~200-500us per call depending on entry count. Called once per snapshot
update, which can be multiple times per frame.

**Fix**:
1. Generation-stamp each source. Only rebuild the unified cache when a source's generation
   changes.
2. Skip the rebuild entirely if the Logs tab is not active (lazy cache).
3. Replace the `BTreeMap` sort with a merge-sort of pre-sorted sources, which would be
   O(N) instead of O(N log N).
4. Pre-compute and cache the formatted timestamp/source/message strings at ingestion
   time rather than formatting on every rebuild.

---

## Finding 5: PostFX pipeline runs per-cell math on every frame (HIGH)

**File**: postfx_pipeline.rs:18-67, postfx.rs

When `fx_config.screen_postfx` or `fx_config.nerv_viz` or `fx_config.particles` is true
(app.rs:1077), the draw path runs per-cell post-processing on the entire content area.

On a 200x50 terminal (10,000 cells), each frame executes:

1. **`self_glow`** (postfx_pipeline.rs:35): iterates all 10,000 cells, computes luminance,
   conditionally applies additive brightening. Cost: ~10,000 cell reads + conditional writes.

2. **`bloom`** (postfx.rs:639): allocates three `Vec<f64>` of size W*H (= 30,000 f64s =
   240KB), iterates all cells twice (source pass + apply pass), with a nested box-kernel
   loop per bright cell. This is O(W * H * R^2) where R = blur radius. With radius=1,
   that's 4x the cell count in inner-loop iterations. **This alone allocates 240KB per frame.**

3. **`drop_shadow`** (postfx.rs:946): iterates all cells again.

4. **`ambient_orbs`** (postfx.rs:845): iterates for orb placement.

5. **`dream_atmosphere`** (postfx.rs:892): iterates for atmosphere.

6. **`state_viz`** (postfx.rs:458): iterates cells for state visualization.

7. **`particle_overlay`** (postfx.rs:530): per-slot iteration with `is_deep_blank` (a 3x3
   neighborhood check per particle).

8. **`dim_overlay`** (postfx.rs:751): if a modal is open, iterates all cells again to
   multiply every color channel by 0.45 (app.rs:1093).

Total per-frame cost when postfx is enabled: 40,000-80,000+ cell operations plus 240KB of
heap allocation (bloom buffers).

**Estimated cost**: 2-8ms per frame on a typical terminal, depending on which effects are
enabled.

**Fix**:
1. **Skip bloom allocation**: reuse a pre-allocated buffer stored on `App`. The bloom buffers
   are the same size every frame (determined by terminal size); allocating them fresh each
   frame is pure waste.
2. **Dirty-rect optimization**: only run postfx on cells that actually changed since the
   last frame. Ratatui already tracks dirty state internally; expose or replicate this.
3. **Reduce effect frequency**: run heavy effects (bloom, ambient_orbs, dream_atmosphere)
   every Nth frame rather than every frame. At 60 FPS, running them at 15 FPS is visually
   indistinguishable.
4. **Gate on visibility**: skip all postfx when the terminal is not the foreground window
   or when the user is idle (no events for >1s).

---

## Finding 6: `Theme::from_env()` reads environment variables on every draw (LOW)

**File**: app.rs, line 1006; theme.rs, line 144

```rust
fn draw(&mut self, frame: &mut Frame<'_>) {
    let theme = Theme::from_env();   // line 1006
    // ...
}
```

`Theme::from_env()` calls `std::env::var_os("ROKO_HIGH_CONTRAST")` and
`std::env::var_os("NO_COLOR")` on every single draw call (theme.rs:145-148). Environment
variable lookup involves a syscall and mutex on the env block.

**Estimated cost**: ~1-5us per draw. Low absolute cost but entirely avoidable.

**Fix**: Cache the theme on `App` at construction time. Environment variables do not change
at runtime. Add a manual refresh via a keybind if ever needed.

---

## Finding 7: `Vec::remove(0)` on history buffers (O(n) shift) (MEDIUM)

**File**: app.rs, lines 3330 and 3343

```rust
sys.cpu_history.push(cpu_pct);
if sys.cpu_history.len() > 60 {
    sys.cpu_history.remove(0);         // line 3330 -- O(n) shift of 59 elements
}
// ...
sys.mem_history.push(mem_frac);
if sys.mem_history.len() > 60 {
    sys.mem_history.remove(0);         // line 3343 -- O(n) shift of 59 elements
}
```

`SysMetrics` declares `cpu_history` and `mem_history` as `Vec<f32>` (state.rs:1156, 1162).
`Vec::remove(0)` shifts all subsequent elements left, which is O(n). These are bounded to
60 elements, so the shift is small in absolute terms, but it happens on every sys-metrics
drain (every 2 seconds).

Also found at:
- `task_outputs.rs:45` -- `self.items.remove(0)` on a task output buffer
- `app.rs:1787, 2900` -- `self.notifications.remove(0)` on notifications

**Estimated cost**: Negligible individually (~nanoseconds for 60-element shifts), but the
pattern indicates a design intent for ring buffers that should use `VecDeque`.

**Fix**: Change `cpu_history` and `mem_history` from `Vec<f32>` to `VecDeque<f32>`.
Use `push_back()` / `pop_front()` instead of `push()` / `remove(0)`. The process-level
`ProcessMetrics` already correctly uses `VecDeque<f32>` for `cpu_history` and
`VecDeque<u64>` for `mem_history` (state.rs:352-354).

---

## Finding 8: `build_efficiency_snapshot()` called from widget render path (MEDIUM)

**File**: `widgets/token_sparkline.rs:94`

```rust
let file_snap = build_efficiency_snapshot(data);
```

`build_efficiency_snapshot()` (pages/efficiency.rs:39) iterates all efficiency events to:
1. Count distinct task keys via `BTreeSet` collection (line 46-51)
2. Categorize every event by model tier (lines 72-76)
3. Build a token series `Vec<u64>` from every event (lines 78-82)

If this widget is rendered on a visible tab, this computation runs on every frame.

**Estimated cost**: O(N) where N = efficiency event count, with BTreeSet allocation. With
100 events, ~20-50us per call.

**Fix**: Cache the `EfficiencySnapshot` on `TuiState` and recompute only when
`efficiency_events` changes (generation stamp). The render path should read from the cache.

---

## Finding 9: Logs sub-tab 1 clones filtered entries on every frame (LOW)

**File**: logs_view.rs, lines 49-53

```rust
if view_state.sub_tab == 1 {
    let filtered: Vec<LogEntry> = all_entries
        .iter()
        .filter(|e| e.source.starts_with("signal:") || e.source.starts_with("episode:"))
        .cloned()       // <-- clones every matching LogEntry
        .collect();
    render_with_entries(frame, area, &filtered, ...);
}
```

When the Signals sub-tab is active, every matching `LogEntry` (which contains four owned
`String` fields) is cloned into a new `Vec` on every frame. With 200+ matching entries, this
is hundreds of string allocations per frame.

**Estimated cost**: ~10-30us per frame when the Signals sub-tab is active.

**Fix**: Change the filter to collect `&LogEntry` references instead of cloning. The
`render_with_entries` function already accepts `&[LogEntry]`, so pass a
`Vec<&LogEntry>` and adjust the signature to accept a borrowed slice, or cache the
filtered view.

---

## Finding 10: `drain_snapshot_channel()` called 3 times per sync loop iteration (MEDIUM)

**File**: app.rs

In the sync `main_loop` (lines 924-1000), `drain_snapshot_channel()` is called at:
1. Line 939: top of every iteration
2. Line 949: after key handling (inside `Event::Key` match arm)
3. Line 975: inside `Event::Tick` match arm

The second and third calls are redundant for the common tick case. The key-handling path
also calls `drain_background_channels()` (line 951), which internally calls
`drain_snapshot_channel()` again (line 3316), making it **four** calls per key event.

Each call does `rx.has_changed()` (cheap) but when a change is present, the full
`DashboardSnapshot.clone()` + `apply_dashboard_snapshot()` runs.

**Estimated cost**: The has_changed() check is ~nanoseconds, but if a snapshot arrives during
a key event, it gets cloned and applied up to 4 times in one iteration.

**Fix**: Call `drain_snapshot_channel()` exactly once at the top of each loop iteration.
Remove the redundant calls from the key and tick handlers.

---

## Finding 11: `apply_git_bg_data` clones git view data redundantly (LOW)

**File**: app.rs, lines 3467-3481

```rust
fn apply_git_bg_data(&mut self, bg: GitBgData) {
    self.tui_state.git_branch_tree = bg.view_data.branches.clone();    // clone
    self.tui_state.git_commit_graph = convert_git_commit_graph(&bg.view_data.commits); // iterate+clone
    self.tui_state.git_worktree_list = convert_git_worktree_list(&bg.view_data.worktrees); // iterate+clone
    self.tui_state.git_view_data = Some(bg.view_data);   // move
    // ...
}
```

`bg.view_data.branches` is cloned (line 3468) and then `bg.view_data` itself is moved into
`git_view_data` (line 3471). The branches clone is unnecessary since the moved `view_data`
already contains them. `git_branch_tree` and `git_view_data.branches` are the same data
duplicated.

The git collection itself (`collect_git_bg_data`, app.rs:217) runs on a background thread
and is properly non-blocking. The only waste is the redundant clone at apply time.

**Estimated cost**: ~5-20us depending on branch/commit count. Triggered only on git
watcher events, not per-frame.

**Fix**: Extract branches/commits/worktrees from `view_data` before the move using
`std::mem::take`, or reference them through the stored `git_view_data` instead of
maintaining separate copies.

---

## Finding 12: No frame budget / adaptive tick rate (ARCHITECTURAL)

**File**: event.rs, lines 50-78; app.rs, line 521

The tick rate is configured once at startup (default 250ms, configurable 50ms-5000ms in
`roko.toml`). The event loop does not measure how long a draw takes and cannot adapt.

If a draw takes 200ms (as implied by 2.1 FPS) and the tick rate is 250ms, the effective
frame time is 450ms (200ms draw + 250ms poll timeout = ~2.2 FPS). The poll timeout does
not account for draw time.

**Estimated cost**: The 250ms default tick rate alone caps the theoretical maximum at ~4
FPS even with zero-cost draws.

**Fix**:
1. Measure draw duration. Subtract it from the next poll timeout so the total frame time
   stays constant.
2. Use an adaptive tick rate: when the system is idle (no animations, no active agents),
   use a long tick (500ms+). When animations are active, use a short tick (16ms for 60
   FPS).
3. The sync `main_loop` already has animation detection (line 967-974); wire this into
   the `EventHandler` tick rate via `set_tick_rate()`.

---

## Summary: Estimated Impact per Fix

| # | Finding | Est. per-frame cost | Fix difficulty |
|---|---------|-------------------|----------------|
| 1 | Async `run()` unconditional draw | 100% waste on no-change frames | Easy |
| 2 | DashboardSnapshot.clone() | 0.5-2ms | Medium (Arc) |
| 3 | 22x bulk .clone() in update_from_snapshot | 0.2-0.5ms | Medium |
| 4 | build_unified_log_cache every update | 0.2-0.5ms | Easy (generation stamp) |
| 5 | PostFX per-cell math + 240KB bloom alloc | 2-8ms | Medium |
| 6 | Theme::from_env() per draw | ~5us | Trivial |
| 7 | Vec::remove(0) on history buffers | ~ns | Trivial |
| 8 | build_efficiency_snapshot in render path | 20-50us | Easy |
| 9 | Logs sub-tab clones filtered entries | 10-30us | Easy |
| 10 | drain_snapshot 3-4x per iteration | up to 4x Finding 2 | Easy |
| 11 | Redundant git data clone | 5-20us | Trivial |
| 12 | No adaptive tick rate | caps at ~4 FPS | Medium |

**Recommended priority order**:
1. Fix #12 (adaptive tick rate) + Fix #1 (conditional draw) -- these are the frame-rate
   ceiling; without them, nothing else matters.
2. Fix #5 (bloom buffer reuse + effect throttling) -- eliminates the largest per-frame
   allocation.
3. Fix #2 + #10 (Arc snapshot + deduplicate drains) -- eliminates the largest per-update
   clone.
4. Fix #3 + #4 (generation-stamped incremental updates) -- eliminates redundant
   collection rebuilds.
5. Everything else (#6-9, #11) as cleanup.
