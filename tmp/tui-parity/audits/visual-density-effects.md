# Visual Density and Effects Audit

Consolidated from reports 08 (visual density) and 10 (effects/atmosphere).
Date: 2026-08-31

---

## Live visual re-audit and implementation status (17:37 run)

The original audit below was based primarily on static code inspection. The
subsequent live run with `preset = "full"` exposed a more important rule that
supersedes several of its prescriptions: **blank widget cells are layout, not
an unrestricted effects canvas**.

Roko's old Full pipeline wrote dense braille glyphs for progress fields, guide
lines, data rain, ripples, and particles into every blank cell across the whole
content area. Because table padding and paragraph whitespace are also blank
cells, the effects obscured borders, indentation, agent output, task rows, and
system metrics. It also ran after modal rendering. The result is captured in
the 17:37 screenshot and is not acceptable even as an opt-in "Full" mode.

Direct comparison with the current Mori implementation in
`/Users/will/dev/uniswap/bardo/apps/mori/src/tui` established the new baseline:

- true-black frame canvas and restrained raised surfaces;
- one-cell VOID gutters between master/detail panes;
- background- or existing-foreground-only post-processing;
- sparse particles in deep negative space, not continuous screen-wide rain;
- effects rendered before dimming and modals;
- content-sized agent/panel regions, with token/system metrics local to the
  Agents detail instead of a global ribbon;
- focused borders are bright and bold; unfocused borders recede to phantom.

### Implemented in the follow-up working tree

| Area | Resolution |
|---|---|
| Full NervViz | Retained, but converted to a low-contrast background-only state field. It never mutates symbols or foregrounds. |
| Particles | Retained, capped at 24, low-contrast, and require a clear 3x3 blank neighborhood. |
| Render order | Canvas -> widgets -> effects -> modal dim -> modals. Effects can no longer overwrite modal content. |
| Palette | Aligned to Mori's true black, boosted ROSEDUST accents, readable text, raised surfaces, and phantom inactive chrome. |
| Dashboard | Added 38/1/rest master-detail split, content-proportional left sections, content-sized agent pool/routes/output, and local token/system strip. |
| Plans / Agents | Added Mori VOID gutters; collapsed Plans' duplicated pipeline/summary chrome into one inline pipeline row; removed the duplicated full-width Agents status grid so transcript space remains primary. |
| Logs | Reclaimed the unused second status row and made inactive chrome recede. |
| Diagnosis | Empty diagnoses no longer reserve a six-row panel; the section appears only when the conductor has a diagnosis. |
| Defaults | Minimal remains the default; Mori-style screen self-glow is on by default unless explicitly disabled. Reduced-motion still disables the stack. |

### Verification

- Integrated `cargo test -p roko-cli --lib 'tui::'`: **301 passed, 0 failed**.
- A 180x55 active-agent `TestBackend` render with Full effects verifies the
  operational transcript survives and caps braille particles at 32 cells.
- Background-only and 3x3 particle-clearance tests enforce the two effect
  invariants that the 17:37 render violated.
- The agent-route regression verifies content rows remain visible at 100x24
  after hiding the empty diagnosis panel.

The remaining density items below are retained as backlog/history. VD-01,
VD-04, VD-05, VD-06, VD-12, VD-14, and VD-15 were resolved in this pass. VD-03
remains a possible compact-terminal refinement, although Mori's current
dashboard also uses distinct Plan/Phase/Tasks regions at large sizes.

## Critical Issues

### VD-01: Bottom ribbon height wastes content rows

- **Severity**: Critical
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs:66`
- **User sees**: Bottom ribbon consumes 6 rows (25% of a 24-row terminal). Three side-by-side widgets each get ~27x4 inner cells. The token sparkline renders one summary line, one sparkline, and maybe one tier bar in that space.
- **Mori shows**: Bottom ribbon packed into 3-4 rows.
- **Fix**: Change `Constraint::Length(6)` to `Constraint::Length(4)` on line 66. The sparkline and sys_metrics renderers already handle reduced height gracefully (they conditionally omit summary line 2 and tier rows when height < 4).
- **Row recovery**: +2 rows on every screen.
- **Backlog**: #9 (Batch 8: Visual polish)

### VD-02: Phase compact always allocates 4 rows

- **Severity**: Critical
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs:83-84` (left panel layout)
- **Widget**: `crates/roko-cli/src/tui/widgets/phase_compact.rs`
- **User sees**: 4 rows permanently reserved for the phase indicator in the left panel, even when no plan is active. Shows an empty phase bar.
- **Mori shows**: No dedicated phase section; phase info embedded inline.
- **Fix**: When `tui_state.current_phase.is_empty()` or no plan is active, set the phase constraint to `Length(0)` instead of `Length(4)`. The plan_tree and task_progress sections get those rows back.
- **Row recovery**: +4 rows when idle.
- **Backlog**: #9 (Batch 8: Visual polish)

