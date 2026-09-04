# Mori TUI Reference Audit for Roko Parity

**Date**: 2026-09-01
**Source**: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/`
**Target**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`

---

## File Inventory Comparison

### Mori TUI structure (20 top-level files + 3 subdirectories)

```
tui/
  atmosphere.rs          # Particle system, heartbeat, breathing, flash effects
  bars.rs                # Gradient/segmented/semantic progress bars
  color.rs               # HSV/RGB, screen/additive blend, LUT-based gradients
  effects_config.rs      # 9 toggleable visual effects (bloom, vignette, etc.)
  hit_test.rs            # Mouse click-to-panel zone detection
  input.rs               # 136-action TuiAction enum, 4 input modes, per-tab keybinds
  layout.rs              # Root render function, modal overlay dispatch, bg viz layer
  math.rs                # Vec2, easing functions, wave combinators
  mod.rs                 # Module exports, terminal init/restore
  nerv_viz.rs            # Braille sub-pixel data visualizations (progress field, ripples, data rain)
  postfx.rs              # Bloom, vignette, dim overlay, modal glow, ambient orbs, dream atmosphere, drop shadow, color grade
  postfx_pipeline.rs     # Per-tab postfx dispatch (self-glow on Dashboard/Agents/Plans)
  tabs.rs                # 7-tab enum (F1-F7)
  theme.rs               # ROSEDUST palette (13 core + 4 extended + 7 derived colors), Gradient util, named gradients
  vfx.rs                 # Stateless VFX fields: plasma, noise, FBM, voronoi, ripple
  modals/                # 13 modal dialogs
  views/                 # 12 view files
  widgets/               # 27 widget files
```

### Roko TUI structure (37 top-level files + 4 subdirectories)

```
tui/
  ansi.rs, app.rs, approval_ipc.rs, atmosphere.rs, config_meta.rs,
  cursors.rs, dashboard.rs, dashboard_gen.rs, display_utils.rs,
  effects_config.rs, event.rs, fs_watch.rs, git_watch.rs, hit_test.rs,
  input.rs, jsonl_cursor.rs, jsonl_tailer.rs, layout.rs, mod.rs,
  postfx.rs, postfx_pipeline.rs, scroll.rs, segment.rs, smoothing.rs,
  snapshot.rs, state.rs, tabs.rs, task_outputs.rs, theme.rs, util.rs,
  verdicts.rs, ws_client.rs
  modals/               # 14 modal files (same set + quit.rs)
  pages/                # 3 files (legacy scaffold)
  views/                # 12 view files (adds affect, atelier, learning, marketplace)
  widgets/              # 18 widget files (missing several mori widgets)
```

---

## 1. What does mori's TUI do that roko doesn't?

### Features mori has that roko lacks or has incomplete:

