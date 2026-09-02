# TUI Render Loop Performance Audit

**Date**: 2026-09-01
**Observed FPS**: ~2.1 FPS (target: 60+ FPS)
**Primary file**: `crates/roko-cli/src/tui/app.rs`

---

## 1. Architecture of the Render Loop

The TUI has two entry points into the event loop, both in `app.rs`:

### 1a. Async `run()` — connected mode (line 541)

```rust
// app.rs:541
pub async fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| app.draw(f))?;                     // <-- DRAWS EVERY ITERATION
        if crossterm::event::poll(app.refresh_rate)? {        // <-- BLOCKS for refresh_rate
            match crossterm::event::read()? { ... }
        }
        if app.pending_refresh {
            app.refresh_snapshot_async().await;               // <-- DISK I/O IN LOOP
        }
        app.drain_snapshot_channel();
        app.drain_approval_requests();
    }
}
```

This path draws unconditionally every loop iteration, then blocks on
`crossterm::event::poll(app.refresh_rate)`. The `refresh_rate` defaults to
**250ms** (see below), meaning this path can never exceed ~4 FPS even with
zero rendering cost.

### 1b. Sync `main_loop()` — standalone mode (line 853)

```rust
// app.rs:853-1003
fn main_loop(&mut self, terminal: &mut TuiTerminal) -> Result<()> {
    let mut events = EventHandler::new(self.refresh_rate);
    // ...
    while self.running {
        self.drain_shutdown_signal();
        let snapshot_changed = ...;
        let sys_changed = ...;
        let approval_pending = ...;
        self.drain_snapshot_channel();
        let mut redraw = snapshot_changed || sys_changed || approval_pending;

        match events.next()? {                               // <-- BLOCKS for tick_rate
            Event::Key(key) => {
                self.handle_key(key);
                self.drain_background_channels();
                terminal.draw(|frame| self.draw(frame))?;    // <-- immediate redraw
                continue;
            }
            Event::Tick => {
                let animated = ...active agents/plans/modals/notifications...;
                if animated { self.tui_state.atmosphere.tick(); redraw = true; }
                self.drain_background_channels();             // <-- may do disk I/O
            }
            ...
        }

        redraw |= self._state_hub.is_none() && last_draw.elapsed() >= Duration::from_secs(1);
        if redraw {
            terminal.draw(|frame| self.draw(frame))?;
        }
    }
}
```

The `EventHandler::next()` blocks until either an input event arrives or the
tick timeout expires. The tick timeout is `self.refresh_rate`.

---

## 2. Identified Bottlenecks

### BOTTLENECK 1 (CRITICAL): Default refresh_rate is 250ms = 4 FPS ceiling

**Severity**: CRITICAL — this alone explains the 2.1 FPS observation
**Location**: `app.rs:520-538`

```rust
fn configured_tui_refresh_rate(workdir: &Path) -> Duration {
    const DEFAULT_MS: u64 = 250;    // <-- 4 FPS ceiling
    const MIN_MS: u64 = 50;         // <-- 20 FPS ceiling even at minimum
    const MAX_MS: u64 = 5_000;
    // ...reads from roko.toml [tui] refresh_rate_ms...
    Duration::from_millis(configured)
}
```

The default `refresh_rate` of 250ms is used as the `crossterm::event::poll()`
timeout in both render loops. When no input events arrive, the TUI sleeps for
up to 250ms between frames. This is the **hard FPS ceiling**.

The `EventHandler` (event.rs:50-78) uses this same value:

```rust
pub fn next(&mut self) -> io::Result<Event> {
    loop {
        let timeout = if elapsed >= self.tick_rate {
            Duration::ZERO
        } else {
            self.tick_rate - elapsed          // <-- up to 250ms blocking
        };
        if event::poll(timeout)? { ... }
        return Ok(Event::Tick);
    }
}
```

**Why 2.1 FPS, not 4 FPS?** The remaining ~1.9 FPS gap comes from actual
rendering and drain work consuming time within each cycle, pushing the
effective period from 250ms to ~475ms.

