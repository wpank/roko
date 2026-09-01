# Widget Visual Quality Audit

**Date:** 2026-09-01
**Scope:** All 17 widgets in `crates/roko-cli/src/tui/widgets/`
**Reference:** Bardo widget catalog (`prd/18-interfaces/screens/02-widget-catalog.md`, 33 widgets)

---

## Evaluation criteria

Each widget is evaluated on 10 axes:
1. Visual density (space efficiency)
2. Typography (bold/italic/dim/underline consistency)
3. Borders (style consistency, nesting)
4. Alignment (element positioning within containers)
5. Padding/margins (spacing)
6. Data formatting (numbers, tokens, costs, time)
7. Empty states (what shows with no data)
8. Overflow (text truncation behavior)
9. Responsiveness (terminal size adaptation)
10. Animation (which widgets animate, should more?)

---

## Executive summary

The roko widget set is **functionally solid** with good visual density and consistent use of
the ROSEDUST palette. However, compared to the bardo catalog's 33 widgets, several quality
gaps are visible:

| Area | Roko status | Bardo reference |
|------|-------------|-----------------|
| Animation depth | Heartbeat pulse + spinner | Phosphor persistence, erosion decay, value-lerping, PAD modulation |
| Empty states | Present but inconsistent format | N/A (bardo widgets degrade, not empty) |
| Data formatting | 3 different duration formatters, 2 different byte formatters | Unified formatting per widget type |
| Overflow | Mix of tail truncation and middle truncation | Consistent per widget type |
| Degradation | Not implemented | 5-phase degradation per behavioral phase |
| Value transitions | Instant snap | Lerped transitions (VitalityNumber: 2-3s lerp) |
| Phosphor persistence | Not implemented | Frame-based decay chains through 6 brightness stages |