| Feature | Mori | Roko |
|---------|------|------|
| **Particle system** | Full physics engine: Particle struct with pos/vel/life/hue, ForceField enum (Gravity, Drag, Radial, Wind), capped at 500 particles, `spawn_burst()` on events | Atmosphere struct exists but particle fields are simpler, no force physics |
| **NERV data viz layer** | `nerv_viz.rs`: braille sub-pixel progress fields, activity ripples with ring physics, data rain streams -- all driven by system state (progress, token rate, activity) | No equivalent `nerv_viz.rs`; missing braille-based state visualizations |
| **VFX field library** | `vfx.rs`: plasma, noise, smooth_noise, FBM, voronoi, ripple generators -- all stateless pure math | No `vfx.rs` equivalent; no procedural noise/field generators |
| **Math utilities** | `math.rs`: Vec2, lerp, remap, 6 easing functions (cubic, quad, elastic), wave combinators, triangle wave | No `math.rs`; limited animation math |
| **Color utilities** | `color.rs`: Full HSV pipeline, screen blend, additive blend, LUT-based O(1) gradient sampling (256-entry), hue-aware lerp | Theme has RGB gradients but no LUT-based sampling, no blend modes |
| **Agent output streaming** | Dedicated `agent_output.rs` widget (700+ lines): role-specific tab bar, per-role transcript panels, auto-scroll with manual override, word wrap, syntax coloring (error/pass/heading/quote), verification status panels, scrollbar | Roko has basic agent output but less structured |
| **Branch tree widget** | `branch_tree.rs`: interactive navigable tree with status badges, plan-colored branches, commit graph coloring | Present but simpler |
| **Context gauge widget** | `context_gauge.rs`: context window pressure visualization | Missing as standalone widget |
| **Token bar widget** | `token_bar.rs`: per-agent token consumption bar | Missing |
| **Wave bar widget** | `wave_bar.rs`: execution wave progress indicator | Missing |
| **Phase timeline widget** | `phase_timeline.rs`: pipeline phase progression view | Missing |
| **Phase bar widget** | `phase_bar.rs`: compact phase indicator | Missing |
| **Plan list widget** | `plan_list.rs`: legacy plan listing | Missing |
| **Agent grid widget** | `agent_grid.rs`: grid layout of agent cards | Missing |
| **Tab bar widget** | `tab_bar.rs`: standalone tab bar renderer | Missing |
| **Status badge widget** | `status_badge.rs`: reusable colored status badges (Active/Done/Idle/Error) | Missing as standalone widget |
| **Monitors view** | `views/monitors.rs` | Not applicable / different concept |
| **Pipeline view** | `views/pipeline.rs` | Not applicable |
| **Review view** | `views/review.rs` | Not applicable |
| **Tasks view** | `views/tasks.rs` | Task progress exists as widget |
| **Processes view** | `views/processes.rs` | Not explicitly present |

---

## 2. What visual elements are in mori that are missing from roko?

### Background visualization layer
Mori has a complete post-render background viz pass (`render_bg_viz_layer` in `layout.rs`) that:
- Computes per-cell background colors driven by `task_weighted_progress()`, agent activity, and error state
- Uses sinusoidal row waves modulated by breathing brightness
- Changes base color on error (red shift vs. rose/purple)
- Only runs on Dashboard tab
- Respects existing bright pixels (skips cells with RGB > 15)

### Panel shadows
`render_panel_shadows` adds drop shadows to all major panels on every tab. The shadow pass runs after bg viz to avoid being overwritten. Per-tab shadow rects are computed from the exact layout constraints.

### Filter overlay
A `/`-activated search filter renders as a small overlay at the bottom of the plan tree with vim-style cursor block.

### Alert ribbon
A 1-line alert row between header and content shows the highest-priority alert (error > gate failures > preflight warnings) with a warning icon and `[Esc]` dismiss hint.

### Content-aware badges on sub-tab bar
Dashboard sub-tabs show live content badges when inactive:
- Agents: `3>` (active agent count)
- Errors: `2x` (error count)
- Context: `14` (MCP tool call count)
- Processes: `95%` (max CPU)

### Progress bar in header
The header bar has a 15-cell gradient progress bar that shifts color through fire gradient as completion rises, plus semantic coloring for progress text.

### In-flight agent indicator
Header shows `3>` count of in-flight parallel agents.

### System metrics in header
Compact `C:42% M:8G N:12K D:4M` system metrics strip with color-coded thresholds.

### MCP status indicator
Header shows `MCP:14` with green/warning color based on ready state.

### Heartbeat animation
Pulsing dot in header and status bar using 4-frame animation (`. o . *`) modulated by atmosphere breathing.

### Semantic progress bar
`bars.rs` has `semantic_bar` that shifts through red/amber/green gradients based on completion percentage.

### Gradient bars with leading edge glow
Filled bars use per-cell gradient coloring with a brighter leading edge character.

### Segmented bars
`segmented_bar` creates NERV-style grouped block segments with dim separators.

---