### VD-03: Triple-bordered left panel wastes 33% of panel area

- **Severity**: Critical
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs:83-130` (left panel rendering)
- **Widgets**: `plan_tree.rs`, `phase_compact.rs`, `task_progress.rs` (each uses `Block::default().borders(Borders::ALL)`)
- **User sees**: Three stacked bordered panels in the left column. Each border costs 2 rows (top + bottom). With 3 panels: 6 border rows consumed out of ~18 available = 33% gone to chrome.
- **Mori shows**: Single continuous left panel with thin horizontal rules between sections, not full border boxes.
- **Fix**: Render one outer `Block` with `Borders::ALL` for the entire left panel. Replace internal block borders with single-line horizontal dividers (`Line::from("---")` or `Block` with `Borders::TOP` only on the second and third sections). Net savings: 4 border rows.
- **Row recovery**: +4 rows in left panel.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-04: Plans view pipeline header + plan summary = 7 fixed rows

- **Severity**: Critical
- **File**: `crates/roko-cli/src/tui/views/plans_view.rs:64-68`
- **User sees**: Pipeline header (3 rows) + plan summary (4 rows) = 7 fixed rows at the top of the left panel. On a 24-row terminal with 22 usable rows, only 15 remain for the plan list. Plans numbering 30+ are severely truncated.
- **Mori shows**: 1-row header, fits 30+ plans.
- **Fix**: Merge the pipeline header into a single-row compact summary (e.g., `"Pipeline: 3 plans | 12/30 tasks | Wave 2/4"`). Collapse the plan summary to 0 rows when no plan is selected; show it as a 2-row inline when a plan is selected. Net: 7 rows becomes 1-3 rows.
- **Row recovery**: +4 to +6 rows in F2 Plans.
- **Backlog**: NEEDS NEW BACKLOG ITEM

---

## Important Issues

### VD-05: Default effects preset is Off -- TUI feels static

- **Severity**: Important
- **File**: `crates/roko-cli/src/tui/effects_config.rs:9`
- **User sees**: With `EffectsPreset::Off` (the default), the only visible animation is the header pulsing dot, atmosphere breathing (very subtle 0.9-1.0 luminance oscillation), and task progress heartbeat pulse. No particles, no background visual activity, no gradient shifts on borders. The TUI feels like a static log viewer.
- **Mori shows**: Always-on particles and shimmer, creating a sense of activity even during idle moments.
- **Fix**: Change line 9 from `#[default] Off` to `#[default] Minimal`. The Minimal preset enables floating particles (braille dots that rise and drift) only when agents are active, writes only to blank cells, and has negligible performance cost. `ROKO_REDUCED_MOTION` still disables everything.
- **Backlog**: #48 (completed) or NEEDS NEW BACKLOG ITEM for the default change

### VD-06: screen_postfx requires double opt-in beyond Full preset

- **Severity**: Important
- **File**: `crates/roko-cli/src/tui/effects_config.rs:64-77`
- **User sees**: Even with `EffectsPreset::Full`, bloom, drop shadows, ambient orbs, dream atmosphere, and self-glow are all disabled. They require an explicit `screen_postfx = true` in `roko.toml` because `apply_preset(Full)` does not set `screen_postfx`, `shadows_enabled`, or `vfx_enabled` to true.
- **Mori shows**: Atmospheric effects visible at the equivalent of "Full" mode.
- **Fix**: In the `apply_preset(Full)` method, set `screen_postfx = true`, `shadows_enabled = true`, and `vfx_enabled = true`. Keep `bloom_enabled = false` for performance (bloom requires screen-blend across all cells). Users who want Full should get the full experience without editing TOML.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-07: Header + status bar duplicate task count, cost, and agent count

