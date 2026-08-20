# Mori TUI Architecture -- Complete Reference

> Source: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/`
> Total: ~10,300 LOC across 47 Rust files (18 modules, 27 widgets, 13 modals, 7 views)
> Framework: ratatui + crossterm
> Date: 2026-08-19

---

## 1. File Structure

### Root Module (`tui/mod.rs` -- 56 lines)

The entry point re-exports 17 submodules and provides two critical functions:

```rust
pub fn init() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>>  // raw mode + alt screen + mouse
pub fn restore() -> io::Result<()>                                    // undo init()
pub fn truncate_chars(s: &str, max_chars: usize, suffix: &str) -> String
```

Uses `crossterm::{EnableMouseCapture, EnterAlternateScreen, enable_raw_mode}`.

### Complete File Inventory

```
tui/
  mod.rs               -- 56 LOC   -- Terminal init/restore, module declarations
  atmosphere.rs         -- 284 LOC  -- Particle system, heartbeat, breathing, spinners, FPS
  bars.rs               -- ~150 LOC -- gradient_bar(), semantic_color() helpers
  color.rs              -- 180 LOC  -- HSV-to-RGB, Gradient LUT, named gradients (fire/ocean/ember/sage/amber/context)
  effects_config.rs     -- 92 LOC   -- EffectsConfig with 9 toggleable VFX booleans
  hit_test.rs           -- ~120 LOC -- HitZones struct for mouse click resolution
  input.rs              -- ~700 LOC -- Key binding dispatch, TuiAction enum, InputMode handling
  layout.rs             -- 362 LOC  -- Top-level render(): header | alert | tab_content | status_bar, modal overlay, bg viz
  math.rs               -- 141 LOC  -- Vec2, easing functions, wave combinators
  nerv_viz.rs           -- 356 LOC  -- State-driven data visualizations (progress_field, activity_ripples, data_rain, state_viz)
  postfx.rs             -- 445 LOC  -- Bloom, vignette, dim_overlay, modal_glow, ambient_orbs, dream_atmosphere, amber_color_grade, drop_shadow
  postfx_pipeline.rs    -- 46 LOC   -- Per-tab postfx routing (self_glow for Dashboard/Agents/Plans)
  tabs.rs               -- ~80 LOC  -- Tab enum + from_index mapping
  theme.rs              -- 266 LOC  -- ROSEDUST color palette, semantic styles, role/phase accent colors, gradient constructors
  vfx.rs                -- 105 LOC  -- Pure-math field generators (plasma, noise, smooth_noise, fbm, voronoi, ripple)

  views/
    mod.rs              -- 7 LOC    -- Declares 6 view submodules
    dashboard.rs        -- 435 LOC  -- F1:Dash master-detail + sub-tab bar + agents_content + bottom_strip
    plans.rs            -- ~900 LOC -- F2:Plans two-column wave tree + plan detail
    agents.rs           -- ~100 LOC -- F3:Agents agent grid view
    git_view.rs         -- ~300 LOC -- F4:Git worktree list + branch diff + commit log
    logs.rs             -- ~200 LOC -- F5:Logs scrollable log viewer with level-based coloring
    config.rs           -- ~250 LOC -- F6:Config two-column: config sections + MCP summary
    context.rs          -- ~300 LOC -- F7:Inspect MCP/AST/learning/fixtures inspection

  widgets/
    mod.rs              -- 27 LOC   -- Declares all 27 widget submodules
    header_bar.rs       -- 433 LOC  -- Top bar: heartbeat + wave + progress bar + ETA + cost + tokens + MCP + sys metrics + spinner + F-keys
    status_bar.rs       -- 221 LOC  -- Bottom bar: git branch + commit + heartbeat + progress + health + merge + keybind hints
    plan_tree.rs        -- 1078 LOC -- Collapsible Wave->Plan tree with inline task progress columns
    plan_list.rs        -- ~100 LOC -- Flat plan list (simpler alternative to plan_tree)
    agent_grid.rs       -- 122 LOC  -- Grid of agent cards (1-3 cols, auto-layout)
    agent_output.rs     -- ~900 LOC -- Agent output viewer with per-agent tabs, parallel agent dynamic tabs
    agent_pool.rs       -- 227 LOC  -- Agent pool summary with inline context gauges + sparklines
    parallel_pool.rs    -- ~200 LOC -- Parallel agent pool (multi-plan variant of agent_pool)
    diff_panel.rs       -- ~150 LOC -- Git diff viewer with +/- coloring
    error_digest.rs     -- ~200 LOC -- Structured error digest grouped by file
    command_output.rs   -- ~150 LOC -- Gate/compiler output with PASS/FAIL badges
    context_gauge.rs    -- ~100 LOC -- Horizontal context gauge with 80%/90% threshold markers
    token_sparkline.rs  -- 195 LOC  -- Multi-row braille sparkline: aggregate + per-agent token burn
    token_bar.rs        -- ~50 LOC  -- Simple inline token bar
    sys_metrics.rs      -- 296 LOC  -- CPU/MEM/NET/DSK/FPS with braille sparklines + animated gauge fills + top procs
    braille.rs          -- 78 LOC   -- Braille sparkline rendering (2x horizontal density)
    scrollbar.rs        -- ~60 LOC  -- Proportional scrollbar indicator
    phase_bar.rs        -- ~80 LOC  -- Pipeline phase progress bar
    phase_compact.rs    -- ~100 LOC -- Compact phase indicator for left panel
    phase_timeline.rs   -- ~100 LOC -- Timeline-style phase visualization
    task_progress.rs    -- ~300 LOC -- Task checklist viewer with per-task status
    wave_bar.rs         -- ~80 LOC  -- Single-row wave progress indicator
    wave_progress.rs    -- ~100 LOC -- Wave progress ribbon (top of dashboard)
    branch_tree.rs      -- ~100 LOC -- Git branch tree visualization
    status_badge.rs     -- ~50 LOC  -- Inline status badges (pass/fail/active)
    tab_bar.rs          -- ~80 LOC  -- Tab selection bar

  modals/
    mod.rs              -- 13 LOC   -- Declares 13 modal submodules
    approval.rs         -- ~150 LOC -- Plan approval prompt (y/n)
    help.rs             -- ~200 LOC -- Keybind reference overlay (?)
    inject.rs           -- ~100 LOC -- Message injection input (i)
    plan_detail.rs      -- ~200 LOC -- Plan detail overlay (Enter from plan tree)
    task_detail.rs      -- ~150 LOC -- Task detail overlay
    task_picker.rs      -- ~150 LOC -- Ctrl-t task picker
    wave_overview.rs    -- ~150 LOC -- Wave overview modal
    agent_pool_modal.rs -- ~150 LOC -- Agent pool detail modal
    queue_overview.rs   -- ~200 LOC -- F8: Queue overview
    batch_review.rs     -- ~150 LOC -- Batch review modal
    confirm.rs          -- ~80 LOC  -- Generic confirmation dialog
    notification.rs     -- ~80 LOC  -- Toast notification overlay (up to 3, auto-dismiss)
    quit.rs             -- ~50 LOC  -- Quit confirmation