## 3. How does mori handle modals?

### Modal architecture

Mori uses a **flag-based modal system** with these key patterns:

1. **State flags**: Each modal has a boolean in `RunState`:
   ```rust
   state.show_plan_detail
   state.show_help
   state.show_wave_overview
   state.show_agent_pool_modal
   state.show_queue_overview
   state.show_task_detail
   state.show_task_picker
   state.pending_approval  // Option<PendingApproval>
   state.pending_confirm   // Option<ConfirmAction>
   state.input_mode == InputMode::Inject
   ```

2. **Render order** (in `layout.rs`):
   - All tab content renders first
   - Atmosphere postfx applies (bloom)
   - Background viz layer renders
   - Panel shadows apply
   - **Then**: if any modal is active, `dim_overlay(area, buf, 0.45)` dims the entire screen
   - Each active modal renders on top with `Clear` + drop shadow + themed block

3. **Dim overlay**: `postfx::dim_overlay` multiplies all fg and bg RGB values by 0.45, making background content visible but subdued.

4. **Drop shadow**: Every modal calls `postfx::drop_shadow(buf, popup)` which darkens 1-cell rim right and below the modal.

5. **Modal glow**: `postfx::modal_glow` adds a 2-cell-radius colored glow around modal borders (though not all modals use it).

6. **Toast notifications**: Up to 3 toasts stack upward from the bottom-right, each in its own bordered box with level-colored border.

7. **Centered rect helper**: All modals use a shared `centered_rect(percent_x, percent_y, area)` function that computes a percentage-based centered region.

8. **Input mode**: `InputMode` enum has 4 states: `Normal`, `Inject`, `Filter`, `Confirm`. Modals that need text input switch the input mode, which changes the key handler.

9. **Key interception order** in `handle_key`:
   - Task picker modal intercepts first (if visible)
   - Task detail modal intercepts second
   - Queue overview intercepts third
   - Then normal key handling proceeds

### 13 modal types:
| Modal | Size | Purpose |
|-------|------|---------|
| `approval` | 60% x 30% | Agent command approval (y/n) |
| `confirm` | 50% x 25% (or 55-60% for merge variants) | Destructive action confirmation with 3 specialized merge layouts |
| `help` | 72% x 78% | Two-column keybinding reference |
| `inject` | 60% x 20% | Text input to steer agent via conductor |
| `notification` | 44 chars x 3 lines | Toast stack (bottom-right) |
| `plan_detail` | large | Scrollable plan detail with tabs (TOML/summary) |
| `queue_overview` | large | Multi-tab milestone/queue view |
| `task_detail` | large | Single task detail with scrolling |
| `task_picker` | large | Filterable task list for jumping |
| `wave_overview` | large | Wave progress and plan status |
| `agent_pool_modal` | large | Agent pool detail view |
| `batch_review` | - | Batch merge review |
| `quit` | - | Quit confirmation |

---

## 4. How does mori handle scrolling?

### Scroll model

1. **Per-zone scrolling**: Scrolling is tied to `FocusZone` (Plans, Tasks, AgentOutput, CommandOutput). The active focus zone receives scroll events.

2. **Focus cycling**: `Tab`/`Shift+Tab` cycles through zones. Each zone maintains independent scroll state.

3. **Auto-scroll with manual override**: Agent output auto-scrolls to bottom by default. Scrolling up manually pauses auto-scroll. `End` key or `Space` resumes auto-scroll.

4. **Scroll state fields** on `RunState`:
   - `log_scroll: usize` -- offset from bottom for log view
   - `agent_scroll: Option<usize>` -- `None` = auto-scroll, `Some(offset)` = manual
   - `plan_detail_scroll: usize` / `plan_summary_scroll: usize` -- modal scroll
   - Various cursor positions for lists