**Fix**:
```rust
// For 60 FPS: use ~16ms tick rate
const DEFAULT_MS: u64 = 16;     // 60 FPS
const MIN_MS: u64 = 8;          // 120 FPS max
const MAX_MS: u64 = 250;        // low-power cap

// Better: use adaptive tick rate
fn adaptive_tick_rate(&self) -> Duration {
    let animated = self.tui_state.agents.iter().any(|a| a.active)
        || self.tui_state.plans.iter().any(|p| p.active)
        || self.has_modal()
        || !self.notifications.is_empty();
    if animated {
        Duration::from_millis(16)    // 60 FPS when animating
    } else {
        Duration::from_millis(100)   // 10 FPS when idle (saves CPU)
    }
}
```

**Expected improvement**: 2.1 FPS -> 30-60 FPS (depending on render cost)


### BOTTLENECK 2 (CRITICAL): Async `run()` draws unconditionally every frame

**Severity**: CRITICAL
**Location**: `app.rs:546-547`

```rust
loop {
    terminal.draw(|f| app.draw(f))?;           // ALWAYS draws
    if crossterm::event::poll(app.refresh_rate)? { ... }
}
```

The async `run()` path calls `terminal.draw()` on **every** loop iteration,
regardless of whether anything changed. The sync `main_loop()` already has a
`redraw` guard — the async path does not.

**Fix**: Add a dirty flag, matching the sync path's pattern:
```rust
loop {
    let snapshot_changed = app.snapshot_rx.as_ref()
        .is_some_and(|rx| rx.has_changed().unwrap_or(false));
    let mut redraw = snapshot_changed;

    if crossterm::event::poll(app.refresh_rate)? {
        match crossterm::event::read()? {
            // ...handle input...
            _ => {}
        }
        redraw = true;
    }

    app.drain_snapshot_channel();
    app.drain_approval_requests();

    if app.pending_refresh { ... redraw = true; }
    if redraw {
        terminal.draw(|f| app.draw(f))?;
    }
}
```

**Expected improvement**: 20-40% reduction in CPU use (avoids redundant
full-buffer diffs when nothing changed)


### BOTTLENECK 3 (HIGH): PostFX pipeline iterates every cell on every frame

**Severity**: HIGH
**Location**: `postfx_pipeline.rs:18-67`, `postfx.rs:150-184` (self_glow),
`postfx.rs:639-705` (bloom), `postfx.rs:751-773` (dim_overlay),
`postfx.rs:892-918` (dream_atmosphere)

With default `Minimal` effects preset, every frame runs:

1. **`self_glow()`** (postfx_pipeline.rs:35-36): iterates `width * height`
   cells, reads fg color, computes luminance, applies brightening. For a
   200x50 terminal, that is 10,000 cells of floating-point math per frame.

2. When modals are active, **`dim_overlay()`** (app.rs:1092-1094) iterates the
   entire content area again (another 10,000 cells), reading and scaling fg
   and bg colors through floating-point multiplication.

3. When `Full` preset is active:
   - **`bloom()`** (postfx.rs:639-705): allocates 3x `Vec<f64>` of
     `width*height` each (240KB for 200x50), then iterates all cells twice
     (source pass + apply pass). This is O(w*h*r^2) where r=radius.
   - **`dream_atmosphere()`** (postfx.rs:892-918): iterates all cells with an
     LCG PRNG, computing grain noise per-cell + breathing modulation.
   - **`vignette()`** (postfx.rs:710-747): iterates all cells, computing
     Euclidean distance with `powi(2)` and `sqrt()` per cell.
   - **`ambient_orbs()`** (postfx.rs:845-887): 3 orbs x 9 cells each (cheap).

4. When `particles` is enabled (default `Minimal`), **`particle_overlay()`**
   (postfx.rs:530-603) iterates up to 24 particle slots with distance checks
   via `is_deep_blank()` which itself checks a 3x3 neighborhood.

**Cost per frame at `Minimal` preset**: ~10,000 cell reads + ~200 color writes
for self_glow, plus ~24 particle slots (negligible). Total: ~0.5-2ms.

**Cost per frame at `Full` preset**: ~50,000+ cell operations including three
full-area passes, bloom allocation, and sqrt/trig per cell. Total: ~5-15ms.

**Fix** (immediate):
```rust
// Skip postfx entirely when effects preset is Off
if matches!(self.fx_config.preset, EffectsPreset::Off) {
    // Skip the postfx_pipeline::apply_pipeline call entirely
}
```