```

---

## 2. Tab System (F1--F8)

### Tab Enum (`tabs.rs`)

```rust
pub enum Tab {
    Dashboard, // F1 -- index 0
    Plans,     // F2 -- index 1
    Agents,    // F3 -- index 2
    Git,       // F4 -- index 3
    Logs,      // F5 -- index 4
    Config,    // F6 -- index 5
    Inspect,   // F7 -- index 6
}
```

F8 is NOT a tab -- it toggles `state.show_queue_overview` (a modal overlay).

### Tab Routing (`layout.rs` line 63)

The top-level `render()` function dispatches based on `state.active_tab`:

```rust
match active_tab {
    Tab::Dashboard => views::dashboard::render(f, root[2], state, atmosphere),
    Tab::Plans     => views::plans::render(f, root[2], state, atmosphere),
    Tab::Agents    => views::agents::render(f, root[2], state, atmosphere),
    Tab::Git       => views::git_view::render(f, root[2], state, atmosphere),
    Tab::Logs      => views::logs::render(f, root[2], state),
    Tab::Config    => views::config::render(f, root[2], state),
    Tab::Inspect   => views::context::render(f, root[2], state, atmosphere),
}
```

### F-Key Visual Indicators (`header_bar.rs`)

The header bar renders F-key indicators on the right edge. The active tab is rendered
with inverted colors (fg=VOID, bg=accent, BOLD), while inactive tabs show the key in
accent color and the label in FG_DIM. F8 (queue) is handled separately and uses DREAM
accent:

```rust
let fkey_items = vec![
    (" F1", Theme::ROSE,     "dash",    Tab::Dashboard),
    (" F2", Theme::BONE_DIM, "plans",   Tab::Plans),
    (" F3", Theme::SAGE,     "agents",  Tab::Agents),
    (" F4", Theme::DREAM,    "git",     Tab::Git),
    (" F5", Theme::DREAM,    "logs",    Tab::Logs),
    (" F6", Theme::BONE_DIM, "cfg",     Tab::Config),
    (" F7", Theme::SAGE,     "inspect", Tab::Inspect),
];
```

---

## 3. Per-Tab Implementation Details

### F1: Dashboard (`views/dashboard.rs` -- 435 LOC)

**Layout**: Master-detail with 38% left panel, 1-char spacer, remaining right panel.
Optional 1-row wave progress ribbon at top when waves exist.

**Left Panel** (vertical stack):
1. `plan_tree::render()` -- Collapsible wave/plan hierarchy (content-proportional height)
2. `phase_compact::render()` -- 4 rows: compact pipeline phase indicator
3. `task_progress::render()` -- Task checklist for selected plan (content-proportional)

Height allocation is **content-aware**: plans and tasks share space proportionally
based on actual content counts. When focused, the task list gets up to 60% of space.
When both fit without scrolling, neither has empty voids.

**Right Panel** (sub-tab bar + content area):
- 1-row sub-tab bar (rendered by `render_sub_tab_bar()`)
- Content area dispatched by `state.detail_sub_tab`

**Sub-tab "Agents" content** (`render_agents_content()`):
Vertical stack of:
1. Agent pool (agent_pool or parallel_pool widget)
2. Agent output (scrollable, focused)
3. Gate output (when gate running -- sizes to content, max 40% height)
4. Bottom strip: Token Burn (left 50%) | System Metrics (right 50%)

### F2: Plans (`views/plans.rs` -- ~900 LOC)

**Layout**: Two columns (31%/69%) with 1-char spacer.

Left: Wave tree (`render_left_panel`) with queue position indicator, filter, and
collapsible wave groups. Right: Plan detail (`render_right_panel`) showing selected
plan's full metadata, or pipeline detail when header selected.

Key features:
- Queue-backed runs show `[q#N]` ordinal suffix
- Plan lines have fixed columns: plan name | progress (6) | bar (8) | delta (8) | verify (3) | age (6)
- Column header row with `|` separators
- Milestone progress line with per-milestone counts
- Gradient progress bars per plan
- Health indicators: retries (warning), flailing (ember), failed (ember)
- Verify status: running/passed/failed/pending
- Git dirty stats: +N/-M lines changed
- Phase abbreviation for active plans (prfl/strt/impl/vfy/mrge/etc.)
- Merge feasibility: clean/identical/conflicted with file count

### F3: Agents (`views/agents.rs` -- ~100 LOC)

Two-column layout (32%/68%). Left: agent list with context gauges. Right: selected
agent's output. Uses `agent_grid::render()` for the grid card layout (auto 1-3 columns
based on active agent count).

### F4: Git (`views/git_view.rs` -- ~300 LOC)

Two-column layout (35%/65%). Left: Worktree list with branch names, commit hashes,
and modification status. Right: Branch diff or commit log. Has a `render_for_plan()`
variant that shows per-plan git state.

### F5: Logs (`views/logs.rs` -- ~200 LOC)

Full-width scrollable log viewer. Each log line is colored by level:
- Error: EMBER
- Warn: WARNING
- Info: TEXT_DIM
- Debug: TEXT_GHOST

Supports PgUp/PgDn scrolling, auto-scroll to bottom.

### F6: Config (`views/config.rs` -- ~250 LOC)

Two-column layout (55%/45%). Left: Configuration sections (provider settings, model
assignments, feature flags, paths). Right: MCP tool summary showing available tools
grouped by server. Both panels use `j/k` navigation and `h/l` section cycling.

### F7: Inspect (`views/context.rs` -- ~300 LOC)

Full-width inspection view. Shows MCP tool state, AST analysis data, learning
subsystem state, and test fixture information. Pinned selection stays across
navigations. Primarily a debugging/observability view.

### F8: Queue Overview (Modal)