5. **Viewport calculation pattern**: Every scrollable panel computes:
   ```rust
   let visible_height = area.height.saturating_sub(2) as usize; // minus borders
   let total = items.len();
   let start = total.saturating_sub(visible_height + scroll);
   let end = total.saturating_sub(scroll);
   let visible = &items[start..end];
   ```

6. **Scrollbar widget**: `scrollbar::render_scrollbar` draws a minimal track+thumb on the rightmost column:
   - Track: `+` in `TEXT_GHOST` color
   - Thumb: `|` in accent color
   - Thumb size proportional to viewport/total ratio
   - Rendered via direct buffer manipulation (no ratatui widget)

7. **Page jump**: Plans tab supports `PgUp`/`PgDn` for 10-item jumps (`NavigatePageUp`/`NavigatePageDown`).

8. **Indicator for scroll position**: When manually scrolled, agent output shows `# X lines above` at top and `[End] to resume auto-scroll` at bottom.

---

## 5. What's mori's render loop like?

### Tick rate and frame rate

- **No fixed FPS target** in the `EventHandler`. The tick rate is configurable via `EventHandler::new(tick_rate)`.
- The app calls `atmosphere.tick()` (or `tick_with_degraded(true)`) on every frame.
- Atmosphere internally tracks `dt` from `Instant::now() - last_frame`, clamped to max 0.1s to prevent huge jumps on lag.
- FPS is computed by accumulating frames over 1-second windows.
- **Degraded mode**: When agents are actively running, `tick_with_degraded(true)` skips particle physics but keeps breathing/heartbeat.

### Render path

1. Clear with background color
2. Apply conditional 1-cell padding for large terminals (>= 50h x 120w)
3. Compute alert (error > gate failure > preflight warning)
4. Split into: header (1 line) | alert (0-1) | content (flex) | status bar (1 line)
5. Render header bar with F-key strip
6. Render alert if present
7. Route to per-tab view (Dashboard/Plans/Agents/Git/Logs/Config/Inspect)
8. Render status bar
9. Apply atmosphere postfx (bloom, self-glow)
10. Background viz layer (Dashboard only)
11. Panel shadows
12. If any modal active: dim overlay at 0.45
13. Render active modals (in priority order)
14. Render toast notifications (up to 3, bottom-right)

### Atmosphere internals

- `frame_count`: monotonic counter
- `elapsed`: total seconds since start
- `heartbeat_phase`: `frame_count / 60 * TAU`
- `breathing_phase`: `elapsed * TAU / 5.2` (~5.2s period, range 0.88-1.0)
- `flash_intensity`: decays at `dt * 3.0` per frame
- `particles[]`: up to 500, with physics (gravity, drag)
- `fps`: computed from 1-second accumulator

---

## 6. What widgets does mori have that roko doesn't?

### Widgets missing from roko (present in mori but not in roko's `widgets/`):

| Mori Widget | Purpose | Roko Status |
|-------------|---------|-------------|
| `agent_grid.rs` | Grid layout of agent cards with role icons | Missing |
| `agent_output.rs` | 700+ line role-tabbed transcript viewer with auto-scroll | Missing as dedicated widget (inline in views) |
| `agent_pool.rs` | Sequential agent pool status panel | Missing as widget |
| `branch_tree.rs` | Interactive git branch tree with status-colored nodes | Missing as widget |
| `command_output.rs` | Gate/compile/test command output viewer | Missing (gate_output.rs exists but different) |
| `context_gauge.rs` | Context window utilization gauge | Missing |
| `plan_list.rs` | Legacy plan list (flat) | Missing |
| `phase_bar.rs` | Compact pipeline phase indicator | Missing |
| `phase_timeline.rs` | Full phase timeline view | Missing |
| `scrollbar.rs` | Minimal track+thumb scrollbar via buffer manipulation | Missing (no scrollbar widget) |
| `status_badge.rs` | Reusable `badge("text", Status)` function | Missing |
| `tab_bar.rs` | Standalone tab bar renderer | Missing |
| `token_bar.rs` | Per-agent token consumption bar | Missing |
| `wave_bar.rs` | Per-wave progress bar | Missing |

