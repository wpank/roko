# 24 - Layout System and Responsive Design Audit

**Date:** 2026-09-01
**Scope:** `crates/roko-cli/src/tui/` -- layout module, app.rs main draw path, all 10 view files, header bar, hit_test, modals

---

## 1. Main Layout: How Is the Screen Divided?

**File:** `app.rs` lines 1017-1030

The main draw path splits the terminal into a five-row vertical layout:

```
+------------------------------------------+
| Header bar (1 row)                       |  Constraint::Length(1)
+------------------------------------------+
| Warning bar (0 or 1 row)                 |  Constraint::Length(warning_height)
+------------------------------------------+
| Wave indicator (0 or 1 row)              |  Constraint::Length(wave_row_height)
+------------------------------------------+
| Content area (flexible)                  |  Constraint::Min(0)
+------------------------------------------+
| Status footer (1 row)                    |  Constraint::Length(1)
+------------------------------------------+
```

The warning and wave rows are conditionally hidden (height 0) when inactive. Before this layout is applied, `responsive_outer_margin()` optionally insets the terminal by 1 cell on all sides for terminals >= 120x50.

The content area is further split by `split_content_area()` when a text input mode is active, stealing 1 row from the bottom for the input bar.

**Verdict:** Clean and correct. The fixed chrome (header + footer) takes 2-4 rows, leaving maximum space for content. The conditional warning/wave rows avoid wasting vertical space when idle.

---

## 2. Fixed vs Dynamic Sizing: Constraint Patterns

### Constraint::Length (fixed rows)

Used extensively and appropriately for:
- **Chrome:** Header bar (1), footer (1), warning bar (0-1), wave bar (0-1), sub-tab bars (1), input bar (1)
- **Widget-specific fixed rows:** Affect gauges (2 each), learning stage indicator (5), bar charts (6-8), stats bars (3), metadata tables (7-8)
- **Table columns:** Many hardcoded column widths (6-14 cells) in tables throughout context_view, dashboard_view, plans_view, config_view, marketplace_view

### Constraint::Min(0)

Used as the flexible "fill remaining space" element in nearly every layout. This is the correct pattern for the primary scrollable content area in each view.

### Constraint::Percentage

Used for panel splits:
- Dashboard left/right: 38% / Min(0)
- Plans left/right: 31% / 1px gutter / Min(0)
- Agents left/right: 32% / 1px gutter / Min(0)
- Git left/right: 35% / 65%
- Atelier: 40% / 60%
- Marketplace: 35% / 65%
- Context view: 20% / 40% / 40% (vertical) with 50%/50% horizontal sub-splits
- Three-panel inspect: 33% / 34% / 33%

### Content-aware sizing (dynamic)

Dashboard left panel (`dashboard_view.rs` lines 91-113) implements genuine content-aware sizing: it calculates `plan_h` and `task_h` based on actual plan/task counts, with clamped minimums. This is the best layout logic in the codebase.

**Verdict:** The mix of Length/Min/Percentage is standard ratatui practice. Table column widths via `Length` are the main fragility point -- they will clip or overflow on narrow terminals.

---

## 3. Small Terminal Behavior (Below 80x24)

### What breaks at 80x24:

1. **Header bar compression is partial.** The `compact` mode triggers below 120 columns (hides percentage display, MCP/NET/DSK/FPS stats, drops colons from CPU/MEM labels). But at 80 columns, the F-key strip alone (`F1:dash F2:plans F3:agents F4:git F5:logs F6:cfg F7:inspect F8:market F9:atelier`) consumes approximately 85+ characters before badges. The header uses `Constraint::Min(0)` for left content and `Constraint::Length(fkey_width)` for the F-key strip -- at 80 columns the left content (status dot, name, progress, elapsed, CPU, MEM, agents, gates) will be severely truncated or invisible.