**Fix** (for Full mode): Pre-compute bloom source map, use integer-only
luminance, avoid per-frame Vec allocations in bloom by reusing buffers:
```rust
struct PostFxBuffers {
    bloom_r: Vec<f64>,
    bloom_g: Vec<f64>,
    bloom_b: Vec<f64>,
}

impl PostFxBuffers {
    fn resize(&mut self, area: usize) {
        self.bloom_r.resize(area, 0.0);
        self.bloom_g.resize(area, 0.0);
        self.bloom_b.resize(area, 0.0);
        // zero out
        self.bloom_r.fill(0.0);
        self.bloom_g.fill(0.0);
        self.bloom_b.fill(0.0);
    }
}
```

**Expected improvement**: 1-15ms per frame saved (0.5ms for Minimal, 5-15ms
for Full). With a 16ms frame budget, that is 3-94% of the frame.


### BOTTLENECK 4 (HIGH): `Theme::from_env()` called every frame

**Severity**: HIGH
**Location**: `app.rs:1006`

```rust
fn draw(&mut self, frame: &mut Frame<'_>) {
    let theme = Theme::from_env();     // <-- CALLED EVERY FRAME
    // ...
}
```

`Theme::from_env()` (theme.rs:144-152) calls `std::env::var_os()` twice per
invocation:

```rust
pub fn from_env() -> Self {
    if std::env::var_os("ROKO_HIGH_CONTRAST").is_some() { ... }
    else if std::env::var_os("NO_COLOR").is_some() { ... }
    else { Self::dark() }
}
```

`std::env::var_os()` acquires a process-wide lock on the environment
variables. At 60 FPS, this is 120 lock acquisitions per second for values that
effectively never change during a TUI session.

**Fix**: Cache the theme once at App construction:
```rust
pub struct App {
    theme: Theme,  // <-- new field
    // ...
}

// In App::new_with_page_inner:
theme: Theme::from_env(),

// In draw():
fn draw(&mut self, frame: &mut Frame<'_>) {
    let theme = &self.theme;
    // ...
}
```

**Expected improvement**: ~0.1ms per frame saved (minor but zero-cost fix)


### BOTTLENECK 5 (HIGH): `drain_snapshot_channel` clones the entire `DashboardSnapshot`

**Severity**: HIGH (when connected to a live runner)
**Location**: `app.rs:3487`

```rust
fn drain_snapshot_channel(&mut self) {
    // ...
    let snapshot = rx.borrow_and_update().clone();   // <-- FULL CLONE
    apply_dashboard_snapshot(&mut self.tui_state, ..., &snapshot);
}
```

`DashboardSnapshot` (roko-core/src/dashboard_snapshot.rs:1013) contains:
- `HashMap<String, PlanState>` — plans with full task lists
- `HashMap<String, TaskState>` — all active tasks
- `HashMap<String, AgentState>` — all agents
- `Vec<GateVerdictView>` — up to 256 gate verdicts
- `VecDeque<DiagnosisSummary>` — up to 50 diagnoses
- Multiple other vecs and strings

A clone of this struct during an active run with 10 plans and 50 tasks could
be 10-50KB of heap allocation and copy. The `watch::Receiver::borrow_and_update()`
provides a read guard; the clone is done to release the guard before the long
`apply_dashboard_snapshot` call.

**Fix**: Use `Arc<DashboardSnapshot>` in the watch channel, or apply directly
from the borrow guard:
```rust
fn drain_snapshot_channel(&mut self) {
    let Some(rx) = self.snapshot_rx.as_mut() else { return; };
    if !rx.has_changed().unwrap_or(false) { return; }

    // Borrow and apply directly without cloning
    let snapshot = rx.borrow_and_update();
    apply_dashboard_snapshot(
        &mut self.tui_state,
        &mut self.notifications,
        &mut self.last_snapshot_error_marker,
        &mut self.last_seen_gate_count,
        &mut self.last_seen_plan_phases,
        &snapshot,
    );
    let snapshot_ref = snapshot.clone(); // only if needed for update_plan_completion_exit
    drop(snapshot);
    self.update_plan_completion_exit(&snapshot_ref);
}
```

Or better, restructure `update_plan_completion_exit` to accept the data it
needs rather than the full snapshot, avoiding the clone entirely.

**Expected improvement**: 0.5-5ms per update saved (depends on snapshot size;
occurs only when snapshot channel fires, not every frame)


### BOTTLENECK 6 (MEDIUM): `drain_snapshot_channel` called multiple times per iteration