### Widgets roko has that mori doesn't:

| Roko Widget | Purpose |
|-------------|---------|
| `cost_by_model.rs` | Cost breakdown per model |
| `dream_view.rs` | Dream/consolidation state viewer |
| `gate_output.rs` | Gate output viewer (different from mori's command_output) |
| `rosedust.rs` | ROSEDUST palette utilities |

---

## 7. What's mori's keyboard model?

### Input mode state machine

```
Normal -> Inject (press 'i')
Normal -> Filter (press '/')
Normal -> Confirm (destructive Ctrl+key)
Inject -> Normal (Esc or Enter)
Filter -> Normal (Esc or Enter)
Confirm -> Normal (y/n)
```

### Key handling architecture

The `handle_key` function takes 11 parameters that describe the full UI state, and dispatches based on:

1. **Input mode** checked first (Inject/Filter/Confirm intercept all keys)
2. **Modal state** checked next (task picker, task detail, queue overview intercept)
3. **Active tab** checked next (Plans/Agents/Git/Logs/Config/Inspect each have per-tab keybinds)
4. **Focus zone** determines scroll target for up/down/pageup/pagedown
5. **Global actions** (Ctrl+key) are available on all tabs

### Key binding summary (136 TuiAction variants):

**Global (all tabs)**:
- `Ctrl-C`: quit
- `F1-F7`: switch tabs
- `1-7`: switch tabs (number keys)
- `Ctrl-r/x/d/g/a/t`: destructive operations (confirm first)
- `?`: help modal
- `q`: quit
- `p`: pause/resume
- `Tab/Shift+Tab`: cycle focus zones
- `y/n`: approve/reject pending command
- `i`: inject message
- `/`: start filter

**Dashboard-specific**:
- `a/o/d/e/g/m/P`: switch detail sub-tab (Agents/Output/Diff/Errors/Git/MCP/Processes)
- `v`: toggle implementation vs verification pane
- `` ` ``: cycle agent sub-tabs
- `Alt+1-7`: jump to agent tab
- `[/]`: cycle process output tabs
- `w`: wave overview
- `u`: queue overview
- `s/r`: soft-retry plan
- `c`: re-verify plan
- `z`: diagnose plan
- `S/R`: repair plan (preserve/clean)

**Plans tab (F2)**:
- `Up/Down/j/k`: navigate tree
- `Left/Right/h/l`: previous/next wave
- `Enter/Esc`: drill in/out of tree
- `PgUp/PgDn`: 10-item page jump
- `M`: merge selected plan to batch
- `m`: prepare batch-to-main merge

**Agents tab (F3)**:
- `Up/Down/j/k`: navigate agent list
- `End`: resume auto-scroll
- `` ` ``: cycle agent tabs

**Git tab (F4)**:
- `Up/Down/j/k`: navigate branch list
- `Enter`: select branch

**Context-sensitive scroll**:
- `Up/Down` meaning changes based on focus zone (Plans=select, Tasks=scroll, AgentOutput=scroll, CommandOutput=scroll)
- `PageUp/PageDown` is modal-aware (scrolls modal if one is open)

### Mouse support

Full mouse support via `EnableMouseCapture`:
- `MouseClick { x, y }`: zone detection for click-to-focus
- `MouseScrollUp/Down { x, y }`: zone-aware scroll
- Hit zones computed from layout for plan tree, sub-tab bar, right content, etc.

---

## 8. How does mori display agent output streaming?

### Agent output widget (`widgets/agent_output.rs`, ~700 lines)

**Two rendering modes:**

1. **Implementation mode** (`AgentPaneGroup::Implementation`):
   - Tab bar showing role tabs: `1:strategist 2:implementer 3:architect 4:auditor 5:scribe 6:critic 7:conductor`
   - Active tab highlighted with accent color
   - Output text shows selected role's transcript

2. **Verification mode** (`AgentPaneGroup::Verification`):
   - Shows per-plan verify chain status panels
   - Tabs for different verification runs
   - Status badges: checkmark/cross/spinner/pending

**Output styling:**
- Error lines (`error`, `fail`): EMBER color
- Success lines (`pass`, `ok`, checkmark): SAGE color
- Headings (`# `, `## `): BONE + BOLD
- Quotes (`> `, `>> `): DREAM color
- Default: FG_DIM

**Word wrapping:**
- Manual word wrap to `area.width - 4` characters
- Long lines are hard-broken at the width boundary

**Auto-scroll behavior:**
- `agent_scroll: Option<usize>` -- `None` means auto-scroll (tail)
- Any manual scroll sets `Some(offset)`, pausing auto-scroll
- `End` key or `Space` resets to `None` (resumes auto-scroll)
- When manually scrolled, first line becomes `"X lines above"` indicator
- When at non-tail position, bottom line shows `"[End] to resume auto-scroll"`

**Scrollbar:**
- Rendered via `scrollbar::render_scrollbar` with role accent color
- Only appears when total lines exceed visible height

**Parallel mode:**
- When `parallel_agents` is non-empty, shows per-instance agent tabs instead of role tabs
- Each instance has its own output buffer and scroll state
- Detail header shows instance ID, model, provider, routing source

---

## 9. What's mori's status bar like?

### Header bar (`widgets/header_bar.rs`)

**Left section** (all on BG_SECONDARY background):
1. Heartbeat animation (4-frame pulsing dot)
2. App name "mori" in ROSE BOLD
3. Wave indicator: "Wave 2/5" in BONE
4. Queue source label in DREAM (if present)
5. Separator `|` in TEXT_PHANTOM
6. 15-cell gradient progress bar
7. Plan count "12/30" with semantic color
8. Percentage "40%" (hidden when compact)
9. In-flight agent count "3>" in ROSE_DIM
10. Separator
11. ETA "ETA:2h15m" in DREAM
12. Elapsed "1h30m" in FG_DIM
13. Cost "$12.34" in BONE_DIM
14. Token count "45K tok" in FG_DIM
15. MCP status "MCP:14" green/warning
16. Separator
17. System metrics: `C:42% M:8G N:12K D:4M` with threshold colors
18. Separator
19. Active agent spinner with role short name and model

**Right section** (Layout-separated):
- F-key strip: `F1:dash F2:plans F3:agents F4:git F5:logs F6:cfg F7:inspect F8:queue`
- Active tab shown inverted (dark text on accent bg)
- Queue modal tab highlighted when open

### Status bar (`widgets/status_bar.rs`)

1. Git branch name in BONE
2. Commit hash (7 chars) in TEXT_GHOST
3. Last commit age ("2m ago") in TEXT_GHOST
4. Separator
5. Heartbeat dot
6. PAUSED indicator (if paused, inverted yellow)
7. Progress text (COMPLETE/ERR:msg/N/M)
8. Health: active plans, live agents, flailing count, retry count, failed count
9. Separator
10. Main merge indicator ("main:3 @abc1234")
11. Separator
12. Context-sensitive keybind hints (changes per tab and focus zone)

---

## 10. What's mori's error/recovery flow?

### Error display hierarchy

1. **Alert ribbon**: 1-line strip between header and content. Shows highest-priority:
   - Error message (truncated to 120 chars) in EMBER
   - Gate failure detection ("test failures detected -- check gate output") in EMBER
   - First preflight warning in WARNING

2. **Status bar error**: Shows `ERR: {first 30 chars}` in error style

3. **Header bar error**: Shows `ERR:{first 16 chars}` in EMBER BOLD

4. **Error badge on sub-tab**: Errors tab shows count badge `2x` in EMBER

5. **Error digest widget** (`widgets/error_digest.rs`): Dedicated panel showing:
   - Structured error parsing from gate output
   - Error count in title
   - Filterable error lines

6. **Toast notifications**: Errors surface as toasts stacked bottom-right with EMBER border

### Recovery actions available from TUI

| Key | Action | Confirmation |
|-----|--------|-------------|
| `s` / `r` | Soft-retry selected plan | ConfirmAction::SoftRetryPlan |
| `z` | Diagnose selected plan (write recovery report) | ConfirmAction::DiagnosePlan |
| `S` | Repair plan preserving work | ConfirmAction::RepairPlanPreserve |
| `R` | Repair plan from clean slate | ConfirmAction::RepairPlanClean |
| `c` | Re-verify plan (re-run gates) | ConfirmAction::ReverifyPlan |
| `Ctrl-r` | Restart ALL plans from scratch | ConfirmAction::RestartAllPlans |
| `Ctrl-d` | Reset selected plan (delete worktree) | ConfirmAction::ResetSelectedPlan |
| `Ctrl-x` | Force commit and advance | ConfirmAction::ForceAdvance |
| `Ctrl-g` | Git reconcile (merge, tag, prune) | ConfirmAction::GitReconcile |

All destructive actions require confirmation via `y/n` in the confirm modal.

---

## Summary: Key Gaps for Roko

### High-priority gaps (functional impact):

1. **Scrollbar widget**: Mori has a clean, reusable scrollbar. Roko has no scrollbar -- panels that scroll have no visual position indicator. This is a usability issue.

2. **Status badge widget**: Mori's `badge("text", Status)` is used everywhere for consistent status coloring. Roko inlines these patterns inconsistently.

3. **Agent output auto-scroll indicator**: Mori shows "X lines above" and "[End] to resume" when manually scrolled. Essential for long agent transcripts.

4. **Context-sensitive status bar hints**: Mori's status bar changes keybind hints based on active tab and focus zone. This is how users discover available actions.

5. **Alert ribbon**: Mori's 1-line alert between header and content ensures errors are always visible regardless of which tab is active.

### Medium-priority gaps (visual polish):

6. **Background viz layer**: State-driven background coloring on Dashboard. Progress fills from bottom, error shifts to red, activity brightens.

7. **Panel shadows**: 1-cell drop shadows on right and bottom of panels. Adds significant visual depth.

8. **Content-aware sub-tab badges**: Live counts on inactive tabs (agent count, error count, MCP calls, CPU%).

9. **Gradient progress bar in header**: 15-cell fire gradient bar with semantic color shifting.

10. **System metrics in header**: Compact CPU/MEM/NET/DISK readout with threshold coloring.

### Lower-priority gaps (atmosphere/VFX):

11. **NERV data viz**: Braille sub-pixel visualizations driven by system state. Unique visual identity.

12. **Particle physics**: Full force-field particle system. Cool but CPU-expensive.

13. **VFX field library**: Plasma, voronoi, FBM noise generators. Used by NERV viz.

14. **Dream atmosphere**: Combined vignette + film grain + breathing + rose tint pass.

15. **Modal glow**: Colored glow halo around modal borders.

### Architecture observations:

- Mori's `RunState` is a single large struct with all TUI state. Roko splits this into `TuiState` + `DashboardData` which is cleaner but means some mori patterns (like cross-cutting badge computation) need adapter methods.

- Mori's render function in `layout.rs` is the single entry point that dispatches everything. Roko's equivalent is `App::draw` in `app.rs`. Both follow the same pattern.

- Mori uses `crossterm::EnableMouseCapture` for mouse support. Roko also does this. Both implement hit-test zone detection.

- Mori has 7 tabs (F1-F7). Roko has 10 tabs (F1-F10) with Marketplace, Atelier, and Learning additions.

- Both share the ROSEDUST palette with the same core color values.

- Both have the same modal set (roko actually has all 14 including quit.rs).