2. **Sidebar panels are too wide.** Plans uses 31% = 24 columns for the wave tree. Agents uses 32% = 25 columns. Dashboard uses 38% = 30 columns. At 80 columns, the right detail panel gets only 54-56 columns minus the 1-cell gutter, leaving ~53 columns for agent output or task detail. This is marginally usable but table columns will overflow.

3. **Sub-tab bar at 80 columns.** The dashboard sub-tab bar renders 8 labels (` a:Agents  o:Output  d:Diff  e:Verify  g:Git  m:MCP  L:Learning  P:Procs `) plus hints -- approximately 85 characters. The gap-fill logic (`area.width.saturating_sub(used + hint.len())`) will produce a gap of 0, but the line still exceeds 80 columns and will be clipped silently by the terminal.

4. **Fixed table column widths.** Many tables use `Constraint::Length(10)` or similar for columns. At 80 columns with a sidebar, the right panel is ~53 columns with borders, leaving ~51 usable. A table with columns summing to 40+ cells will barely fit.

5. **Vertical space.** At 24 rows, chrome takes 2-4 rows (header + footer + optional warning/wave). Content gets 20-22 rows. Affect view needs 2+2+2+2+Min = 8 fixed rows minimum. Learning router needs 5+6+Min = 11 fixed rows minimum. These work but leave very little scrollable area.

### What breaks below 80x24 (e.g., 60x20):

1. **Header bar is nearly invisible.** The F-key strip alone needs 80+ chars.
2. **Sidebars become degenerate.** 31% of 60 = 18 columns -- barely enough for a plan name.
3. **Modal centering math uses integer division** and `centered_rect_fixed` clamps to `min(area.width)`, so modals will scale down but may be illegible.
4. **Responsive outer margin is correctly skipped** (threshold: 120x50).

**Verdict: 80x24 is marginal. The header bar and sub-tab bar will clip. Sidebars work but are tight. Below 80x24, the TUI is unusable for most tabs.**

---

## 4. Wide Terminal Behavior (200+ Columns)

### Space utilization:

1. **Responsive outer margin adds only 1 cell per side** at 120x50+. At 200 columns this leaves 198 usable columns. The margin is negligible and appropriate.

2. **Percentage-based panel splits scale proportionally.** Dashboard left panel at 38% of 198 = 75 columns for the plan tree. This is far more than needed for a list of plan names. The right panel gets ~122 columns -- also excessive for most content.

3. **Tables with `Constraint::Length` columns do NOT expand.** A table with columns summing to 60 cells will leave 60+ columns of empty space on a 200-column right panel. The `Min(0)` fill column absorbs the remainder, but only one column can flex.

4. **The header bar fills width correctly.** The left content and F-key strip use `Min(0)` / `Length(fkey_width)` split, so the left spans grow to fill. But the individual span elements are fixed-width text, so the gap between left content and F-key strip becomes a large empty region.

5. **Three-panel views (inspect 33/34/33, context 50/50) scale well** -- each panel gets ample room.

6. **No maximum width constraint exists.** The layout will stretch to any terminal width. Mori had the same behavior.

**Verdict: Wide terminals work but produce excessive whitespace in sidebar columns and between header bar elements. Tables do not take advantage of extra width -- they remain fixed-width with a single flex column.**

---

## 5. Panel Splits: Horizontal vs Vertical -- Are They Optimal?

| View | Split | Ratio | Assessment |
|------|-------|-------|------------|
| Dashboard (F1) | Horizontal L/R | 38% / gutter / Min | Correct for master-detail. Left panel conditional on active plans -- good. |
| Plans (F2) | Horizontal L/R | 31% / gutter / Min | Standard tree+detail. 31% is reasonable. |
| Agents (F3) | Horizontal L/R | 32% / gutter / Min | Standard roster+output. |
| Git (F4) | Horizontal L/R | 35% / 65% | No gutter. The two-percentage split can lose a pixel to rounding vs the gutter+Min pattern used elsewhere -- minor inconsistency. |
| Logs (F5) | Vertical only | status bar + Min | Full-width log view. Correct. |
| Config (F6) | Vertical only | (not audited in detail) | |
| Inspect (F7) | Varies by sub-tab | 20/40/40 vertical, 50/50 horizontal, 33/34/33 | The mix of layouts per sub-tab is good. |
| Marketplace (F8) | Horizontal L/R | 35% / 65% | Same pattern as Git (no gutter). |
| Atelier (F9) | Horizontal L/R | 40% / 60% | No gutter. |
| Learning (F10) | Vertical only | fixed + Min + fixed | Correct for data-centric view. |