**Severity**: MEDIUM
**Location**: `app.rs:939, 949, 975` (sync main_loop)

In the sync `main_loop`, `drain_snapshot_channel()` is called up to **three
times** per loop iteration:

```rust
self.drain_snapshot_channel();                    // line 939 (pre-event)
match events.next()? {
    Event::Key(key) => {
        self.drain_snapshot_channel();            // line 949 (post-key)
        // ...
    }
    Event::Tick => {
        self.drain_snapshot_channel();            // line 975 (tick)
        // ...
    }
}
```

Additionally, `drain_background_channels()` (line 3316) calls
`drain_snapshot_channel()` internally, leading to a fourth call on tick:

```rust
fn drain_background_channels(&mut self) {
    self.drain_snapshot_channel();                // line 3316
    // ...
}
```

The function guards with `has_changed()`, so redundant calls are cheap (~ns
for the watch check). But the pattern obscures the flow and the first call at
line 939 runs **before** checking whether the event loop should even draw.

**Fix**: Consolidate to a single drain point per iteration:
```rust
while self.running {
    match events.next()? {
        Event::Key(key) => { self.handle_key(key); redraw = true; }
        Event::Tick => { /* animation logic */ }
        // ...
    }
    // Single drain point
    self.drain_all_channels();
    if redraw { terminal.draw(...)?; }
}
```

**Expected improvement**: Negligible FPS improvement, but cleaner control flow


### BOTTLENECK 7 (MEDIUM): `Vec::remove(0)` for history buffers

**Severity**: MEDIUM
**Location**: `app.rs:3329-3330, 3342-3343`

```rust
sys.cpu_history.push(cpu_pct);
if sys.cpu_history.len() > 60 {
    sys.cpu_history.remove(0);       // <-- O(n) shift of 59 elements
}

sys.mem_history.push(mem_frac);
if sys.mem_history.len() > 60 {
    sys.mem_history.remove(0);       // <-- O(n) shift of 59 elements
}
```

`Vec::remove(0)` is O(n) because it shifts all remaining elements left. For a
60-element history, this copies 59 `f32` values (236 bytes) per update. The
`SysMetrics` struct in `state.rs:1156` declares these as `Vec<f32>`.

Interestingly, the `TuiState` counterpart uses `VecDeque` (state.rs:352-354):
```rust
pub cpu_history: VecDeque<f32>,
pub mem_history: VecDeque<u64>,
```

The fix is already half-present but not applied to the `SysMetrics` struct.

**Fix**: Use `VecDeque` or the existing `push_bounded_history` helper
(app.rs:3928):

```rust
// Already exists at app.rs:3928
fn push_bounded_history<T>(history: &mut VecDeque<T>, value: T, max_len: usize) {
    if history.len() >= max_len { history.pop_front(); }
    history.push_back(value);
}

// Change SysMetrics to use VecDeque<f32>
pub struct SysMetrics {
    pub cpu_history: VecDeque<f32>,
    pub mem_history: VecDeque<f32>,
    // ...
}
```

**Expected improvement**: Negligible at 60 elements, but prevents a scaling
issue if history length increases


### BOTTLENECK 8 (MEDIUM): `refresh_snapshot()` reloads config and TOML on every call

**Severity**: MEDIUM
**Location**: `app.rs:2968-2997`

```rust
fn refresh_snapshot(&mut self) {
    if self.replay_disk_snapshots || self._state_hub.is_none() {
        self.data = DashboardData::load_best_effort(&self.workdir);  // HEAVY DISK I/O
        self.scaffold = DashboardScaffold::new_in(&self.workdir);    // more disk I/O
        // ...
        if let Some(state_hub) = &self._state_hub {
            let _ = state_hub.bootstrap_from_workdir(&self.workdir); // FULL re-bootstrap
            state_hub.replay_log_into_snapshot(&events_path);        // full log replay
        }
    }
    self.fx_config = EffectsConfig::load_from_root(&self.workdir);   // reads roko.toml
    // ...
}
```

`DashboardData::load_best_effort()` (dashboard.rs:447) reads and parses:
- `state-snapshot.json` (JSON parse of executor state)
- `engrams.jsonl` (signal log, potentially megabytes)
- `episodes.jsonl` (episode log)
- `events.json` (event log)
- `efficiency.jsonl`, `c-factor.jsonl` (learning data)
- `roko.toml` (config parse)