Not a tab but a modal overlay toggled by F8. Uses `modals/queue_overview.rs`.
Shows the full queue state: milestones, plan ordering, skip specs, and overall
queue progress. Dimmed background with `postfx::dim_overlay(0.45)`.

---

## 4. Dashboard Sub-Tab System

Within F1:Dashboard, the right panel has a sub-tab bar with 7 options + 1 pane toggle:

```rust
const TABS: [(DetailSubTab, &str, Color); 7] = [
    (DetailSubTab::Agents,    "a:Agents",  Theme::ROSE),
    (DetailSubTab::Output,    "o:Output",  Theme::BONE_DIM),
    (DetailSubTab::Diff,      "d:Diff",    Theme::SAGE),
    (DetailSubTab::Errors,    "e:Errors",  Theme::EMBER),
    (DetailSubTab::Git,       "g:Git",     Theme::DREAM),
    (DetailSubTab::Context,   "m:MCP",     Theme::SAGE),
    (DetailSubTab::Processes, "P:Procs",   Theme::WARNING),
];
```

Plus the **pane toggle** `v:impl` / `v:verify` that switches `AgentPaneGroup`.

### Sub-tab rendering:

| Key | SubTab      | Widget/View                                    | What it shows                                |
|-----|-------------|------------------------------------------------|----------------------------------------------|
| `a` | Agents      | `render_agents_content()` composite            | Agent pool + output + gate output + metrics  |
| `o` | Output      | `agent_output::render()`                       | Selected agent's raw output stream           |
| `d` | Diff        | `diff_panel::render()`                         | Git diff with +/- syntax coloring           |
| `e` | Errors      | `error_digest::render()`                       | Structured errors grouped by file + runtime  |
| `g` | Git         | `git_view::render()` or `render_for_plan()`    | Worktree list + branch state for plan        |
| `m` | Context/MCP | `views::context::render()`                     | MCP tool state + AST + learning data         |
| `P` | Processes   | `views::processes::render()`                   | OS process list with CPU/MEM per mori child  |

### Sub-tab bar features:

- Active tab: inverted colors (fg=VOID, bg=accent) + BOLD + UNDERLINED
- Inactive tabs: FG_DIM on BG_RAISED
- **Content-aware badges** on inactive tabs:
  - Agents: `{N}[>]` active agent count
  - Errors: `{N}[x]` error count (EMBER)
  - MCP: `{N}` tool call count (SAGE)
  - Processes: `{max_cpu}%` if any process >5% CPU (WARNING)