**Inconsistency: Three views (Dashboard, Plans, Agents) use a 1-cell VOID gutter between panels. Four views (Git, Marketplace, Atelier, Context) use no gutter.** The gutter pattern is better for visual separation but should be consistent.

**Verdict: The panel splits are reasonable. The master-detail horizontal pattern is appropriate for all list+detail views. The inconsistent gutter usage is a minor issue.**

---

## 6. Panel Resize Capability (Drag Borders)

**There is no panel resize capability.** Searched for `drag`, `resiz` (as in user-initiated resize), `split_ratio`, `panel_width`, `sidebar_width` -- no hits related to user-adjustable panel boundaries.

The only resize handling is the `Event::Resize` handler in `app.rs` (lines 552, 962-963) which updates `self.terminal_size` and triggers a redraw. This handles terminal window resizing correctly but provides no user-driven panel resizing.

**Verdict: No panel drag/resize. This is standard for ratatui TUIs and not necessarily a gap -- but it limits power users who want a wider sidebar or wider output panel.**

---

## 7. Does the Layout Respect Terminal Size on Resize?

**Yes.** The resize handling chain is:

1. Crossterm emits `Event::Resize(width, height)` when the terminal window changes size.
2. The TUI event handler (`event.rs` line 69-70) converts this to `Event::Resize`.
3. `app.rs` (`run_event_loop` line 962-963, `run_connected_loop` line 552-553) updates `self.terminal_size = (width, height)` and sets `redraw = true`.
4. On the next draw, `frame.area()` returns the new terminal size, and all layout computations re-execute from scratch.
5. `hit_test.rs` `HitZones::compute()` uses the current `area` parameter, so mouse hit zones are recomputed per-frame.

**Verdict: Fully correct. The layout recalculates every draw frame from `frame.area()`, and the stored `terminal_size` is kept in sync for scroll calculations between draws.**

---

## 8. Hardcoded Widths That Should Be Dynamic

### Critical (will clip on narrow terminals):

1. **Table column widths in `context_view.rs`.** Lines 384-387, 481-484, 587-589, 721-723, 805-809, 884-888 use `Constraint::Length(5)` through `Constraint::Length(11)` for table columns. These sum to 28-41 cells per table, leaving no room for the data itself on narrow terminals. Should use at least one `Min(0)` flex column.

2. **Table column widths in `config_view.rs`.** Lines 564-566, 666-669 use `Length(6)` through `Length(10)` for fixed columns.

3. **Table column widths in `dashboard_view.rs`.** Lines 548-550 (`Length(14)`, `Length(10)`), lines 1113-1117, 1410-1412.

4. **Table column widths in `plans_view.rs`.** Lines 902-903, 1004-1008, 1076-1079.

5. **`parallel_pool.rs` widget** (lines 90-94): four fixed columns (`Length(12)`, `Length(10)`, `Length(12)`, `Length(8)`) sum to 42 cells.

### Minor (acceptable for their context):

6. **Header bar `truncate_label` uses hardcoded max=24** (line 131). Acceptable since the header is compact by design.

7. **`centered_rect_fixed(42, 8, area)` for the Quit modal** (line 267). Acceptable -- quit confirmation needs minimal space.

8. **Dashboard `diagnosis_height`** is dynamically computed from content -- this is correct.

9. **Affect view gauges** at `Length(2)` each -- correct for Gauge widgets.