- **Severity**: Important
- **Files**: `crates/roko-cli/src/tui/widgets/header_bar.rs:298-361`, `crates/roko-cli/src/tui/widgets/status_bar.rs:67-140`
- **User sees**: Three data points appear in both the header bar and status bar simultaneously: task done/total count, cost `$X.XX`, and active agent count `Nag`. Both bars are always visible, so the exact same numbers occupy two rows.
- **Mori shows**: Header and footer showed complementary data, not duplicates.
- **Fix**: Keep task count and cost in the header (where they anchor the progress bar context). In the status bar, replace the duplicated raw values with complementary metrics: throughput rate (tasks/hour), ETA, and most recent gate verdict. The status bar already has cost-with-budget (`$X / $budget (Y%)`), which adds context -- remove the header's bare `$X.XX` in favor of the status bar's budget-relative version.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-08: Muted-dominant color scheme creates monochrome appearance

- **Severity**: Important
- **File**: `crates/roko-cli/src/tui/theme.rs`
- **User sees**: Many text elements default to `theme.muted()` (TEXT_GHOST #6a5a68), creating a wall of same-hue dim text. Sub-tab bar uses only 2 color states (active = accent background, inactive = muted). Table headers universally use `theme.accent()` (ROSE #aa7088). During an active run, only 8-10 distinct hues are visible.
- **Mori shows**: 12-14 distinct hues. Tokens in cyan, cost in yellow, timing in dim white. Each sub-tab has its own color.
- **Fix**: Assign distinct hues to data categories across all widgets:
  - Tokens: DREAM (#8a7aaa or similar purple)
  - Cost: WARNING (amber)
  - Timing/ETA: BONE (#c8b8a0)
  - Gate results: SAGE (green)
  - Error counts: EMBER (red)
  - Sub-tab labels: each gets its own role_accent color (agents=SAGE, diff=EMBER, verify=WARNING, etc.)
  - Table headers: vary by content type rather than universal ROSE
- **Backlog**: #49 (completed theme alignment) -- NEEDS NEW BACKLOG ITEM for per-category hue assignment

### VD-09: PAUSED and HALTED indicators use text-only styling, not badges

- **Severity**: Important
- **Files**: `crates/roko-cli/src/tui/widgets/status_bar.rs:67-78` (PAUSED), `crates/roko-cli/src/tui/widgets/phase_compact.rs` (HALTED)
- **User sees**: PAUSED is bold amber text on the standard dark bar background. HALTED is bold ember text. Neither has a contrasting background fill. They blend into the bar and are easy to miss.
- **Mori shows**: Inverted badge styling (dark text on bright background) making PAUSED pop aggressively.
- **Fix**: Change PAUSED from `fg(WARNING)` to `fg(Theme::VOID).bg(Theme::WARNING).add_modifier(Modifier::BOLD)`. Same for HALTED: `fg(Theme::VOID).bg(Theme::EMBER)`. The task progress badges (RUN, DONE, FAIL, WAIT) already use this inverted pattern correctly (task_progress.rs:295-303).
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-10: Header progress bar is single-color, not per-cell gradient

- **Severity**: Important
- **File**: `crates/roko-cli/src/tui/widgets/header_bar.rs:219-242`
- **User sees**: The header progress bar samples `gradient_fire()` once at the current completion fraction. At 50% the entire filled portion is amber. Each cell is the same color.
- **Mori shows**: Per-cell gradient coloring -- cell 0 is one color, cell N is another, creating a visual sweep.
- **Fix**: In the bar rendering loop, sample `gradient_fire(cell_position / bar_width)` per cell instead of once for the whole fill. The wave_progress bar already does this with `gradient_ocean()` at wave_progress.rs (per-cell animated ocean gradient), so the pattern exists in the codebase.
- **Backlog**: #14 (completed header parity) -- NEEDS NEW BACKLOG ITEM for gradient enhancement

### VD-11: Four identical "waiting for trend data" in Learning sub-tab

- **Severity**: Important
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs:1237`
- **User sees**: When the Learning sub-tab is active and data is sparse, 4 stacked sparkline panels each show `" waiting for trend data"` in muted text. The entire screen is repeated identical muted text -- visually monotonous and wasteful.
- **Mori shows**: N/A (no equivalent), but generic repetition is a density anti-pattern.
- **Fix**: Replace the four individual panels with a single consolidated message: `"collecting efficiency data -- trends appear after first hour"`. Show one small progress indicator instead of four copies. When partial data arrives (e.g., 1 of 4 metrics has points), show that one and collapse the others.
- **Backlog**: NEEDS NEW BACKLOG ITEM

---

## Minor Issues

### VD-12: Empty-state vertical centering wastes rows

- **Severity**: Minor
- **Files**: `crates/roko-cli/src/tui/views/plans_view.rs:244-265`, `crates/roko-cli/src/tui/views/agents_view.rs:179-204`
- **User sees**: Empty states center text vertically in their panels. On a 15-row panel, 7 rows of blank padding appear above the message.
- **Mori shows**: Empty states pinned to the top, leaving available space below.
- **Fix**: Remove vertical centering from empty state rendering. Pin messages to line 1-2 of the panel. The remaining space below stays visually empty but does not push the message into the middle of an otherwise blank panel.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-13: Sub-tab bar uses only 2 color states

- **Severity**: Minor
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs` (sub-tab rendering)
- **User sees**: All inactive sub-tabs look identical (muted text). Active tab has accent background. No visual differentiation between the 8 sub-tabs.
- **Mori shows**: Color-coded tabs.
- **Fix**: Assign each sub-tab its own `role_accent` color for the label text when inactive. Use that color as background when active. Map: Agents=SAGE, Output=BONE, Diff=EMBER, Verify=WARNING, Learning=DREAM, Processes=TEXT_DIM, MCP=ROSE_BRIGHT, Affect=DREAM_REM.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-14: Wave progress widget area stays allocated when empty

- **Severity**: Minor
- **File**: `crates/roko-cli/src/tui/widgets/wave_progress.rs:22-24` (early return)
- **User sees**: When no waves are configured, the wave progress widget returns early but its 40% horizontal slice of the bottom ribbon remains blank -- a 27x4 empty area.
- **Mori shows**: N/A (mori had no wave concept).
- **Fix**: In `render_bottom_ribbon()` (dashboard_view.rs:1919), check whether waves exist before allocating the 40% slice. When empty, redistribute the space: give 60% to the efficiency sparkline and 40% to sys_metrics (or 70/30).
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-15: Diagnosis panel shows long explanatory text when empty

- **Severity**: Minor
- **File**: `crates/roko-cli/src/tui/views/dashboard_view.rs` (diagnosis panel rendering)
- **User sees**: When no conductor diagnoses exist, the panel renders 6-9 rows explaining what the conductor circuit breaker does. This is tutorial text, not status.
- **Mori shows**: Short status: "all systems healthy" or equivalent.
- **Fix**: Replace multi-paragraph explanation with a one-liner: `"all gates healthy -- no conductor alerts"`. The explanation belongs in docs or `roko explain conductor`, not in a runtime panel.
- **Backlog**: NEEDS NEW BACKLOG ITEM

### VD-16: Border color does not animate with state

- **Severity**: Minor
- **Files**: `crates/roko-cli/src/tui/theme.rs`, all view files using `Block::default().borders()`
- **User sees**: Focused panels get ROSE borders, unfocused get TEXT_DIM. These are static assignments -- no animated transitions between states.
- **Mori shows**: Border colors that shift subtly with state over time.
- **Fix**: Modulate the focused border color by `atmosphere.breathing_brightness()` to produce a subtle animated pulse on the active panel border. Example: `brighten(Theme::ROSE, state.atmosphere.breathing_brightness())` applied to the border style. Keep unfocused borders static.
- **Backlog**: NEEDS NEW BACKLOG ITEM

---

## Row Recovery Summary (80x24)

If VD-01 through VD-04 are addressed:

| Fix | Rows Recovered |
|---|---|
| VD-01: Bottom ribbon 6 to 4 | +2 |
| VD-02: Phase compact collapse when idle | +4 |
| VD-03: Shared border for left panel sections | +4 |
| VD-04: Plans header 7 to 2 | +5 |
| **Total potential** | **+15 rows** |

On a 24-row terminal with ~20 content rows, recovering 15 rows would nearly double the usable content area. Even VD-01 + VD-02 alone (+6 rows) is significant.

---

## Effects Stack Summary (current working tree)

### What runs at each preset level

| Preset | Visible Effects |
|---|---|
| **Off** | Static UI plus widget-local status animations; no screen PostFX or particles. |
| **Minimal (default)** | Restrained self-glow plus sparse particles that require deep blank space around them. |
| **Full** | Minimal plus a low-contrast, state-driven background field. No effect changes cell symbols or foreground text. |
| **Reduced motion** | `ROKO_REDUCED_MOTION` forces the complete effect stack off. |

### Always-on regardless of preset

| Effect | Location | Notes |
|---|---|---|
| Heartbeat timing | `atmosphere.rs` heartbeat() | Double-pulse per 1.5s cycle, used by dot/task progress/phase bar |
| Spinners | `atmosphere.rs` spinner() | 10-frame braille spinner + 4-frame ethereal spinner |

### Correctly matching mori

| Feature | Status |
|---|---|
| Health-aware pulsing dot | MATCH -- double-beat rhythm, 4 health-color states |
| Braille sparklines (token + sys) | MATCH -- 2x horizontal density via Unicode braille |
| System gauge shimmer | BETTER -- per-cell sinusoidal ripple animation |
| Wave progress animation | MATCH -- per-cell animated ocean gradient |
| Task progress pulse | CLOSE -- heartbeat brightness oscillation instead of per-cell gradient |
| Reduced motion support | BETTER -- `ROKO_REDUCED_MOTION` env var |
| High contrast support | BETTER -- `ROKO_HIGH_CONTRAST` + `NO_COLOR` env vars |
| Daimon affect containment | CLEAN -- properly bounded within its Rect, no bleed-through |

---

## File Reference

| File | Controls |
|---|---|
| `crates/roko-cli/src/tui/theme.rs` | 22 named colors, 8 semantic styles, 2 gradients, role/phase accent maps |
| `crates/roko-cli/src/tui/layout.rs` | responsive_outer_margin, split helpers |
| `crates/roko-cli/src/tui/app.rs` | Top-level layout: header(1) + wave(0-1) + content(Min) + footer(1) |
| `crates/roko-cli/src/tui/effects_config.rs` | EffectsPreset enum (Off/Minimal/Full), EffectsConfig struct |
| `crates/roko-cli/src/tui/postfx.rs` | Background-only state field and clearance-aware sparse particles; legacy helpers remain covered by unit tests |
| `crates/roko-cli/src/tui/postfx_pipeline.rs` | Self-glow effect |
| `crates/roko-cli/src/tui/atmosphere.rs` | Widget-local breathing values, heartbeat, and spinners; no full-frame rewrite |
| `crates/roko-cli/src/tui/views/dashboard_view.rs` | F1 Mori-style master/detail layout and sub-tabs; metrics are local to Agents |
| `crates/roko-cli/src/tui/views/plans_view.rs` | F2 plan tree with compact inline pipeline summary and full detail pane |
| `crates/roko-cli/src/tui/views/agents_view.rs` | F3 role roster and transcript-first detail pane |
| `crates/roko-cli/src/tui/views/learning_view.rs` | F10 layout, trend sparklines |
| `crates/roko-cli/src/tui/widgets/header_bar.rs` | Pulsing dot, gradient progress bar, token/cost counters |
| `crates/roko-cli/src/tui/widgets/status_bar.rs` | PAUSED badge, cost/budget, keybind hints |
| `crates/roko-cli/src/tui/widgets/plan_tree.rs` | Collapsible wave/plan tree |
| `crates/roko-cli/src/tui/widgets/task_progress.rs` | Scrollable task list, RUN/DONE/FAIL/WAIT badges |
| `crates/roko-cli/src/tui/widgets/phase_compact.rs` | Phase indicator, HALTED badge, ethereal spinner |
| `crates/roko-cli/src/tui/widgets/wave_progress.rs` | Animated ocean-gradient wave segments |
| `crates/roko-cli/src/tui/widgets/token_sparkline.rs` | Efficiency summary + braille sparkline + tier bars |
| `crates/roko-cli/src/tui/widgets/sys_metrics.rs` | CPU/MEM/NET/DSK/FPS with gauges + sparklines |
| `crates/roko-cli/src/tui/views/affect_view.rs` | Daimon affect panel (properly contained) |