- Right-aligned `v:impl`/`v:verify` pane toggle button
- Background: `BG_RAISED` (#0E0C12)

---

## 5. Global Header Bar (`widgets/header_bar.rs` -- 433 LOC)

The header bar is a **single row** rendered on all tabs. It packs an extraordinary
amount of information using compact formatting and separator pipes (`|`).

### Layout (left to right):

```
[heartbeat] mori  Wave 2/5  [queue_label] [skip:N] | [progress_bar 15w] 12/25 48% 3[>] | ETA:2h30m 45m30s $1.23 456K tok MCP:12 | C:36% M:8G N:[up]55K D:R2M | [spinner] impl(opus) | [F-key strip]
```

### Sections:

1. **Heartbeat dot**: Pulsing dot using HEARTBEAT_FRAMES (`[".", "o", ".", "*"]`), brightness
   modulated by `atmosphere.heartbeat()`. Color: pulsing rose RGB derived from heartbeat phase.

2. **App name**: "mori" in ROSE, BOLD.

3. **Wave indicator**: "Wave N/M" in BONE when waves exist.

4. **Queue label**: Source label in DREAM, skip count in EMBER.

5. **Separator**: `|` in TEXT_PHANTOM on BG_SECONDARY.

6. **Progress bar**: 15-char wide. Filled chars (`[block]`) use `gradient_fire()` color.
   Empty chars (`-`) in TEXT_PHANTOM. Plan count "N/M" with `semantic_color()`.
   Percentage shown when not compact and not complete.

7. **In-flight indicator**: `{N}[>]` for active agents in ROSE_DIM.

8. **ETA**: "ETA:{duration}" in DREAM. Calls `state.estimated_remaining_seconds()`.

9. **Elapsed time**: Formatted duration in FG_DIM.

10. **Cost**: Smoothed USD cost (e.g., "$1.23") in BONE_DIM. Only shown when > $0.001.

11. **Tokens**: Total input+output tokens, formatted as "456K tok" or "1M tok" in FG_DIM.

12. **MCP**: "MCP:{count}" in SAGE (ready) or WARNING (not ready). Uses smoothed count.

13. **System metrics** (compact):
    - `C:36%` -- CPU percentage, color by threshold (SAGE/WARNING/EMBER at 50%/80%)
    - `M:8G` -- Memory used, same thresholds
    - `N:[up]55K` -- Network upload rate in DREAM
    - `D:R2M` -- Disk read rate in BONE_DIM

14. **Active agent spinner**: Braille spinner + role short name + model slug.
    Color: role accent. Model shortened (remove "gpt-", "-codex"->"c", "-mini"->"m").

15. **F-key strip** (right-aligned via Layout): See tab system section above.

### Responsive behavior:

- `compact` mode activates when `area.width < 120`: hides percentage, removes colons
  from metric labels.
- Layout uses `Constraint::Min(0)` for left content and `Constraint::Length(fkey_width)`
  for right F-key strip.

---

## 6. Global Status Bar (`widgets/status_bar.rs` -- 221 LOC)

The bottom status bar provides contextual information and keybind hints.

### Layout (left to right):

```
main [abc1234] 3m ago | [heartbeat] 12/25 3[>] 2ag [warn]1 [retry]3 [fail]1 | [up] main:5 @def5678 | [keybind hints for current tab/focus]
```

### Sections:

1. **Git branch + commit**: Branch name in BONE, short commit hash in TEXT_GHOST,
   last commit relative time in TEXT_GHOST. Pipe separator in ROSE_DIM.

2. **Heartbeat dot**: Same HEARTBEAT_FRAMES as header but in ROSE_DIM.

3. **Paused indicator**: " PAUSED " with inverse colors (BG text on STATUS_WARN bg).

4. **Plan progress**: "N/M" or "COMPLETE" or "ERR: {msg}". Color: ROSE normal,
   SAGE complete, error_style error.

5. **Health summary**:
   - `{active}[>] {live}ag` -- active plans and live agents (ROSE_DIM)
   - `[warn]{N}` -- flailing plans count (retries >= 5 or iteration > 3) in EMBER
   - `[retry]{N}` -- total retries across all plans in WARNING
   - `[fail]{N}` -- failed plan count in EMBER

6. **Main merge indicator**: `[up] main:{count} @{commit}` when plans have been
   merged to main. Shows last merge commit hash.

7. **Context-sensitive keybind hints**: Changes per tab AND per focus zone within
   the Dashboard tab:

```
Tab 0 (Dashboard):
  Error state:  "up/dn:nav  s/r:retry  z:diagnose  S/R:repair  c:reverify  p:pause  ?:help"
  Approval:     "y:approve  n:reject  i:inject  p:pause  q:quit"
  Filter mode:  "type:filter  Enter:accept  Esc:cancel"
  Plans focus:  "up/dn:nav  Enter:detail  s/r:retry  z:diag  S/R:repair  h/l:tree  /:filter  ?:help"
  Tasks focus:  "up/dn:tasks  Enter:task detail  Tab:panel  Ctrl-t:picker  v:verify  p:pause  ?:help"
  Output focus: "up/dn:scroll  End:auto  `:agent  Alt+1..7:jump  a/o/d/e/g/m/P:detail  v:verify  i:inject  p:pause"
  Command focus:"up/dn:scroll  Tab:panel  y/n:gate  Ctrl-a:all  a/o/d/e/g/m/P:detail  p:pause  ?:help"

Tab 1 (Plans): "up/dn:nav  left/right:waves  Enter/Esc:tree  PgUp/PgDn:jump  /:filter  s:retry  z:diag  S/R:repair  c:reverify"
Tab 2 (Agents):"up/dn:nav  Tab:panel  `:cycle  Alt+1..7:jump  End:auto  p:pause  F1:dash  ?:help"
Tab 3 (Git):   "up/dn:nav  Enter:select  p:pause  F1:dash  ?:help"
Tab 4 (Logs):  "up/dn/PgUp/PgDn:scroll  p:pause  F1:dashboard  ?:help"
Tab 5 (Config):"j/k:nav  h/l:cycle  Enter:toggle  p:pause  MCP summary on right  F1:dashboard"
Tab 6 (Inspect):"Inspect view  MCP/AST/learning/fixtures  selection stays pinned  p:pause  F1:dashboard  ?:help"
```

---

## 7. System Metrics (`widgets/sys_metrics.rs` -- 296 LOC)

Rendered inside a bordered "System" block. Shows 4--6 rows plus optional top-process list.

### Layout:

```
 CPU  36.5%  [gauge 10w]  [braille sparkline]
 MEM  50.3G  [gauge 10w]  [braille sparkline]
 NET  [up]55K             [braille sparkline]
 DSK  R2.5M               [braille sparkline]
 FPS  30.1
 GW   direct (or gateway URL)
 -- top procs --------
 claude     45.2%   890M  [plan-base]
 cargo      12.1%   456M
```

### Data sources:

All metric data comes from `state.sys: SystemSnapshot`, which is populated by a
**background sysinfo thread** (not the TUI thread). The TUI drains `sys_rx.try_recv()`
on each tick to get the latest snapshot:

```rust
while let Ok(snap) = sys_rx.try_recv() {
    state.sys = snap;
}
```

`SystemSnapshot` contains:
- `cpu_pct: f32` + `cpu_history: VecDeque<f32>` (rolling window)
- `mem_used_bytes: u64`, `mem_total_bytes: u64` + `mem_history: VecDeque<f32>`
- `net_rx_bytes_sec: f64` + `net_rx_history: VecDeque<f64>`
- `disk_read_bytes_sec: f64` + `disk_r_history: VecDeque<f64>`
- `top_procs: Vec<ProcInfo>` (name, cpu_pct, mem_mb, optional plan_base)

### Display values use smoothing:

The displayed values come from `state.smooth: SmoothedValues` which exponentially
interpolates toward targets each tick:

```rust
const RATE: f64 = 8.0;
const DT: f64 = 0.016;
let alpha = 1.0 - (-RATE * DT).exp();
self.cpu_pct += (target - self.cpu_pct) * alpha;
```

This produces fluid, non-jumpy metric updates at ~60fps.

### Animated gauge fills:

The `render_mini_gauge()` function renders solid-block gauges with per-cell
**breathing shimmer**:

```rust
let shimmer = 1.0 + (cell_t * 6.28 + breathing * 3.0).sin() * 0.08;
let br = (r as f64 * breathing * shimmer).min(255.0) as u8;
```

Each cell in the gauge has a slightly different brightness based on its position,
creating a wave effect that rolls across the gauge bar.

### Braille sparklines:

`braille.rs` maps pairs of data points to single braille characters for 2x horizontal
density. Each terminal cell encodes two samples (left/right columns of the braille
pattern). The left column uses bits `[0x40, 0x04, 0x02, 0x01]` (bottom to top),
right column uses `[0x80, 0x20, 0x10, 0x08]`.

### Color thresholds:

```rust
fn pct_color(pct: f64) -> Color {
    if pct >= 0.8 { Theme::EMBER }
    else if pct >= 0.5 { Theme::WARNING }
    else { Theme::SAGE }
}
```

FPS color: >= 50 SAGE, >= 25 WARNING, else EMBER.

---

## 8. Matrix-Style Visual Effects

The TUI has a sophisticated multi-layer VFX system split across several files.

### 8.1 Atmosphere Engine (`atmosphere.rs` -- 284 LOC)

The `Atmosphere` struct is the central animation controller. Created once, ticked
every frame, passed to most render functions.

**State:**
```rust
pub struct Atmosphere {
    frame_count: u64,        // monotonic frame counter
    elapsed: f64,            // total elapsed seconds
    dt: f64,                 // delta time since last frame
    heartbeat_phase: f64,    // TAU-based heartbeat (60-frame period)
    breathing_phase: f64,    // ~5.2s period breathing cycle
    particles: Vec<Particle>,// up to 500 particles
    flash_intensity: f64,    // event flash (decays at 3.0/s)
    flash_hue: f64,          // flash color (default 325 = rose)
    fps: f64,                // measured FPS (1s window)
    effects_config: EffectsConfig,
    rng: fastrand::Rng,
}
```

**Tick function** (`tick_with_degraded()`):
- Measures real delta time, clamps to max 0.1s
- Increments frame counter
- Updates heartbeat phase (60-frame period = 2s at 30fps)
- Updates breathing phase (~5.2s period)
- Decays flash intensity
- Measures FPS (1-second window)
- Updates particle physics (skipped in degraded mode when agents are busy)

**Animation outputs:**
- `spinner()` -> braille spinner character (10 frames, changes every 3 frames)
- `spinner_ethereal()` -> ethereal character spinner (8 frames, changes every 4)
- `heartbeat()` -> 1.0 +/- 0.05 sinusoidal pulse
- `shimmer()` -> 1.0 +/- 0.10 faster shimmer
- `breathing_brightness()` -> 0.88--1.0 range, 5.2s period
- `frame()` -> raw frame counter for deterministic animations

**Degraded mode**: When agents are actively running, `tick_with_degraded(true)` is
called, which skips particle physics updates to reduce CPU overhead.

### 8.2 Particle System (`atmosphere.rs`)

Particles are spawned via `spawn_burst(x, y, count)` (triggered by plan completion
events). Each particle has:

```rust
struct Particle {
    pos: Vec2, vel: Vec2, life: f64, max_life: f64,
    hue: f64, size: f64, ch: char, color: Color,
}
```

Force fields applied each tick:
- Gravity (0.6 downward)
- Drag (0.98 velocity multiplier)
- Optional radial and wind forces

Particles use ETHEREAL character set, fade with life, and are rendered on top of
all content. Cap: 500 particles.

### 8.3 PostFX Pipeline

Applied after all widgets render, before modals:

1. **Ambient fill** (`postfx.rs`): Currently no-op (widgets handle their own fills)

2. **Per-tab pipeline** (`postfx_pipeline.rs`):
   - Dashboard, Agents, Plans: `self_glow()` -- boosts bright cells by 12%
   - Other tabs: no postfx

3. **Particle rendering**: On top of everything

4. **Background visualization layer** (`layout.rs::render_bg_viz_layer()`):
   Dashboard only. Sets bg colors on cells based on:
   - Progress (fills from bottom up with warmer tones)
   - Agent activity (brighter when agents active)
   - Breathing phase (gentle brightness wave)
   - Per-cell variation (noise-based shimmer)
   - Error state (shifts to red tones)

5. **Panel drop shadows** (`postfx.rs::drop_shadow()`): 1-cell dark rim on right
   and bottom edges of each panel. Darkens bg to 30% and fg to 50%.

6. **Modal dimming**: `dim_overlay(0.45)` when any modal is active.

### 8.4 NERV Visualizations (`nerv_viz.rs` -- 356 LOC)

State-driven data visualizations that fill empty cells (never overwrite text).

1. **Progress field** (`progress_field()`): Percolation field driven by completion
   percentage. At 0%: very sparse, slow braille dots. At 100%: dense, fast, bright.
   Uses smooth_noise for flow patterns. Colors in rose/violet range (hue 325-340).

2. **Activity ripples** (`activity_ripples()`): Expanding concentric ring patterns.
   Ring count (3-8) and speed driven by agent activity level.

3. **Data rain** (`data_rain()`): Matrix-style vertical braille streams. Density
   and speed driven by token throughput rate. Each column has independent phase/seed.
   Head-to-tail fade with braille pattern degradation.

4. **State viz** (`state_viz()`): Composite layer combining progress_field (always on)
   and data_rain (when agents active with token throughput).

### 8.5 VFX Library (`vfx.rs` -- 105 LOC)

Pure-math field generators with no state or allocations:
- `plasma(x, y, t)` -- classic plasma effect (4 combined sine waves)
- `noise(x, y, seed)` -- pseudo-random hash function
- `smooth_noise(x, y, seed)` -- bilinear interpolation of noise
- `fbm(x, y, seed, octaves, lacunarity, gain)` -- fractal Brownian motion
- `voronoi(x, y, seed)` -- Voronoi cell distance
- `ripple(x, y, cx, cy, t)` -- expanding ring from center point

Character palettes:
```rust
const DENSITY: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
const ORBS: [char; 6] = ['[dot]', '[degree]', '[bullet]', '[circle]', '[circle]', '[circle]'];
const ETHEREAL: [char; 8] = ['[star]', '[dot]', '[degree]', '[star4]', '[ring]', '[star]', '[star6]', '[flower]'];
```

### 8.6 Effects Configuration (`effects_config.rs`)

9 toggleable VFX booleans:

```rust
pub struct EffectsConfig {
    pub bloom: bool,              // default: true
    pub vignette: bool,           // default: false
    pub dream_atmosphere: bool,   // default: false
    pub amber_color_grade: bool,  // default: false
    pub ambient_orbs: bool,       // default: false
    pub ambient_fill: bool,       // default: false
    pub particles: bool,          // default: false
    pub breathing: bool,          // default: true
    pub screen_postfx: bool,      // default: true
}
```

`degraded()` preset: disables bloom, dream_atmosphere, ambient_orbs, particles, and
screen_postfx. Keeps vignette, amber_color_grade, ambient_fill, and breathing.

---

## 9. Key Binding System (`input.rs` -- ~700 LOC)

### TuiAction Enum

The input handler maps crossterm key events to a `TuiAction` enum. Key actions include:

```rust
pub enum TuiAction {
    Quit, Pause, Resume,
    SelectTab(usize),          // F1-F7
    ToggleQueueOverview,       // F8
    NextPlan, PrevPlan,        // arrow up/down in plan tree
    NextTask, PrevTask,        // arrow up/down in task list
    SelectPlan,                // Enter
    ExpandWave, CollapseWave,  // h/l in plan tree
    ScrollUp, ScrollDown,      // arrow up/down in output/logs
    ScrollPageUp, ScrollPageDown,
    ScrollToEnd,               // End -- auto-scroll
    CycleAgent,                // ` (backtick)
    JumpAgent(usize),          // Alt+1..7
    SelectAgentTab(usize),     // direct agent tab selection
    DetailSubTab(DetailSubTab),// a/o/d/e/g/m/P
    ToggleAgentPaneGroup,      // v -- impl/verify
    Filter,                    // /
    FilterAccept, FilterCancel,
    Inject,                    // i
    Approve, Reject,           // y/n
    Retry,                     // s or r
    Diagnose,                  // z
    Repair,                    // S or R
    Reverify,                  // c
    Verify,                    // v in tasks
    ToggleHelp,                // ?
    TogglePlanDetail,          // Enter in plan tree
    ToggleWaveOverview,
    ToggleAgentPoolModal,
    ToggleTaskDetail,          // Enter in task list
    ToggleTaskPicker,          // Ctrl-t
    CycleFocus,                // Tab
    InjectSubmit, InjectCancel,
    PlanDetailTab(PlanDetailTab), // plan detail sub-tabs
    // ... many more
}
```

### InputMode State Machine

```rust
pub enum InputMode {
    Normal,   // standard keybinds
    Filter,   // typing filter text (/ mode)
    Inject,   // typing inject message (i mode)
}
```

### FocusZone (within Dashboard)

```rust
pub enum FocusZone {
    Plans,        // left: plan tree
    Tasks,        // left: task list
    AgentOutput,  // right: agent output panel
    CommandOutput, // right: gate/command output panel
}
```

Tab cycles focus: Plans -> Tasks -> AgentOutput -> CommandOutput -> Plans.

### Key binding mapping (by tab and focus):

**Global** (all tabs):
- F1-F7: Switch tabs
- F8: Toggle queue overview
- ?: Toggle help modal
- q/Ctrl-c: Quit
- p: Pause/resume pipeline

**Dashboard + Plans focus**:
- Up/Down: Navigate plan tree
- Enter: Open plan detail modal or toggle selection
- h/l: Collapse/expand wave nodes
- /: Enter filter mode
- s, r: Retry selected plan
- z: Diagnose selected plan
- S, R: Repair selected plan
- c: Reverify selected plan
- M: Merge selected plan

**Dashboard + Tasks focus**:
- Up/Down: Navigate task list
- Enter: Task detail modal
- Ctrl-t: Task picker modal
- v: Verify task

**Dashboard + AgentOutput focus**:
- Up/Down: Scroll output
- End: Auto-scroll to bottom
- `: Cycle through agents
- Alt+1..7: Jump to specific agent
- a/o/d/e/g/m/P: Switch detail sub-tab
- i: Inject message to agent
- v: Toggle impl/verify pane group

**Mouse support**: `hit_test.rs` computes `HitZones` by replaying the layout math
to determine which panel occupies which screen region. Mouse clicks resolve to the
appropriate action based on zone.

---

## 10. Plan List Tree View with Wave Grouping (`widgets/plan_tree.rs` -- 1078 LOC)

This is the most complex widget. It renders a collapsible hierarchical tree:

```
 Q  milestone-1  8/12  milestone-2  4/7
 plan          | prog |   bar  |  delta | vfy | age
 [collapse] [>] Wave 1  (8/10) [========--] [retry]2 [fail]1 after W0 ----
    [check] plan-name       impl  | 5/8  |[======--]| +12/-3 | [check]v | 30m
       [bar 8w]  phase impl . branch feat/x . [retry]2task+1spawn . iter 2
    [check] other-plan            | 3/3  |[========]|    .   |  .  |  2h
 [collapse] . Wave 2  (0/5) [--------] after W1 ----
    [circle] pending-plan         |  .   |    .     |    .   |  .  |  ~5m
```

### Features:

1. **Wave headers**: Collapsible via `state.wave_expanded` HashSet. Shows:
   - Collapse icon ([>]/[v])
   - Status icon (check/play/dot)
   - Wave number + progress (done/total)
   - 8-char gradient bar (ocean gradient, heartbeat pulse when active)
   - Health indicators (flailing/warning/failed counts)
   - Blocker label ("after W{N}" -- computed via `wave_blockers`)
   - Dash-fill to full width

2. **Plan lines**: Fixed-column layout with `|` separators:
   - Prefix: indent (3 chars if in wave) + status icon (check/play/cross/circle)
   - Plan name: Truncated with middle ellipsis, phase suffix for active plans
   - Progress column (6 chars): "N/M" with semantic color
   - Bar column (8 chars): Block fill with semantic color
   - Delta column (8 chars): Git +N/-M stats OR health indicator
   - Verify column (3 chars): check-v/cross-v/circle-v/running
   - Age column (6 chars): Compact time (30m, 2h, 1d)

3. **Selected plan detail row**: When a plan is selected and focused, an expanded
   detail row shows: progress glyphs, phase, branch, retry counts, iteration,
   merge status (clean/identical/conflicted with file count), merge commit hash.

4. **Empty space fill**: When the plan tree has significant empty space below its
   content, `nerv_viz::progress_field()` fills it with progress-driven braille
   animation.

5. **Scrollbar**: Proportional scrollbar using `scrollbar::render_scrollbar()` when
   content exceeds visible height.

6. **Filter**: `matches_filter()` does case-insensitive substring matching on plan
   base names. Filter indicator shown at top: `/filter_text/`.

7. **Queue summary line**: When queue-backed, shows milestone progress with per-milestone
   counts, skip summary, and "F2 full view" hint.

### Color logic (semantic):

- Done/Completed/MergedToMain: SAGE (green)
- Active + all tasks dispatched: WARNING (amber -- phase is bottleneck)
- Active + partial: `semantic_color(fill_pct)` gradient
- Failed: EMBER (red)
- Pending + no progress: TEXT_GHOST (very dim)
- Phase suffix: colored by `Theme::phase_accent()`

---

## 11. Color Scheme and Styling (ROSEDUST Palette)

### Named Colors (`theme.rs`):

```
VOID            #000000  true black background
ROSE            #B97894  primary accent -- warm pink/mauve
ROSE_BRIGHT     #DC9BB4  alerts, focused elements -- lighter rose
ROSE_DIM        #8C6070  secondary rose -- muted
BONE            #D7C69E  scarcest emphasis -- warm ivory
BONE_DIM        #A08E6C  softer bone
TEXT            #A58E9E  default text -- rose-tinted grey
TEXT_DIM        #91788A  secondary text
TEXT_GHOST      #6E5569  tertiary text -- warm pink, still readable
DREAM           #7873A5  indigo/violet accent
SAGE            #7D9E8C  success green -- muted
EMBER           #C36E55  error red-orange
WARNING         #C39B5F  caution amber-gold

BG_RAISED       #0E0C12  slight lift from void
BG_SECONDARY    #0E0C10  subtle lift for bars
BG_HIGHLIGHT    #221C24  selection highlight
BG_BUBBLE_ALT   #121016  alt bubble background
ROSE_DEEP       #412434  deep rose for fills
ROSE_EMBER      #502D3E  warm dark accent
TEXT_PHANTOM    #372A37  structural elements, near-invisible
```

### Design principles:

- **Warm monochrome base**: Almost everything is rose-tinted grey, not neutral grey
- **Color scarcity**: BONE is the "scarcest emphasis" -- used only for truly important info
- **Semantic colors**: Only 3 status colors (SAGE/WARNING/EMBER) for pass/caution/fail
- **Deep blacks**: BG variations are nearly black (#0E0C10 to #0E0C12) for depth
- **Accent is indigo**: DREAM provides the cool counterpoint to the warm rose palette
- **No pure white**: Maximum brightness is BONE (#D7C69E), never #FFFFFF

### Named gradients:

```rust
gradient_fire()    -- progress bars: dark red (100,30,30) -> amber (200,100,30) -> gold (220,180,60)
gradient_context() -- context gauges: SAGE -> WARNING -> EMBER
gradient_ocean()   -- wave bars: deep blue -> teal -> cyan (HSV-based LUT)
fire_gradient()    -- brighter rose variant: 320->350 hue
ember_gradient()   -- deep red -> orange (0-30% completion)
amber_gradient()   -- amber -> gold (30-70% completion)
sage_gradient()    -- yellow-green -> bright green (70-100%)
context_gradient() -- sage -> warning -> ember (HSV-based)
ocean_gradient()   -- deep blue -> teal -> cyan (HSV-based)
```

### Per-role accent colors:

```
Conductor:       EMBER           Implementer: ROSE
Strategist:      BONE_DIM        Architect:   SAGE
Auditor:         WARNING         Scribe:      DREAM
Critic:          TEXT            Researcher:  DREAM
```

### Gradient implementation (`color.rs`):

HSV-based gradient LUT with O(1) sampling. `Gradient::from_hsv_stops()` pre-computes
a 256-entry lookup table with hue interpolation that handles wrapping (avoids going
through green when interpolating red->violet).

---

## 12. Event/Update Loop Architecture

### Frame timing:

The TUI runs inside a `tokio::select!` loop in `app/parallel.rs`. The tick arm
uses a `tokio::time::interval`:

```rust
let mut tick = tokio::time::interval(Duration::from_millis(16)); // ~60fps target
```

### Per-tick processing order:

1. **Write live status** to disk (every 1s)
2. **Atmosphere tick**: `atmosphere.tick_with_degraded(state.any_agent_active())`
3. **Drain system metrics**: `while let Ok(snap) = sys_rx.try_recv()`
4. **Handle dirty flags**: Deferred task checklist refresh
5. **Refresh selected plan recovery state**
6. **Smooth value interpolation**: Compute targets, call `state.smooth.tick(&targets)`
7. **Set executor parallelism limits** from config
8. **Handle particle burst events**
9. **Update crash state** (every 2s)
10. **Orphan reaper** (every 30s)
11. **Clamp agent tab** to valid range
12. **Sample token burn history** (every 2 frames)
13. **Adaptive FPS**: Full 60fps when user interacting, ~20fps when idle (skip 2/3 frames)
14. **Terminal draw**: `terminal.draw(|f| tui::layout::render(f, &state, &atmosphere))`
15. **Expire notifications** (auto-dismiss after TTL)
16. **Conductor tick** (every 60 frames): stall detection, ghost turns, context pressure

### Frame skip logic:

```rust
let user_idle = last_user_input.elapsed() > Duration::from_secs(3);
let should_draw = if state.any_agent_active() && user_idle {
    frame_skip_counter % 3 == 0  // ~20fps
} else {
    true  // full 60fps
};
```

### Data flow:

```
Background sysinfo thread  ----sys_rx----> state.sys (SystemSnapshot)
Agent events                ----agent_rx--> state.agents, state.parallel_agents
Gate completion events      ----gate_rx---> state.gate_output, state.error
Git events                  ----git_rx----> state.git_branch, state.git_worktree_list
Orchestrator events         ----orch_rx---> state.plans, state.execution_waves
Crossterm key events        ----EventStream-> input::handle_key() -> TuiAction
Conductor ticks             ----periodic--> state.conductor_actions
```

### SmoothedValues interpolation:

All numeric display values go through exponential smoothing to prevent visual jumps:

```rust
const RATE: f64 = 8.0;   // convergence speed
const DT: f64 = 0.016;   // assumed 60fps
let alpha = 1.0 - (-RATE * DT).exp();  // ~0.12 per frame
self.value += (target - self.value) * alpha;
```

Fields smoothed: cost_usd, cpu_pct, mem_bytes, net_rate, disk_rate, token_total,
token_rate, mcp_calls, task_done, per-agent tokens.

---

## 13. Streaming/Real-Time Update Patterns

### Agent output streaming:

Agent output arrives via `agent_rx` channel as `AgentEvent` messages. The event loop
appends to `state.agents[role].output` string. The agent_output widget renders the
last N lines with auto-scroll (when at bottom). User can scroll up to pin, End key
resumes auto-scroll.

### Gate output streaming:

Gate progress lines arrive via `gate_progress_tx/rx` channel. Each line is appended
to the per-plan gate output buffer. The command_output widget renders with auto-scroll
and PASS/FAIL badges derived from content analysis (regex on "FAILED", "error[E",
"test result: ok").

### Token burn history:

Sampled every 2 frames (every ~32ms) from `state.agents[role].input_tokens`. Stored
in `state.token_burn_history: HashMap<AgentRole, VecDeque<u64>>` with a 120-sample
rolling window. Used by `token_sparkline` widget for braille sparkline charts.

Rate calculation uses a 30-sample window (1 second at 30fps):
```rust
let delta = history.back() - history[start];
rate = delta * 60.0 / (window / 30.0)  // tokens per minute
```

### System metrics background thread:

A dedicated thread runs sysinfo polling (CPU, memory, network, disk, process list)
and sends `SystemSnapshot` via `sys_rx`. The TUI thread drains all pending snapshots
each tick, keeping only the latest. This ensures heavy OS calls never block rendering.

### Notification toasts:

`state.notifications: Vec<Notification>` with per-notification TTL. Rendered as
overlays at the bottom-right (up to 3 visible, newest first). Expired notifications
are reaped each tick.

---

## 14. Modal System

### Modal rendering order (`layout.rs` lines 91--157):

1. Check if any modal is active
2. Apply `dim_overlay(0.45)` to entire screen
3. Render modals in priority order:
   - Inject input
   - Filter overlay
   - Approval prompt
   - Plan detail
   - Help overlay
   - Wave overview
   - Agent pool modal
   - Queue overview
   - Task detail
   - Task picker
   - Confirm dialog
4. Toast notifications (on top of everything)

### Modal list with trigger keys:

| Modal              | Trigger     | File                        |
|--------------------|-------------|-----------------------------|
| Help               | `?`         | `modals/help.rs`            |
| Plan detail        | `Enter`     | `modals/plan_detail.rs`     |
| Task detail        | `Enter`     | `modals/task_detail.rs`     |
| Task picker        | `Ctrl-t`    | `modals/task_picker.rs`     |
| Inject message     | `i`         | `modals/inject.rs`          |
| Approval           | auto        | `modals/approval.rs`        |
| Confirm            | `M` (merge) | `modals/confirm.rs`         |
| Wave overview      | wave click  | `modals/wave_overview.rs`   |
| Agent pool modal   | agent click | `modals/agent_pool_modal.rs`|
| Queue overview     | `F8`        | `modals/queue_overview.rs`  |
| Batch review       | auto        | `modals/batch_review.rs`    |
| Notification toast | auto        | `modals/notification.rs`    |
| Quit confirm       | `q`         | `modals/quit.rs`            |

### Modal visual effects:

- Background dimmed to 45% brightness
- Modal panels get `modal_glow()` with 2-cell radius glow around the panel
- Drop shadows on modal panels

---

## 15. TUI Launch and Lifecycle

### Initialization (`main.rs`):

```rust
fn main() -> anyhow::Result<()> {
    // SIGPIPE ignored
    // CLI parsed
    // Config built

    // Panic hook: restore terminal + capture crash report
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::restore();
        // ... write crash report ...
    }));

    // Tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(128)
        .build()?;

    let result = rt.block_on(app::run(config));

    // ALWAYS restore terminal (even on early exit)
    let _ = tui::restore();

    result
}
```

### Terminal setup (`tui::init()`):

```rust
enable_raw_mode()?;
execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
Terminal::new(CrosstermBackend::new(stdout))
```

### Crash safety:

The terminal is restored in THREE places:
1. Normal exit after `app::run()` completes
2. Panic hook (before crash report)
3. Error path after `app::run()` returns Err

This ensures the terminal is never left in raw mode, even on crashes.

### Conditional padding:

When terminal is large enough (height >= 50, width >= 120), a 1-cell margin is added
around all content for breathing room:

```rust
let padded = if area.height >= 50 && area.width >= 120 {
    area.inner(Margin { vertical: 1, horizontal: 1 })
} else {
    area
};
```

---

## 16. Rendering Architecture Summary

### Render pipeline (per frame):

```
1. Clear background (Theme::default_style())
2. Apply conditional padding
3. Compute root layout: header(1) | alert(0-1) | content(min) | status(1)
4. Render header_bar (all tabs)
5. Render alert row if present
6. Route to active tab view
7. Render status_bar (all tabs)
8. Apply atmosphere postfx (bloom, self-glow)
9. Apply bg_viz_layer (Dashboard only -- state-driven bg colors)
10. Apply panel drop shadows
11. Dim overlay if any modal active
12. Render modals in priority order
13. Render toast notifications
```

### Widget composition pattern:

All widgets follow the same signature pattern:
```rust
pub fn render(f: &mut Frame, area: Rect, state: &RunState, atmosphere: &Atmosphere)
```

Some add `focused: bool` parameter. None maintain their own state -- all state comes
from the shared `RunState` struct and the `Atmosphere` animation controller.

### Layout approach:

- `Layout::default().direction(Horizontal/Vertical).constraints([...]).split(area)`
- Fixed-width columns for data-dense views (progress, bars, delta, verify, age)
- Percentage-based splits for panels (31%/69%, 38%/62%, etc.)
- Content-aware height allocation (plan tree and task list share proportionally)
- `Constraint::Min(0)` for elastic regions
- Spacer columns (1 char, untouched) between panels

### Performance optimizations:

1. Degraded atmosphere mode (skip particles when agents running)
2. Adaptive frame rate (60fps interactive, ~20fps idle)
3. Background sysinfo thread (no OS calls on render thread)
4. Deferred checklist refresh (dirty flag, not on every keypress)
5. Smoothed values (prevent unnecessary redraws from jumpy data)
6. Pre-computed gradient LUTs (O(1) color sampling)
7. Frame skip counter for periodic tasks (token sampling, conductor tick, orphan reap)

---

## 17. Key Data Structures (from `state/mod.rs`)

### RunState (the god object -- ~300 fields)

Selected critical fields relevant to TUI rendering:

```rust
pub struct RunState {
    pub plans: Vec<RunPlanEntry>,
    pub execution_waves: Vec<(usize, Vec<String>)>,
    pub current_wave: usize,
    pub agents: IndexMap<AgentRole, AgentState>,
    pub parallel_agents: Vec<ParallelAgentState>,
    pub active_tab: usize,
    pub detail_sub_tab: DetailSubTab,
    pub focus: FocusZone,
    pub selected_plan: usize,
    pub selected_agent_tab: usize,
    pub plan_scroll_offset: usize,
    pub wave_expanded: HashSet<usize>,
    pub filter_text: String,
    pub filter_active: bool,
    pub input_mode: InputMode,
    pub error: Option<String>,
    pub complete: bool,
    pub git_branch: String,
    pub mcp: McpState,
    pub sys: SystemSnapshot,
    pub smooth: SmoothedValues,
    pub cumulative_cost_usd: f64,
    pub cumulative_input_tokens: u64,
    pub cumulative_output_tokens: u64,
    pub token_burn_history: HashMap<AgentRole, VecDeque<u64>>,
    pub notifications: Vec<Notification>,
    pub show_help: bool,
    pub show_plan_detail: bool,
    pub show_queue_overview: bool,
    // ... ~250 more fields ...
}
```

### RunPlanEntry:

```rust
pub struct RunPlanEntry {
    pub base: String,
    pub num: String,
    pub status: RunPlanStatus,
    pub phase: String,
    pub iteration: u32,
    pub task_retries: u32,
    pub spawn_retries: u32,
    pub git_branch_short: Option<String>,
    pub git_dirty: Option<(u32, u32)>,
    pub estimated_minutes: Option<u32>,
    pub actual_minutes: Option<u32>,
    pub started_at: Option<Instant>,
    pub merge_feasibility: MergeFeasibility,
    pub merge_commit: Option<String>,
}
```

### RunPlanStatus:

```rust
pub enum RunPlanStatus {
    Pending,
    Active,
    Done,           // tasks complete, awaiting merge
    Completed,      // merged or fully done
    CompletedPrior, // was complete before this run
    Failed,
    Skipped,
    MergedToMain,
}
```