**Verdict: Table column widths are the primary hardcoded-width problem. Most use all-fixed columns with at most one `Min(0)` flex column. On narrow terminals, the table will either clip content or overflow the panel boundary. The fix is to use `Min(N)` instead of `Length(N)` for at least the widest column in each table.**

---

## 9. Tab Bar: Does It Truncate on Narrow Terminals?

**The main F-key tab strip in the header bar does NOT truncate.** It renders all 9 F-key items (F1-F9) as fixed-width spans, computes their total width as `fkey_width`, and uses `Constraint::Length(fkey_width)` for the right portion of the header. The left content portion uses `Constraint::Min(0)` and absorbs the remaining space.

At 80 columns, the F-key strip needs approximately 85+ characters (including badges). Since `Length(fkey_width)` is a hard constraint, ratatui will allocate all 85+ characters to the right portion and give the left portion 0 or negative width. The result is that **left header content (status dot, name, progress, elapsed, CPU, MEM, agents, gates) will be invisible**, but the F-key strip will render correctly.

At very narrow terminals (e.g., 60 columns), the F-key strip itself will be clipped by the terminal viewport -- ratatui does not wrap or truncate within a `Paragraph` rendered on a single line.

The header bar has a `compact` mode (line 171: `let compact = area.width < 120`) that hides some metrics but does NOT reduce the F-key strip.

**Verdict: The F-key strip does not truncate or collapse. It pushes the left content off-screen at narrow widths. A progressive disclosure approach (hide less-used tabs, abbreviate labels, or switch to a single active-tab indicator) would fix this.**

---

## 10. Sub-Tab Bar: Does It Wrap or Truncate?

**The sub-tab bar does NOT wrap.** It is rendered as a single `Line` of `Span`s inside a `Paragraph` widget on a 1-row area. If the total span width exceeds the area width, the content is silently clipped at the right edge.

The dashboard sub-tab bar (`dashboard_view.rs` lines 196-261) renders 8 sub-tab labels with badges and a trailing hint string. The gap-fill logic (line 255: `let gap = (area.width as usize).saturating_sub(used + hint.len())`) handles the case where the content is wider than the area by producing a gap of 0 -- but the spans are still all rendered, so the rightmost labels and hints may be clipped.

At 80 columns with a 38% left panel and 1-cell gutter, the right panel is ~49 columns. The sub-tab labels sum to approximately 75 characters. The rightmost 2-3 sub-tabs and the hint text will be invisible.

The agents view right panel also has a role tab bar (`agents_view.rs` line 513) with a sub-tab bar taking 1 row.

**Verdict: Sub-tab bars silently clip on narrow terminals. The rightmost tabs become inaccessible by mouse (though keyboard shortcuts still work). A horizontal scroll indicator or dynamic label shortening would fix this.**

---

## 11. Proposed Adaptive Layouts

### Tier 1: Critical fixes (prevent breakage)

**A. Collapsible F-key strip.** At `width < 100`, show only the active tab label and use number keys (1-9) instead of F1-F9. At `width < 80`, hide the strip entirely and rely on keyboard-only navigation.

```
>= 120 cols:  F1:dash  F2:plans  F3:agents  F4:git  ...  (current behavior)
100-119 cols: 1:D 2:P 3:A 4:G 5:L 6:C 7:I 8:M 9:A     (abbreviated)
80-99 cols:   [Dashboard]                    1-9:tabs    (active only + hint)
< 80 cols:    roko [Dashboard]                           (name + active tab)
```

**B. Collapsible sidebars.** At `width < 100`, collapse the left sidebar to show only icons/abbreviations. At `width < 80`, hide the sidebar entirely and switch to a stacked vertical layout (list on top, detail on bottom, toggled by Enter/Escape).

```
>= 120 cols: 38% sidebar / gutter / 62% detail    (current)
100-119:     25% sidebar / gutter / 75% detail     (narrower sidebar)
80-99:       full-width stacked (list or detail)   (toggle mode)
< 80:        full-width detail only                (hide list)
```