**Top 5 priorities for visual improvement:**
1. Unify duration/cost/byte formatting across all widgets
2. Add phosphor-style value change animations (FlashNumber pattern)
3. Standardize empty state rendering
4. Add staleness decay (dim values that haven't changed)
5. Implement 2-3 degradation tiers for low-vitality/error states

---

## Per-widget audit

### 1. header_bar (`header_bar.rs`, 778 LOC)

**Purpose:** Top-line status bar with 9 sections: health dot, app name, queue label, wave
indicator, progress bar, plan count, ETA/elapsed/cost/tokens, system metrics, agent spinner,
F-key strip.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Excellent. Every pixel is used. 9 sections pack a remarkable amount of information into a single line. |
| Typography | A- | Consistent BOLD for app name, active tab, and status labels. ROSE for branding. The `sep()` helper provides uniform thin separators. |
| Borders | A | No borders (headerless single-line design). Clean. |
| Alignment | A | Right-aligned F-key strip via Layout split. Left content flows naturally. |
| Padding | B+ | Single-space padding is consistent (`" "` prefix). Some sections have double-space gaps that could be tighter. |
| Data formatting | B | Three different formats in one bar: `format_elapsed` (Xh/Xm/Xs), inline `{}M tok`/`{}K tok`, and cost with `$X.XX`/`$X.XXX`. The cost format switches at $1.00 threshold -- good. Token format is integer-only (no "1.2M") unlike `token_sparkline.rs` which uses "1.2M". |
| Empty states | A | Graceful: sections simply don't render when data is absent (no queue label, no wave, no tokens). |
| Overflow | B+ | `truncate_label` caps plan names at 24 chars with `..` suffix. Uses char-count truncation, not `truncate_middle`. |
| Responsiveness | A | `compact` mode at `width < 120` hides percentage, colon in CPU/MEM labels, MCP/NET/DSK/FPS. Well-implemented. |
| Animation | A | Health-aware pulsing dot (2-frame heartbeat with brightness modulation), active agent spinner with role label. |

**Bardo comparison:** Closest to a condensed version of the Hearth sidebar status. Bardo's
FlashNumber widget would add value-change flash effects to the token/cost counters. The
heartbeat dot is good but lacks the bardo DecisionRing's multi-phase awareness.

**Specific improvements:**
- **Token formatting inconsistency:** `header_bar` uses `{}M tok` (integer division) while
  `token_sparkline` uses `{:.1}M` (one decimal). Unify to `fmt_tokens()` from
  `token_sparkline.rs`.
- **Cost formatting inconsistency:** Cost switches format at $1.00 but the status bar uses
  `{:.2}` uniformly. Pick one strategy.
- **Missing:** Budget utilization percentage should use semantic coloring (green < 50%, amber
  50-80%, red > 80%) -- currently it uses `BONE_DIM` regardless of spend level.
- **Missing:** Staleness indicator for the elapsed timer -- if no progress has been made in
  5+ minutes, the elapsed timer should dim or pulse to indicate stall.

---

### 2. status_bar (`status_bar.rs`, 397 LOC)

**Purpose:** Bottom status bar with 5 sections: git info, heartbeat + pause, plan progress +
health, cost/budget, context-sensitive keybind hints.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Efficient single-line layout with meaningful data in every section. |
| Typography | A | BOLD for PAUSED badge (inverted WARNING bg), BOLD for error styles. |
| Borders | A | No borders. Clean footer design. |
| Alignment | B+ | Content flows left-to-right. No right-alignment for keybinds (they could benefit from right-alignment like the header's F-key strip). |
| Padding | B | Spacing around separators is `" | "` (pipe with spaces). Consistent but the pipe character differs from header's thin `|` separator -- header uses `\u{2502}` (box drawing), status bar also uses `\u{2502}` -- good consistency. |
| Data formatting | B+ | Cost uses `${:.2} / ${budget:.2} ({:.0}%)` consistently. Health counts use compact `2>` for active, `3ag` for agents. |
| Empty states | A | Git info section is conditionally rendered. No awkward blank spaces. |
| Overflow | B | Keybind hints are capped at 5 tokens via `truncate(5)` -- good. But hint strings themselves are never truncated and could overflow on very narrow terminals. |
| Responsiveness | B- | No explicit compact mode. All sections render regardless of width. At 60 cols, the keybinds could overflow or collide with cost data. |
| Animation | B+ | 4-frame heartbeat animation (`\u{00b7}`, `\u{00b0}`, `.`, `\u{25cf}`). |

**Bardo comparison:** The context-sensitive keybinds are a good pattern not explicitly in
bardo. The PAUSED badge is well-done with inverted colors.

**Specific improvements:**
- **Add compact mode:** Mirror the header's `width < 120` check. At narrow widths, truncate
  git info to just branch name, hide agent/failure counts, show fewer keybinds.
- **Right-align keybinds:** Push the keybind hints to the right edge (mirroring header's
  F-key strip) to create visual symmetry with the header.
- **Keybind overflow protection:** If available width is < sum of keybind widths, truncate
  the list dynamically rather than relying on the fixed cap of 5.
- **Cost duplication:** Both header and status bar show cost. Consider whether the status bar
  should show cost only when the header is scrolled off or collapsed.

---

### 3. plan_tree (`plan_tree.rs`, 1050 LOC)

**Purpose:** Collapsible wave-grouped plan tree with fixed-column layout, inline progress
bars, scrollbar, and data-rain fill.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Fixed-column layout (plan name, 6-char progress, 8-char bar, 8-char delta, 3-char verify, 6-char age) uses space exceptionally well. Column header row aids scanability. |
| Typography | A- | BOLD for active plans and wave headers. Status icons (`checkmark`, `cross`, `play`) are well-chosen. Phase abbreviations (`impl`, `vfy`, `mrge`) are compact and readable. |
| Borders | A | Single-border block with focused/unfocused styles. No nesting. Scrollbar uses buffer-direct `\u{2588}` for thumb. |
| Alignment | A | Fixed columns with right-aligned numbers (`{:>6}`) and left-aligned names. Column separator `\u{2502}` creates clean vertical lines. |
| Padding | A- | Wave header has horizontal line fill (`\u{2500}` repeat) that adapts to width -- excellent. Indentation uses 3-space indent for wave children. |
| Data formatting | A- | `format_duration()` is consistent (Xs/Xm/Xh/Xd). Progress uses `done/total` capped at 99. Verify cell uses `\u{2713}` for all-pass, `passed/total` otherwise. |
| Empty states | B+ | Empty plan list renders an empty bordered box with title. No explicit "no plans" message. Compared to bardo's UnitArray which shows grid cells going dark, this is functional but plain. |
| Overflow | A | Uses `truncate_middle()` from `util.rs` which preserves start and end of strings with ellipsis in the middle -- excellent for plan names like `e34-safety-contracts-strict`. |
| Responsiveness | B+ | Column header hidden at `width < 32`. Title adapts. But fixed column widths don't scale -- at very narrow widths, the plan name column gets squeezed. |
| Animation | B+ | Wave progress bars pulse with heartbeat when active. Gradient ocean color for wave bars. Selected plan shows detail row with mini progress bar. |

**Bardo comparison:** The fixed-column layout is clean -- bardo's UnitArray uses a grid of
uniform cells instead. The wave tree hierarchy with collapse/expand is a stronger navigation
pattern than bardo's flat grid. The scrollbar implementation (buffer-direct rendering) is
more lightweight than using ratatui's built-in Scrollbar widget.

**Specific improvements:**
- **Empty state:** Add a centered "no plans loaded" message with spinner when the plan list
  is empty, matching the task_progress empty state pattern.
- **Column width scaling:** At widths > 160, expand the progress bar column from 8 to 12+
  chars for more visual resolution. At widths < 60, hide the delta and verify columns.
- **Data rain comment:** The code says "data-rain visualization removed -- kept empty space
  clean" -- this was the right call. Bardo's DataRain widget occupies unused space with
  falling hex streams, but for an agent orchestrator that would be distracting noise.
- **Selected plan detail:** The detail row is useful but uses a different progress bar style
  (compact_progress_glyphs with `\u{2588}` + `\u{2500}`) than the column bar. Unify.
- **Missing:** Age column should use semantic coloring -- plans running > 10m should shift
  to WARNING, > 30m to EMBER, indicating potential stalls.

---

### 4. task_progress (`task_progress.rs`, 506 LOC)

**Purpose:** Scrollable task checklist with semantic progress bar, status badges, time tags,
and scroll indicators.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A- | Good use of space. Progress bar + summary badge + task list. The `RUN`/`DONE`/`FAIL`/`WAIT` status badge with inverted colors is an excellent pattern. |
| Typography | A | BOLD for active tasks and status badge. Status icons are well-chosen: `\u{2713}` done, `\u{25ba}` active (with pulse), `\u{2717}` blocked/failed, `\u{00b7}` pending. |
| Borders | A | Standard bordered block with focused/unfocused styles. ratatui Scrollbar widget for vertical scrolling. |
| Alignment | B+ | Task ID right after icon, then time tag, then title. No column alignment between tasks -- IDs and titles float. Compared to plan_tree's fixed columns, this is less structured. |
| Padding | B+ | " icon " prefix (3 chars). Time tag `\u{23F1}Xm ` inline. Scroll indicators ` \u{25b2} more` / ` \u{25bc} more` are clear. |
| Data formatting | B+ | `compact_duration()` format: "5m", "1h05m", "45s". Different from `format_elapsed()` in header_bar (which uses "Xh" not "XhXXm"). |
| Empty states | A | " spinner waiting for tasks..." with animated spinner. One of the better empty states. |
| Overflow | B+ | Task titles truncated with "..." suffix at available width minus prefix. Uses char-count, not `truncate_middle()`. Inconsistent with plan_tree. |
| Responsiveness | B | Progress bar hidden at `inner_width <= 8`. Header rows calculated dynamically. But no compact mode for very small areas. |
| Animation | A- | Active task icon pulses with `pulse_rose()` heartbeat. Progress bar leading edge pulses. Spinner in empty state. |

**Bardo comparison:** The status badge (`RUN`/`DONE`/`FAIL`/`WAIT`) with inverted background
is excellent -- equivalent to bardo's FlashNumber approach but for categorical state. The
pulse on active task icons maps to bardo's heartbeat concept. Missing: bardo's PhosphorLog
aging (completed tasks should dim over time).

**Specific improvements:**
- **Column alignment:** Align task IDs in a fixed-width column (8 chars) so titles start at
  the same position. Currently "t-001" and "t-100" cause different title offsets.
- **Truncation consistency:** Use `truncate_middle()` instead of tail truncation, matching
  plan_tree's approach. Task titles like "Wire SystemPromptBuilder into runner event loop"
  benefit from seeing both beginning and end.
- **Duration format unification:** Use the same duration formatter as header_bar. Currently
  `compact_duration(3661)` returns "1h01m" while header's `format_elapsed(3661)` returns
  "1h01m" -- actually consistent by accident. But the function names differ and the
  implementations are duplicated.
- **Missing:** No ETA per task. If we know average task duration, showing "~3m" estimated
  remaining per task would match the header's ETA pattern.
- **Missing:** Completed tasks should use DIM text style after a few seconds, creating a
  visual gradient of recency (bardo PhosphorLog pattern).

---

### 5. agent_status_grid (`agent_status_grid.rs`, 436 LOC)

**Purpose:** Dense per-agent status table with role, model, turns, context %, and effort
level. Has both full and compact variants.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Full grid packs 7 data points per agent into a single line. Compact variant shows icon + role + status. Good progressive disclosure. |
| Typography | A- | BOLD for active agents. Role names colored by `Theme::role_accent()`. Column header in muted style. |
| Borders | B+ | Standard bordered block. Title includes active/total count. But uses `theme.accent()` for border which differs from plan_tree's focused/unfocused pattern. |
| Alignment | A | Fixed-width columns: role 12, model 14, turns 5, ctx% 5, effort 8. Right-aligned numbers. Left-aligned text. Clean. |
| Padding | B | Single-space separators between columns. Leading " " before icon. Column spacing is tight. |
| Data formatting | B | Context % as `{:>4.0}%`. Turns as integer. Effort as text label ("high"/"medium"/"low"/"-"). Model truncated with `truncate_middle`. |
| Empty states | A- | "  no agents" with muted style. Simple but clear. |
| Overflow | B+ | "... +N more" indicator when agents exceed visible rows. Good. Model names use `truncate_middle`. |
| Responsiveness | B- | Column widths are hardcoded. No adaptation for narrow terminals. The compact variant exists but callers must choose -- no automatic fallback. |
| Animation | B- | No animation. Active agents are BOLD but don't pulse. Status icons are static circles/checks. |

**Bardo comparison:** Closest to bardo's UnitArray but as a table rather than a grid.
Bardo's UnitArray cells have shimmer, alert dimming, breach cascade, and staleness decay --
none of which are present here. The effort level (proxy from context usage) maps loosely to
bardo's PAD modulation concept.

**Specific improvements:**
- **Unify border pattern:** Use `Theme::focused_border_style()` /
  `Theme::unfocused_border_style()` instead of custom `theme.accent()` / `theme.muted()`.
  This widget uses the `&Theme` instance pattern while most other widgets use `Theme::` static
  methods. See consistency issue below.
- **Add animation:** Active agents should pulse their status icon (like task_progress does)
  rather than using a static bold circle. The heartbeat dot from header_bar could be reused.
- **Context % gauge:** Replace the plain `{:>4.0}%` text with a mini inline gauge (like
  sys_metrics.rs uses for CPU/MEM). Context exhaustion is a critical operational metric that
  deserves visual weight.
- **Staleness:** Agents that haven't reported in N ticks should dim, matching bardo's
  UnitArray staleness pattern.
- **Automatic fallback:** When area height is small enough, automatically switch from full to
  compact mode rather than requiring the caller to decide.

---

### 6. gate_output (`gate_output.rs`, 177 LOC)

**Purpose:** Live gate rung output viewer with color-coded lines (compile/test/clippy output).

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | B+ | Essentially a scrolling log viewer. Line classification adds semantic value but the content is raw output. |
| Typography | A- | BOLD for error lines. Dim for download/blocking lines. Color-coding maps well to cargo output semantics: SAGE for success, EMBER for errors, WARNING for warnings, DREAM for test runs. |
| Borders | B+ | Border color changes when gate is running (WARNING) vs idle (TEXT_GHOST). Good semantic signaling. Title includes running rung name + elapsed + spinner. |
| Alignment | N/A | Raw text output, no column structure. |
| Padding | B | No padding on content lines -- they render as-is from gate output. Title has standard block padding. |
| Data formatting | B | Elapsed time in title as raw seconds `{elapsed}s`. Not using any duration formatter -- inconsistent with other widgets. |
| Empty states | A- | Two states: idle ("no gate output") and waiting (" spinner waiting for output..."). Good. |
| Overflow | B | Uses `Wrap { trim: false }` for long lines. This means long compiler error lines wrap at the widget boundary, which can break readability of error messages. |
| Responsiveness | B | Returns early if `inner.width < 4 || inner.height < 1`. No other adaptation. |
| Animation | B+ | Spinner in title when running. Auto-scroll to bottom (tail follow). Manual scroll override. |

**Bardo comparison:** Closest to bardo's PhosphorLog but without the phosphor persistence
effect. Bardo's log entries fade through 6 brightness stages over time. Gate output lines
remain at full brightness forever. The line classification is a good roko-specific addition
(bardo uses tier-based coloring, roko uses pattern matching on cargo output).

**Specific improvements:**
- **Elapsed format:** Use `format_elapsed()` or `compact_duration()` instead of raw
  `{elapsed}s` in the title.
- **Line number gutter:** For error diagnostics, showing a line number gutter (like
  `{line_num:>4} | content`) would help correlate with compiler output.
- **Truncation instead of wrap:** For gate output, truncation with horizontal scroll might be
  more useful than wrapping, since compiler errors often have deeply indented context.
- **Phosphor aging:** Old lines should dim. The most recent 5-10 lines should be at full
  brightness, older lines should fade to TEXT_DIM, then TEXT_GHOST. This creates bardo's
  "visual gradient of time" effect.
- **Error highlighting:** Lines matching `error[E` should have a left-margin indicator
  (a colored `|` or `>`) to make them visually distinct even when scrolled away from.

---

### 7. token_sparkline (`token_sparkline.rs`, 314 LOC)

**Purpose:** Efficiency summary with token sparkline chart and model tier distribution bars.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Packs summary stats (tokens, cost, avg/task), a braille sparkline, and tier distribution bars into a compact widget. |
| Typography | B+ | Labels in BONE_DIM, values in BONE/WARNING/FG. No BOLD usage. Tier labels are color-coded (T0=SAGE, T1=ROSE, T2=WARNING). |
| Borders | B+ | Standard bordered block. Border color changes when cost > 0 (ROSE_DIM vs TEXT_GHOST). |
| Alignment | B+ | Summary lines use inline spans with label-value pairs. Tier bars use fixed-width label + proportional bar + count suffix. |
| Padding | B | " " prefix on summary lines. Tier labels have fixed format `" T0 haiku  "`. |
| Data formatting | A | `fmt_tokens()` is the best token formatter: handles 0, <1k, <10k (1 decimal), <1M (integer k), >1M (1 decimal M). `fmt_rate()` handles idle/per-min. Cost as `${:.2}`. |
| Empty states | B+ | " spinner waiting for data..." when series has < 2 points. Good. |
| Overflow | B | Sparkline window adapts to width (10/50/100 samples at 80/120/120+ cols). No text truncation needed. |
| Responsiveness | A- | Returns early at `inner_width < 10 || inner_height < 2`. Sparkline window scales with width. Second summary row hidden at `inner.height <= 3`. Tier bars fill remaining rows. |
| Animation | B+ | Pulsed color on sparkline via `breathing_brightness()`. Border color semantic. |

**Bardo comparison:** Closest to bardo's Sparkline widget (braille resolution) combined with
elements of ConfidenceBar (the tier distribution bars). Bardo's sparkline has rising/falling
segment coloring and min/max markers -- roko's uses a single pulsing color. The tier bars map
to bardo's UnitArray concept.

**Specific improvements:**
- **Rising/falling colors:** Color segments of the sparkline differently for rising (SAGE)
  vs falling (EMBER) trends, matching bardo's Sparkline spec. Currently the entire sparkline
  uses a single pulsing ROSE color.
- **Min/max markers:** Mark the peak and trough in the visible sparkline window with BONE
  and EMBER respectively, matching bardo's Sparkline spec.
- **Tier bar labels:** "haiku"/"sonnet"/"opus" are hardcoded labels that will break for
  non-Anthropic models. The tier detection (`model_tier()`) only checks for "haiku"/"opus"
  and defaults to "T1". This should be generalized.
- **Cost semantic coloring:** The cost value uses WARNING color regardless of amount. It
  should use semantic thresholds (like cost_by_model does: green < $1, amber $1-5, red > $5).

---

### 8. sys_metrics (`sys_metrics.rs`, 231 LOC)

**Purpose:** System gauges for CPU, MEM, NET, DSK, FPS with braille sparklines and mini
inline gauges.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Exceptional. Each metric gets: 4-char label, 6-char value, 10-char mini gauge, sparkline. Five metrics in 5 rows. |
| Typography | B+ | Labels in TEXT_DIM. Values in semantic color. No BOLD. Consistent pattern across all 5 metrics. |
| Borders | B+ | Standard bordered block. Static TEXT_GHOST border (doesn't change on alarm). |
| Alignment | A | Fixed-width labels (4 chars). Right-aligned values. Gauge and sparkline fill remaining space. |
| Padding | B+ | Clean spacing: "LABEL VALUE GAUGE SPARKLINE". Single space separators. |
| Data formatting | A- | `fmt_bytes()`: "X.XG" / "XM" / "XK". `fmt_rate()`: "X.XG"/"X.XM"/"X.XK"/"XB"/"0B". CPU as `{:>5.1}%`. All well-formatted. |
| Empty states | B | Returns early if `inner.width < 12 || inner.height < 2`. No message shown. The widget just disappears. |
| Overflow | B+ | Gauge width adapts: `10.min(w - 18)`. Sparkline width fills remainder. NET/DSK lines are simpler (no gauge/sparkline). |
| Responsiveness | A | Progressive disclosure: CPU always shown, MEM at height >= 2, NET at >= 3, DSK at >= 4, FPS at >= 5. Each metric conditionally rendered. |
| Animation | A | Per-cell shimmer on gauge fills using breathing_brightness() + sinusoidal phase offset. Sparklines animate with new data. This is the most animated non-header widget. |

**Bardo comparison:** The mini gauge with per-cell shimmer is an excellent implementation of
bardo's MortalityGauge pattern scaled down. The breathing shimmer creates subtle organic
motion. NET's up/down arrows (`\u{2193}`/`\u{2191}`) are clean.

**Specific improvements:**
- **Border alarm:** When CPU > 80% or disk < 1GB, the border should change color (EMBER or
  WARNING) to signal alarm, matching the gate_output pattern.
- **Empty state:** Show "collecting..." message instead of rendering nothing when metrics
  haven't been sampled yet.
- **NET sparkline:** CPU and MEM get sparklines but NET and DSK don't. If vertical space
  permits, add sparklines for network throughput.
- **MEM format inconsistency:** `sys_metrics` uses `{:.1}G` / `{:.0}M` while `header_bar`
  uses `{:.0}G` / `{:.0}M` (via `fmt_bytes_short`). The header rounds more aggressively.
  Unify or document the intent.

---

### 9. cost_by_model (`cost_by_model.rs`, 428 LOC)

**Purpose:** Per-model cost statistics table with Model, Provider, Tasks, Pass%, Avg Duration,
Total Cost, $/Task columns.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A- | Standard table layout. 7 columns is dense. TOTAL row at bottom is a good pattern. |
| Typography | A | BOLD for TOTAL row and column headers. Pass% uses semantic coloring (success/warning/danger). Cost uses semantic thresholds. |
| Borders | B | Uses `theme.accent()` for border regardless of state. Should use focused/unfocused pattern. |
| Alignment | A | Right-aligned numbers (`{:>5}`, `{:>5.1}%`, `{:>7.4}`). Left-aligned model/provider. Fixed column widths via Constraints. |
| Padding | B+ | Column spacing via ratatui Table's `column_spacing(1)`. Single space. |
| Data formatting | A | `format_duration()`: "Xms"/"Xs"/"Xm" with right-alignment. `format_cost()`: dash for < $0.001, 4 decimals < $1, 3 decimals < $10, 2 decimals >= $10. Well-tiered. |
| Empty states | A- | "  no efficiency data" in muted style. Clear. |
| Overflow | B+ | Model names truncated with `truncate_model()` which uses middle truncation ("abc..xyz"). Max 24 chars. Provider max 12. |
| Responsiveness | B | Returns early at `inner.width < 20 || inner.height < 3`. Column widths use Min/Length constraints that flex. But the 7-column layout doesn't degrade at narrow widths. |
| Animation | C | No animation at all. Static table. |

**Bardo comparison:** No direct bardo equivalent (bardo doesn't have model cost tracking).
The table layout is professional. The TOTAL row with bold styling is a good accounting
pattern.

**Specific improvements:**
- **Row selection highlighting:** The `row_highlight_style` is set but there's no selection
  state (`focused = false` is hardcoded). Wire up selection support.
- **Sparkline per model:** Add an inline sparkline column showing cost trend per model over
  time, using the braille renderer. This would show which models are getting more or less
  expensive.
- **Sort indicator:** Show which column is sorted by (currently implicit: alphabetical by
  model name). Allow column-based sorting with an indicator arrow.
- **Narrow mode:** At width < 80, hide Provider and Avg Dur columns and expand Model width.

---

### 10. error_digest (`error_digest.rs`, 634 LOC)

**Purpose:** Error aggregation with two modes: compact inline (gate summary + error list) and
full scrollable panel with categorized errors.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Compact mode: gate pass/fail ratio + recent failures in 3 rows. Full mode: category summary bar + timestamped scrollable list. |
| Typography | A | BOLD for error count and ITALIC for empty state. Category icons (cross, hammer, warning, flag, lightning) are distinctive. |
| Borders | A- | Border turns red (danger) when errors are active, muted when clean. Good semantic signaling. |
| Alignment | B+ | Timestamps left-aligned `[HH:MM:SS]`, then category icon, then source, then message. No fixed columns for the error list though. |
| Padding | B | "  " prefix for category summary. " " prefix for error lines. |
| Data formatting | A- | `fmt_ts()`: `HH:MM:SS` from millisecond timestamps. Simple and correct. Category counts shown as "category: N". |
| Empty states | A | Full panel: centered "no errors recorded" with italic styling and explanatory subtext ("errors from gates, agents, and runtime will appear here when they occur"). One of the best empty states. |
| Overflow | B+ | Error messages truncated with `..` at max available width. Compact mode shows last 3 failures. Full mode is scrollable. |
| Responsiveness | B | Returns early at `inner.height < 3`. Layout split is fixed (Length(3) + Min(0)). |
| Animation | C+ | No animation. The red border on active errors is the only visual signal. |

**Bardo comparison:** The error categories with icons map loosely to bardo's tier
differentiation in PhosphorLog (T0-T3 brightness levels). The empty state is superior to
most bardo widgets which don't have explicit empty states (they use degradation instead).

**Specific improvements:**
- **Error flash:** When a new error appears, flash the border or the newest error line
  briefly (bardo FlashNumber pattern). Currently errors appear silently.
- **Category filter:** Allow filtering by category (gate/compile/agent/preflight/runtime)
  to reduce noise during long runs with many errors.
- **Deduplication count:** When errors are deduplicated, show the count: "gate failed for
  t-001 (x3)" instead of just keeping the latest timestamp.
- **Time-relative display:** Consider "2m ago" format instead of absolute HH:MM:SS, which
  requires mental math to determine recency.

---

### 11. diff_panel (`diff_panel.rs`, 88 LOC)

**Purpose:** Unified diff viewer with +/- syntax coloring.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | B | Simple diff viewer. One line per diff line. No side-by-side mode. |
| Typography | A | BOLD for diff headers and hunk markers. Cyan for `@@` markers. Green for additions, red for deletions. Standard diff coloring. |
| Borders | B | Static muted border. Title is lowercase "diff". |
| Alignment | N/A | Raw diff output, no column structure. |
| Padding | N/A | No padding on diff lines. |
| Data formatting | N/A | Raw diff text. |
| Empty states | B+ | Centered "no diff" in muted style. Simple. |
| Overflow | B- | Uses `Wrap { trim: false }` which breaks long diff lines across multiple visual rows. This can make diffs hard to read, especially with indented code. |
| Responsiveness | B | Returns early if `inner.width == 0 || inner.height == 0`. No other adaptation. |
| Animation | C | No animation. Static content. Scroll position can be pinned or auto-scrolled to end. |

**Bardo comparison:** No direct bardo equivalent. This is utilitarian.

**Specific improvements:**
- **Line numbers:** Add line numbers in a gutter: `{old_line:>4} {new_line:>4} | content`.
  This is standard for diff viewers.
- **Horizontal scroll:** Replace line wrapping with horizontal scroll for long lines.
  Wrapped diffs of code are nearly unreadable.
- **File header highlighting:** Give `diff --git` and `index` lines a background color
  (BG_SECONDARY) to visually separate files in multi-file diffs.
- **Stats summary:** Show `+N/-N` line count summary in the title or footer.
- **Title:** Capitalize "Diff" for consistency with other widget titles.

---

### 12. wave_progress (`wave_progress.rs`, 119 LOC)

**Purpose:** Proportional wave progress ribbon with animated ocean gradient fill.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Single-row ribbon. Each wave gets proportional width with "WN" label + bar. |
| Typography | B+ | Wave labels ("W0", "W1") in BONE for current, FG_DIM otherwise. |
| Borders | N/A | No borders. Inline ribbon. |
| Alignment | B+ | Proportional width per wave based on plan count. Minimum 3 chars per wave. |
| Padding | B | " " separators between label and bar within each wave. No inter-wave separator -- adjacent waves blend visually. |
| Data formatting | B | Wave label is "W{index}". Simple. |
| Empty states | A | Returns early (renders nothing) when no waves exist. Clean. |
| Overflow | B | Minimum 3 chars per wave. At very narrow widths with many waves, wave labels would be dropped and only bars shown. |
| Responsiveness | B+ | Returns early at `width < 10`. Proportional sizing adapts to any width. |
| Animation | A | Per-cell ocean gradient with animated time offset (`elapsed * 0.1`). The gradient shifts over time, creating a flowing water effect on the current wave. Completed waves are static SAGE. |

**Bardo comparison:** The animated ocean gradient is excellent -- it's the closest roko gets
to bardo's "visual motion" philosophy. The flowing color on the active wave segment conveys
progress as organic movement.

**Specific improvements:**
- **Inter-wave separators:** Add a thin `\u{2502}` separator between wave segments to
  prevent adjacent bars from blending together.
- **Wave completion flash:** When a wave completes, flash its segment BONE for 1-2 seconds
  before settling to SAGE. Currently it snaps to green.
- **Percentage label:** For the current wave, overlay "42%" on the bar (if width allows)
  rather than requiring the viewer to estimate from the fill position.

---

### 13. parallel_pool (`parallel_pool.rs`, 148 LOC)

**Purpose:** Table of parallel agent instances with sorting and selection.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | B+ | Standard 6-column table: agent id, role, model, task, progress, cumulative usage. |
| Typography | A- | BOLD for column header. Status labels are center-aligned and bold (`{:^8}`). Usage shows "in Xk out Xk" pattern. |
| Borders | B | Uses unfocused border style. Title is lowercase "parallel agents". |
| Alignment | A | Fixed-width columns via Constraints. Status label centered. Numbers right-aligned. |
| Padding | B | column_spacing(1). Tight. |
| Data formatting | B | Token usage as "in Xk out Xk" with integer division by 1000. Loses precision for sub-1k values. |
| Empty states | B+ | Centered "no parallel agents" in muted style. |
| Overflow | B | Uses `truncate()` (tail truncation, not middle) for agent id (12), role (10), model (12), task (18). IDs benefit from middle truncation. |
| Responsiveness | B- | Fixed column widths. No adaptation. |
| Animation | C | No animation. Static table with selection highlight. |

**Bardo comparison:** Similar to agent_status_grid but as a table. Lacks any of bardo's
UnitArray motion.

**Specific improvements:**
- **Truncation:** Use `truncate_middle` for agent IDs and model names (matching
  agent_status_grid and plan_tree patterns).
- **Token format:** Use the `fmt_tokens()` helper from token_sparkline instead of raw
  `{}k` division. Sub-1k values should show as-is, not "0k".
- **Title case:** Capitalize "Parallel Agents" for consistency.
- **Active indicator:** Add a pulsing dot or spinner for agents with Active status.

---

### 14. dream_view (`dream_view.rs`, 433 LOC)

**Purpose:** Dream state visualization: phase header with progress, replay candidates with
utility bars, hypotheses with confidence levels.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Three-section layout: phase header (3 rows), replay candidates (40%), hypotheses (40%). Utility bars and confidence labels are compact. |
| Typography | A | BOLD for phase name and indicator dot. DIM for utility scores. Phase colors are well-differentiated: DREAM_BRIGHT (NREM), DREAM_REM (REM), SAGE (Integration), EMBER (Threat). |
| Borders | A- | Outer block with focused/unfocused styles. Sub-panels use `Borders::TOP` only, avoiding nested boxes. |
| Alignment | B+ | Utility bars are fixed 10-char width with score suffix. Hypothesis confidence is `[XX%]` prefix. |
| Padding | B | No leading space on lines. Phase header could use 1-char indent for visual breathing room. |
| Data formatting | A- | Utility as `{:.2}`. Confidence as `{:.0}%`. Archive as "occupied/total cells". Best quality as `{:.3}`. Cycles as integer. Waking improvement as `+X.X%`. |
| Empty states | A- | "No replay candidates" and "No active hypotheses" in TEXT_GHOST. Phase defaults to "Idle". Archive shows "Archive: --" when unavailable. |
| Overflow | B+ | Uses `truncate()` for episode IDs (20 chars) and hypothesis summaries (width - 8). |
| Responsiveness | B+ | Returns early at `inner.height < 3 || inner.width < 20`. Layout uses percentage splits that adapt to height. |
| Animation | B- | No animation beyond the outer block's focused styling. The dream phase indicator dot is static despite being BOLD. |

**Bardo comparison:** Maps directly to bardo's CounterfactualBranch (dream branching tree)
and the Dream screen's BrailleDensityMap. Bardo's dream widgets have phosphor effects and
dreamlike visual corruption in Terminal phase. Roko's dream_view is clean and functional but
lacks the atmospheric quality bardo specifies.

**Specific improvements:**
- **Phase animation:** The phase indicator dot should pulse when a dream is active (NREM or
  REM phase), matching the heartbeat dot pattern from header_bar.
- **Utility bar gradient:** Use a gradient from TEXT_PHANTOM to DREAM_BRIGHT on the utility
  bars instead of a single color, creating visual depth.
- **Archive heatmap:** If space permits, render the MAP-Elites archive as a mini braille
  density map (using the braille module) rather than just "42/100 cells".
- **Dreamlike visual effects:** During REM phase, the border could use DREAM_REM color and
  the text could have subtle breathing effects, creating atmosphere.

---

### 15. phase_compact (`phase_compact.rs`, 359 LOC)

**Purpose:** Compact 2-line phase widget with segmented phase bar and active phase detail.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | Two lines: segmented bar (each phase gets proportional width) and detail text. Very efficient. |
| Typography | A | BOLD for spinner in active phase and HALTED label. Phase bar uses solid blocks for done/active, dashes for pending. |
| Borders | B+ | Standard bordered block with focused/unfocused styles. Title includes active phase name. |
| Alignment | A | Segmented bar fills full width with proportional segments. Detail line is left-aligned with icon prefix. |
| Padding | N/A | No padding needed -- the bar fills the full inner width. |
| Data formatting | B+ | Detail line shows phase name, percentage as `{:.0}%` (capped at 99), elapsed as `Xm XXs`. The elapsed format differs from other widgets -- uses "Xm XX s" with zero-padded seconds. |
| Empty states | A- | "waiting..." for no active phases. "all phases complete" in SAGE when done. "HALTED at {name}" in EMBER for failures. All three states handled. |
| Overflow | B+ | Returns early if `inner.height < 1 || inner.width < 10`. Segment widths degrade gracefully with leftover distribution. |
| Responsiveness | B+ | Works at any width >= 10. Segments proportionally scale. Second line conditionally rendered at height >= 2. |
| Animation | A | Active phase segment ends with spinner character (`spinner_ethereal()`). Detail line icon pulses with `pulse_active()`. The active segment is visually alive. |

**Bardo comparison:** Maps to bardo's ActionGateIndicator (action gate status bar). Bardo's
version would have phosphor persistence on phase transitions -- when a phase completes, its
segment would briefly flash BONE then settle to SAGE. Roko's transitions are instant.

**Specific improvements:**
- **Phase transition flash:** When a phase completes, flash the segment BONE_BRIGHT for 0.5s
  before settling to SAGE. This creates a satisfying "completion pulse" effect.
- **ETA in detail:** Add estimated time remaining based on elapsed time and percentage.
- **Elapsed format:** Use `compact_duration()` from task_progress for consistency instead of
  the custom `Xm{:02}s` format.
- **Failed phase pulsing:** The HALTED state uses static EMBER. It should pulse or flash to
  draw attention.

---

### 16. braille (`braille.rs`, 79 LOC)

**Purpose:** Braille sparkline rendering primitives. Not a visible widget itself -- a renderer
used by sys_metrics and token_sparkline.

| Axis | Grade | Notes |
|------|-------|-------|
| Visual density | A | 2x horizontal density via braille character encoding. Two data points per terminal cell. |
| Typography | N/A | Single color per sparkline. No text. |
| Borders | N/A | Inline component, no borders. |
| Alignment | N/A | |
| Padding | N/A | Empty data shows spaces in DarkGray. |
| Data formatting | A | Handles f64, f32, and u64 inputs. Auto-scaling for u64 (delta-normalized). Fixed max for f32/f64. |
| Empty states | B+ | Returns space-filled spans with DarkGray when data is empty. Correct but invisible. |
| Overflow | A | Automatically windows to the most recent `width * 2` data points. |
| Responsiveness | A | Width is a parameter -- callers size it. |
| Animation | B | Color is static per sparkline. No per-cell color variation. |

**Bardo comparison:** Matches bardo's Sparkline spec for resolution. Bardo's version adds
rising/falling segment coloring and min/max markers which are absent here.

**Specific improvements:**
- **Per-segment coloring:** Allow passing a color callback `fn(value) -> Color` so rising
  values can be SAGE and falling values can be EMBER, matching bardo's Sparkline spec.
- **Min/max markers:** Mark the peak cell with BONE color and the trough with EMBER.
- **Gradient option:** Support gradient-colored sparklines where color maps from SAGE (low
  values) through WARNING to EMBER (high values), useful for CPU/memory gauges.

---

### 17. rosedust (`rosedust.rs`, 10 LOC)

**Purpose:** Compatibility shim re-exporting the canonical theme's gradient functions.

| Axis | Grade | Notes |
|------|-------|-------|
| All | N/A | This is a 10-line re-export module. Not a visible widget. Provides `brighten()`, `gradient_fire()`, `gradient_ocean()` from the canonical theme. |

**No visual audit needed.** This is infrastructure.

---

## Cross-cutting issues

### Issue 1: Theme API inconsistency

Some widgets accept `&Theme` as a parameter and use instance methods (`theme.muted()`,
`theme.accent()`), while others use static `Theme::` constants and methods
(`Theme::SAGE`, `Theme::focused_border_style()`). Both patterns work but create visual
inconsistency:

**Instance pattern** (agent_status_grid, cost_by_model, error_digest, diff_panel,
parallel_pool, gate_output): `theme.muted()`, `theme.accent`, `theme.danger`.

**Static pattern** (header_bar, status_bar, plan_tree, task_progress, phase_compact,
dream_view, wave_progress, sys_metrics, token_sparkline): `Theme::SAGE`, `Theme::EMBER`,
`Theme::focused_border_style()`.

The static pattern is more common (9 widgets vs 6) and avoids threading a `&Theme` parameter.
Widgets using the instance pattern were likely ported earlier and should migrate.

### Issue 2: Duration formatting

Four separate duration formatters exist:

| Widget | Function | Example output | Location |
|--------|----------|---------------|----------|
| header_bar | `format_elapsed()` | "1h01m", "5m03s", "45s" | `header_bar.rs:114` |
| plan_tree | `format_duration()` | "1d", "2h", "5m", "45s" | `plan_tree.rs:853` |
| task_progress | `compact_duration()` | "1h01m", "5m", "45s" | `task_progress.rs:351` |
| cost_by_model | `format_duration()` | "500ms", "5.0s", "2.0m" | `cost_by_model.rs:323` |
| phase_compact | inline | "2m31s" (custom format) | `phase_compact.rs:208` |
| gate_output | inline | "45s" (raw seconds) | `gate_output.rs:83` |

These should be unified into 2-3 canonical formatters in a shared `display_utils` module:
1. **compact** (for inline use): "45s", "5m", "2h", "1d"
2. **detailed** (for headers/status): "1h01m", "5m03s"
3. **milliseconds** (for latency): "500ms", "5.0s"

### Issue 3: Byte formatting

Two byte formatters:

| Widget | Function | Example output |
|--------|----------|---------------|
| header_bar | `fmt_bytes_short()` | "12G", "384M", "256K" (no decimal on G) |
| sys_metrics | `fmt_bytes()` | "12.1G", "384M", "256K" (1 decimal on G) |

Should be unified. The 1-decimal variant is more informative.

### Issue 4: Token formatting

Two token formatters:

| Widget | Function | Example output |
|--------|----------|---------------|
| header_bar | inline | "1M tok", "25K tok" (integer only) |
| token_sparkline | `fmt_tokens()` | "1.2M", "4.5k", "500" (1 decimal) |
| parallel_pool | inline | "25k" (integer division by 1000) |

Should use `fmt_tokens()` everywhere.

### Issue 5: Truncation strategy

Two truncation approaches:

| Strategy | Used by |
|----------|---------|
| Middle truncation (`truncate_middle()`) | plan_tree, agent_status_grid, dream_view |
| Tail truncation ("text...") | task_progress, parallel_pool, header_bar |

Middle truncation is superior for identifiers (plan names, agent IDs, model names) because
both the prefix and suffix carry meaning. Tail truncation is acceptable for free-text titles.
Establish a convention: identifiers use middle truncation, descriptions use tail truncation.

### Issue 6: Empty state inconsistency

| Widget | Empty state message | Style |
|--------|-------------------|-------|
| task_progress | " spinner waiting for tasks..." | TEXT_DIM, animated |
| agent_status_grid | "  no agents" | muted |
| agent_status_grid (compact) | "  no agents" | muted |
| cost_by_model | "  no efficiency data" | muted |
| parallel_pool | "no parallel agents" | muted, centered |
| error_digest (full) | "no errors recorded" + explanation | muted, italic, centered |
| error_digest (compact) | "No errors" | muted |
| diff_panel | "no diff" | muted, centered |
| gate_output (idle) | " no gate output" | muted |
| gate_output (running) | " spinner waiting for output..." | animated |
| token_sparkline | " spinner waiting for data..." | TEXT_DIM, animated |
| plan_tree | (empty bordered box) | nothing |

**Recommendation:** Establish two empty state patterns:
1. **Active waiting:** " spinner waiting for {noun}..." (animated, TEXT_DIM) -- for states
   that will populate during a running session
2. **Passive empty:** "no {noun}" centered in muted style -- for states that may never
   populate

### Issue 7: Missing animations

Bardo specifies that **every widget moves**. Roko's animation coverage:

| Animation type | Widgets that have it | Widgets that should |
|---------------|---------------------|---------------------|
| Heartbeat pulse | header_bar (dot), task_progress (active icon), phase_compact (spinner), sys_metrics (gauge shimmer), wave_progress (gradient flow) | agent_status_grid (active agents), error_digest (new errors) |
| Breathing brightness | token_sparkline, sys_metrics | dream_view (active dream), gate_output (running state) |
| Spinner | header_bar (agent), task_progress (empty state), gate_output (title), token_sparkline (empty state), phase_compact (active phase) | parallel_pool (active agents), cost_by_model (during aggregation) |
| Value flash | (none) | header_bar (cost/token changes), plan_tree (progress changes), task_progress (status transitions) |
| Phosphor decay | (none) | gate_output (old lines), task_progress (completed tasks), error_digest (old errors) |

---

## Summary: priority improvements

### P0: Formatting unification (low risk, high consistency)

1. Create `tui/format.rs` with canonical `fmt_duration_compact()`, `fmt_duration_detail()`,
   `fmt_duration_ms()`, `fmt_bytes()`, `fmt_tokens()`, `fmt_cost()`.
2. Replace all inline formatters with calls to the canonical versions.
3. Estimated LOC: ~50 new, ~80 changed.

### P1: Empty state standardization (low risk, visual polish)

1. Define two patterns (active waiting, passive empty) in a shared helper.
2. Update plan_tree (add "no plans" message), parallel_pool (add spinner variant).
3. Estimated LOC: ~30 new, ~40 changed.

### P2: Truncation consistency (low risk, visual polish)

1. Convention: identifiers use `truncate_middle()`, descriptions use tail truncation.
2. Update task_progress, parallel_pool, header_bar to use appropriate strategy.
3. Estimated LOC: ~20 changed.

### P3: Theme API migration (moderate risk, consistency)

1. Migrate 6 instance-pattern widgets to static `Theme::` pattern.
2. Remove `&Theme` parameter from `render_*` functions.
3. Estimated LOC: ~100 changed across 6 files.

### P4: Value change animation (moderate risk, high visual impact)

1. Implement a `FlashState` for numeric values (bardo's FlashNumber pattern).
2. Add it to header_bar cost/token counters and plan_tree progress counts.
3. Estimated LOC: ~80 new, ~40 changed.

### P5: Phosphor aging (moderate risk, high visual impact)

1. Implement frame-based brightness decay for log-style widgets.
2. Apply to gate_output (old lines dim), error_digest (old errors dim).
3. Estimated LOC: ~60 new, ~30 changed.

### P6: Status bar responsiveness (low risk, resilience)

1. Add compact mode to status_bar mirroring header_bar's approach.
2. Right-align keybind hints.
3. Estimated LOC: ~30 changed.

---

## Widget-to-bardo mapping

| Roko widget | Closest bardo widget(s) | Gap |
|-------------|------------------------|-----|
| header_bar | (no direct equivalent; condensed Hearth sidebar) | Flash effects, lerped transitions |
| status_bar | (no direct equivalent) | Responsive width handling |
| plan_tree | UnitArray + custom tree | Staleness decay, shimmer, breach cascade |
| task_progress | PhosphorLog (conceptually) | Phosphor persistence, tier coloring |
| agent_status_grid | UnitArray | Shimmer, staleness, alert dimming |
| gate_output | PhosphorLog | Phosphor aging, resolution loss |
| token_sparkline | Sparkline + ConfidenceBar | Rising/falling colors, min/max markers |
| sys_metrics | MortalityGauge (scaled down) | Erosion effect on decreasing values |
| cost_by_model | (no equivalent) | Row animation |
| error_digest | (no equivalent; partial PhosphorLog) | Flash on new errors, category filtering |
| diff_panel | (no equivalent) | Line numbers, horizontal scroll |
| wave_progress | (no equivalent) | Inter-wave separators, completion flash |
| parallel_pool | UnitArray (table variant) | Active agent animation |
| dream_view | CounterfactualBranch + BrailleDensityMap | Dreamlike visual effects, archive heatmap |
| phase_compact | ActionGateIndicator | Phase transition flash, ETA |
| braille | Sparkline (renderer) | Per-segment coloring, min/max |
| rosedust | (infrastructure) | N/A |

**Bardo widgets with no roko equivalent (15 of 33):**
FlashNumber, MortalityGauge, WaveformTrace, ConfidenceBar, DecisionRing, MAGIPanel,
VitalityNumber, ATFieldWireframe, DataRain, PhilosophicalWhisper, ConvergenceLines,
LatticePattern, PersistenceDiagramWidget, SomaticMarkerPanel, CausalGraphMinimap.

Most of these are domain-specific to bardo's DeFi/mortality concepts and are not needed in
roko. The ones that would transfer well:
- **FlashNumber:** Generic value-change flash wrapper. Universally useful.
- **WaveformTrace:** Block-character time series. Useful for PAD/affect waveforms.
- **ConfidenceBar:** Decay-aware confidence display. Useful for knowledge tier visualization.
- **VitalityNumber:** Lerped single-number display. Useful for cost/token aggregates.