This is called via `pending_refresh` flag from `dispatch_action`, which fires
on certain key inputs. It is **not** called every frame, but when it fires, it
can block the render loop for 50-500ms depending on log file sizes.

**Fix**: The incremental RC-6 path at lines 3386-3418 already addresses this
for the filesystem watcher trigger. Ensure `pending_refresh` also uses the
incremental path:
```rust
if self.pending_refresh {
    self.pending_refresh = false;
    self.incremental_refresh();  // not full reload
}
```

**Expected improvement**: 50-500ms per trigger avoided (not every frame, but
eliminates visible stalls)


### BOTTLENECK 9 (MEDIUM): `EffectsConfig::load_from_root()` reads TOML every refresh

**Severity**: MEDIUM
**Location**: `app.rs:2950, 2982`, `effects_config.rs:136-164`

```rust
// Called from refresh_snapshot / refresh_snapshot_async
self.fx_config = EffectsConfig::load_from_root(&self.workdir);
```

This reads and parses `roko.toml` from disk. Not per-frame (only on explicit
refresh), but redundant: the effects config is already managed in memory and
persisted only when the user cycles presets via Ctrl-E.

**Fix**: Only reload effects config when the filesystem watcher detects
`roko.toml` changes, not on every snapshot refresh.

**Expected improvement**: 1-5ms per refresh saved


### BOTTLENECK 10 (LOW): `notifications.remove(0)` in `expire_notifications()`

**Severity**: LOW
**Location**: `app.rs:2900`

```rust
fn expire_notifications(&mut self) {
    self.notifications.retain(|n| !n.is_expired());
    while self.notifications.len() > MAX_NOTIFICATIONS {
        self.notifications.remove(0);    // O(n) per removal
    }
}
```

Same O(n) pattern as the history buffers. Capped at 20, so maximum cost is
~20 shifts, but could be a `VecDeque` for O(1) removal from the front.

**Expected improvement**: Negligible


### BOTTLENECK 11 (LOW): `current_view_state()` clones filter string every frame

**Severity**: LOW
**Location**: `app.rs:3213-3260`

```rust
fn current_view_state(&self) -> ViewState {
    match self.tui_state.active_tab {
        Tab::Dashboard => ViewState {
            search_query: self.tui_state.filter.clone(),   // <-- clone per frame
            // ...
        },
        Tab::Plans => ViewState {
            search_query: self.tui_state.filter.clone(),   // <-- clone per frame
            // ...
        },
        // 8 more tabs, each cloning the filter string
    }
}
```

The `ViewState` is constructed fresh every frame. If `filter` is an `Arc<str>`
or a `Cow<'_, str>`, the clone cost drops to a pointer copy.

**Fix**: Use `&str` reference in `ViewState` (lifetime from `&self`), or make
`filter` an `Arc<String>`.

**Expected improvement**: Negligible for short filters; prevents a scaling
issue with long search queries.

---

## 3. Non-Bottleneck Observations (Already Correct)

### Background sys metrics: correctly off the render thread

The `collect_sys_metrics_bg()` function (app.rs:3824-3868) runs on a dedicated
background thread with a 2-second sleep. It communicates via a `watch::Sender`
which the render thread drains non-blockingly via `has_changed()`. This is
correct and does not block rendering.

### Filesystem and git watchers: correctly debounced

`fs_watch.rs` uses a 200ms debounce window with `notify`. `git_watch.rs` uses
a 500ms debounce window. Both communicate via bounded `sync_channel(4)` and
are drained with `try_recv()` (non-blocking). Git data collection is spawned
onto a background thread. All correct.

### `drain_snapshot_channel`: correctly guarded

The `has_changed()` check (app.rs:3483) ensures the function is a no-op when
no new data has arrived. The clone is the only cost, and it only fires when the
snapshot actually changes.

### Agent stream clients: correctly lazily connected

`sync_agent_stream_clients()` (app.rs:3602) only activates when the Agents tab
is selected, and disconnects when switching away. The `drain_agent_stream_clients()`
uses `try_recv()` (non-blocking).

---

## 4. Priority-Ordered Fix List