**C. Sub-tab bar shortening.** At `width < 100`, shorten sub-tab labels to single characters. At `width < 70`, hide the hint text.

```
>= 100 cols:  a:Agents  o:Output  d:Diff  e:Verify  ...  (current)
70-99 cols:   a o d e g m L P                              (keys only)
< 70 cols:    a o d e g m L P                              (no hint)
```

### Tier 2: Wide-terminal optimization

**D. Maximum content width.** Cap the main content area at 240 columns and center it within the terminal. This prevents excessive whitespace at ultra-wide (300+) terminals.

```rust
fn responsive_outer_margin(area: Rect) -> Rect {
    let max_width = 240u16;
    if area.width > max_width {
        let side = (area.width - max_width) / 2;
        Rect { x: area.x + side, width: max_width, ..area }
    } else if area.width >= 120 && area.height >= 50 {
        // existing 1-cell margin
    } else {
        area
    }
}
```

**E. Table column flex.** Replace at least one `Constraint::Length(N)` per table with `Constraint::Min(N)` so that tables expand to use available width on wide terminals.

**F. Three-column dashboard at 200+.** When `width >= 200`, split the dashboard into three panels (plan tree / task detail / agent output) instead of two. This uses the extra width productively.

### Tier 3: Future considerations

**G. User-adjustable panel splits.** Store split ratios in `TuiState` and allow `[` / `]` keys to widen/narrow the active panel by 5%. Persist to a layout config file.

**H. Consistent gutter usage.** Standardize on the VOID gutter pattern for all master-detail views (currently missing from Git, Marketplace, Atelier, Context views).

**I. Minimum terminal size guard.** On startup, check terminal size and warn (or refuse to start) if below 60x16. Currently the TUI will render garbage at very small sizes.

---

## Summary Table

| # | Question | Status | Severity |
|---|----------|--------|----------|
| 1 | Main layout structure | Clean 5-row vertical, conditional rows | OK |
| 2 | Fixed vs dynamic sizing | Tables use all-fixed columns | Medium |
| 3 | Small terminal (80x24) | Header bar clips, sub-tabs clip, sidebars tight | High |
| 4 | Wide terminal (200+) | Excessive whitespace, tables don't expand | Low |
| 5 | Panel split optimality | Reasonable, inconsistent gutter usage | Low |
| 6 | Panel resize capability | None | Low |
| 7 | Resize handling | Fully correct | OK |
| 8 | Hardcoded widths | ~30 table column definitions are fixed | Medium |
| 9 | Tab bar truncation | F-key strip never truncates, pushes left content off | High |
| 10 | Sub-tab bar behavior | Silently clips rightmost labels | Medium |
| 11 | Adaptive layout proposals | 9 proposals in 3 tiers | -- |

### Files audited

- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` (main draw, resize handling, content area calculations)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/layout.rs` (responsive_outer_margin, centered_rect, split helpers)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/hit_test.rs` (HitZones::compute layout replay)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs` (10 tabs, F1-F10)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs` (header bar, compact mode, F-key strip, warning bar)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/dashboard_view.rs` (F1, master-detail, sub-tab bar, content-aware left panel sizing)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` (F2, 31% sidebar)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` (F3, 32% sidebar, role tabs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/git_view.rs` (F4, 35/65 split)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/logs_view.rs` (F5, full-width)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/context_view.rs` (F7, multi-layout sub-tabs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/marketplace_view.rs` (F8, 35/65 split)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/atelier_view.rs` (F9, 40/60 split)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/learning_view.rs` (F10, fixed+flex vertical)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/affect_view.rs` (affect gauges, fixed vertical)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/modals/mod.rs` (modal sizing, centered_rect, centered_rect_fixed)

---

## Implementation Status (2026-09-02 swarm)

Responsive layout fixes (task #25): terminal size handling, column collapse breakpoints,
minimum size behavior.