| Priority | Fix | Expected FPS Gain | Effort |
|---|---|---|---|
| **P0** | Change default `refresh_rate` from 250ms to 16ms (or adaptive) | 2 FPS -> 30-60 FPS | Trivial (one constant) |
| **P1** | Add redraw guard to async `run()` path | -20-40% CPU | Small |
| **P2** | Cache `Theme::from_env()` once at App construction | ~0.1ms/frame saved | Trivial |
| **P3** | Avoid `DashboardSnapshot` clone (borrow-and-apply) | 0.5-5ms per update | Medium |
| **P4** | Pre-allocate PostFX bloom buffers (reuse across frames) | 5-15ms/frame on Full | Medium |
| **P5** | Make `refresh_snapshot()` incremental when called via `pending_refresh` | Eliminates 50-500ms stalls | Medium |
| **P6** | Change `SysMetrics` history from `Vec` to `VecDeque` | O(1) vs O(n) per update | Trivial |
| **P7** | Consolidate `drain_snapshot_channel` calls | Code clarity | Trivial |
| **P8** | Stop reloading `roko.toml` for effects on every refresh | 1-5ms per refresh | Trivial |
| **P9** | Use `&str` or `Arc` for `ViewState::search_query` | Negligible | Trivial |

---

## 5. The 60 FPS Fix (Minimum Viable)

The single most impactful change is **P0**: reducing the `event::poll()` timeout.
This is a one-line change:

```rust
// app.rs:521
const DEFAULT_MS: u64 = 16;  // was 250
```

However, a 16ms poll rate with the current rendering pipeline will increase CPU
usage significantly because the postfx pipeline runs on every redraw. The ideal
approach combines P0 with an adaptive rate:

```rust
fn configured_tui_refresh_rate(workdir: &Path) -> Duration {
    // Base rate from config (still respected)
    let configured = read_config_rate(workdir).unwrap_or(16).clamp(8, 250);
    Duration::from_millis(configured)
}

// In main_loop, dynamically adjust:
fn effective_tick_rate(&self) -> Duration {
    let animated = self.tui_state.agents.iter().any(|a| a.active)
        || self.tui_state.plans.iter().any(|p| p.active)
        || self.has_modal()
        || !self.notifications.is_empty()
        || self.fx_config.particles
        || self.fx_config.nerv_viz;

    if animated {
        Duration::from_millis(16)    // 60 FPS when actively animating
    } else {
        Duration::from_millis(100)   // 10 FPS when idle (saves CPU)
    }
}
```

And update the event handler's tick rate dynamically:
```rust
// In main_loop, before events.next():
events.set_tick_rate(self.effective_tick_rate());
```

The `EventHandler` already has `set_tick_rate()` (event.rs:45-47), so this is
ready to wire.

---

## 6. Frame Budget at 60 FPS

At 60 FPS, each frame has a 16.6ms budget:

| Component | Cost (Minimal preset) | Cost (Full preset) |
|---|---|---|
| `event::poll()` + drain | ~0.1ms | ~0.1ms |
| `draw()` — layout + widgets | ~2-4ms | ~2-4ms |
| `draw()` — self_glow postfx | ~0.5ms | ~0.5ms |
| `draw()` — bloom postfx | N/A | ~5-10ms |
| `draw()` — dream_atmosphere | N/A | ~2-4ms |
| `draw()` — particles | ~0.2ms | ~0.2ms |
| `draw()` — dim_overlay (modal) | ~0.3ms | ~0.3ms |
| `terminal.draw()` diff + flush | ~1-2ms | ~1-2ms |
| **Total** | **~4-7ms** | **~10-21ms** |

At `Minimal` preset (default), the render pipeline fits comfortably in 16.6ms.
At `Full` preset, bloom is the main risk — it may push frames over budget on
large terminals.

---

## 7. Conclusion

The TUI's 2.1 FPS is not caused by expensive rendering. The rendering pipeline
takes ~4-7ms per frame at default settings — well within a 60 FPS budget. The
bottleneck is the **250ms poll timeout** that gates the entire event loop. A
single constant change (P0) would bring the TUI from 2.1 FPS to 30-60 FPS. The
remaining fixes (P1-P9) address CPU efficiency and occasional stalls but are not
required for the target frame rate.

---

## Implementation Status (2026-09-02 swarm)

`Theme::from_env()` per-frame rebuild eliminated (task #1): theme instance cached in App
struct, called once at startup instead of every frame. p95 draw benchmark remains open.
