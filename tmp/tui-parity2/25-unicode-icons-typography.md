# 25 - Unicode Symbols, Icons, and Typography Audit

Audit date: 2026-09-01

Scope: all files under `crates/roko-cli/src/tui/` plus CLI output files
(`output_format.rs`, `doctor.rs`, `prd.rs`, `commands/*.rs`, `runner/output_sink.rs`,
`agent_serve.rs`, `chat.rs`).

---

## 1. Complete Unicode Symbol Inventory

### 1.1 Status Indicators

| Codepoint | Glyph | Meaning | Files using it |
|-----------|-------|---------|----------------|
| `\u{2713}` | checkmark | Done / passed / success | plan_tree, agent_status_grid, agents_view, plans_view, task_progress, marketplace_view, config_cmd, server, prd, output_sink, queue_manifest, job |
| `\u{2717}` | X mark | Failed / blocked / error | plan_tree, agent_status_grid, agents_view, plans_view, task_progress, marketplace_view, status_bar, server, output_sink, job |
| `\u{2718}` | heavy X mark | Gate failure (error_digest only) | error_digest |
| `\u{25b6}` | play triangle | Active / running | plan_tree, agents_view, plans_view, logs_view, learning_view, marketplace_view, job |
| `\u{25cb}` | open circle | Pending / idle / open | plan_tree, agent_status_grid, plans_view, marketplace_view, agent_serve, dashboard (circuit), job |
| `\u{25cf}` | filled circle | Active / heartbeat | agent_status_grid, header_bar, status_bar, dashboard, agent_serve, dream_view |
| `\u{00b7}` | middle dot | Inactive / pending / separator | plan_tree, plans_view, agents_view, task_progress, status_bar, gate_output, phase_compact |
| `\u{25d4}` | circle with upper-right quadrant | Assigned (half-done) | marketplace_view, job |
| `\u{25d1}` | circle with right half black | Submitted / half-open | marketplace_view, dashboard (circuit), job |

### 1.2 Navigation and Arrows

| Codepoint | Glyph | Meaning | Files |
|-----------|-------|---------|-------|
| `\u{25b8}` | small right triangle | Collapse indicator / active count | plan_tree, plans_view, status_bar, header_bar, dashboard_view |
| `\u{25be}` | small down triangle | Expanded indicator | plan_tree, plans_view |
| `\u{25b2}` | up triangle | Scroll-up indicator | task_progress (scrollbar) |
| `\u{25bc}` | down triangle | Scroll-down indicator | task_progress (scrollbar) |
| `\u{25ba}` | right-pointing pointer | Running task (task_progress) | task_progress |
| `\u{2191}` | up arrow | Network upload / nav hint | sys_metrics, status_bar |
| `\u{2193}` | down arrow | Network download | sys_metrics |
| `\u{2192}` | right arrow | Transition / fix suggestion / trend | marketplace_view, doctor, output_format |

### 1.3 Trend Indicators (dashboard gate thresholds)

| Glyph | Meaning | File |
|-------|---------|------|
| `\u{2191}` up arrow | Gate trend up | dashboard.rs |
| `\u{2192}` right arrow | Gate trend flat | dashboard.rs |
| `\u{2193}` down arrow | Gate trend down | dashboard.rs |

### 1.4 Progress Bars and Block Elements

| Codepoint | Glyph | Meaning | Files |
|-----------|-------|---------|-------|
| `\u{2588}` | full block | Filled portion of bar | header_bar, wave_progress, phase_compact, plan_tree, plans_view, task_progress, sys_metrics, token_sparkline, learning_view, dashboard_view, context_view, marketplace_view, dream_view, agents_view |
| `\u{2591}` | light shade | Empty portion of bar (alt) | plans_view (BLOCKS), task_progress, context_view, marketplace_view, dream_view |
| `\u{2500}` | box-drawing horizontal | Empty portion of bar (primary) / separators | header_bar, wave_progress, phase_compact, plan_tree, plans_view, sys_metrics, token_sparkline, agents_view, error_digest, config_view, config_meta, segment, output_format, develop, context_view |
| `\u{2502}` | box-drawing vertical | Column separator / scrollbar track | plan_tree, plans_view, header_bar, status_bar, agents_view, app |

#### Sub-cell fractional blocks (plans_view only)

| Codepoint | Glyph | Usage |
|-----------|-------|-------|
| `\u{258F}` | left 1/8 block | Smooth progress bar |
| `\u{258E}` | left 1/4 block | Smooth progress bar |
| `\u{258D}` | left 3/8 block | Smooth progress bar |
| `\u{258C}` | left half block | Smooth progress bar |
| `\u{258B}` | left 5/8 block | Smooth progress bar |
| `\u{258A}` | left 3/4 block | Smooth progress bar |
| `\u{2589}` | left 7/8 block | Smooth progress bar |

#### Sparkline block elements (dashboard, learning_view)

| Codepoint | Glyph | Usage |
|-----------|-------|-------|
| `\u{2581}` | lower 1/8 block | Sparkline level 1/8 |
| `\u{2582}` | lower 1/4 block | Sparkline level 2/8 |
| `\u{2583}` | lower 3/8 block | Sparkline level 3/8 |
| `\u{2584}` | lower half block | Sparkline level 4/8 |
| `\u{2585}` | lower 5/8 block | Sparkline level 5/8 |
| `\u{2586}` | lower 3/4 block | Sparkline level 6/8 |
| `\u{2587}` | lower 7/8 block | Sparkline level 7/8 |
| `\u{2588}` | full block | Sparkline level 8/8 |

### 1.5 Error Type Icons (error_digest only)

| Codepoint | Glyph | Meaning |
|-----------|-------|---------|
| `\u{2718}` | heavy X | Gate failure |
| `\u{2692}` | hammer and pick | Compile error |
| `\u{26a0}` | warning triangle | Agent error (also plans_view) |
| `\u{2691}` | flag | Preflight error |
| `\u{26a1}` | lightning | Runtime error |

### 1.6 Miscellaneous Symbols

| Codepoint | Glyph | Meaning | Files |
|-----------|-------|---------|-------|
| `\u{25a0}` | black square | Legend marker (learning chart) | learning_view |
| `\u{25c8}` | diamond inside diamond | Pipeline indicator | plans_view |
| `\u{23F1}` | stopwatch | Elapsed time marker | task_progress |
| `\u{2026}` | horizontal ellipsis | Truncation | util.rs, plan_tree |
| `\u{2014}` | em dash | Missing value / separator | learning_view, marketplace_view, atelier_view, chat, prd, develop, config_cmd, do_cmd, output_sink, dashboard_view, plan_generate |
| `\u{00b0}` | degree sign | Heartbeat frame | status_bar |
| `\u{202F}` | narrow no-break space | Token number formatting | output_format |
| `\u{2212}` | minus sign | Under estimate | output_format |
| `\u{0394}` | delta | Cost difference | output_format |
| `\u{2716}` | heavy multiplication X | Failure detection in output | output_format |
| `\u{00B7}` | middle dot | Event separator | output_format |
| `\u{FEFF}` | BOM | UTF-8 BOM stripping | plan_discovery, plan |
| `\u{00d7}` | multiplication sign | Cost label | learning_view |
| `\u{207b}\u{2074}` | superscript -4 | Cost label (x10^-4) | learning_view |
| `\u{2588}` | full block | Text cursor in input | marketplace_view |

### 1.7 Braille Characters (U+2800..U+28FF)

Used in `widgets/braille.rs` for sparkline rendering. Characters are computed
algorithmically from data points using the standard 8-dot braille encoding.
Each cell encodes two data samples (left/right columns, 4 vertical dots each).

### 1.8 Spinner Animations

**Primary spinner** (`atmosphere.rs`): 10-frame Braille dots sequence
```
'⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'
```
Rate: advances every 4 frames. Used in header_bar for active agent display.

**Ethereal spinner** (`atmosphere.rs`): 4-frame quarter-circle sequence
```
'◜', '◝', '◞', '◟'
```
Rate: advances every 8 frames. Used in phase_compact for active phase.

### 1.9 Heartbeat Animations

**Header bar** (`header_bar.rs`): 2-frame filled/empty circle
```
"\u{25cf}", "\u{25cb}"   (● ○)
```
Rate: advances every 15 frames. Color pulsed by `atmosphere.heartbeat()`.

**Status bar** (`status_bar.rs`): 4-frame sequence
```
"\u{00b7}", "\u{00b0}", ".", "\u{25cf}"   (· ° . ●)
```
Rate: advances every 8 frames. Uses fixed `ROSE_DIM` color.

### 1.10 Box-Drawing Characters

**Thin (light) set only.** Used consistently across all widgets:

| Char | Codepoint | Usage |
|------|-----------|-------|
| `─` | `\u{2500}` | Horizontal rule / empty bar segment / separators |
| `│` | `\u{2502}` | Vertical separator / scrollbar track |
| `└` | `\u{2514}` | Tree branch end (agents_view topology, context_view) |
| `├` | `\u{251C}` | Tree branch middle (agents_view topology) |

**Double-line set** (dashboard.rs `render_boxed_panel` only):

| Char | Codepoint | Usage |
|------|-----------|-------|
| `═` | `\u{2550}` | Double horizontal |
| `║` | `\u{2551}` | Double vertical |
| `╔` | `\u{2554}` | Double top-left corner |
| `╗` | `\u{2557}` | Double top-right corner |
| `╠` | `\u{2560}` | Double left T-junction |
| `╣` | `\u{2563}` | Double right T-junction |
| `╚` | `\u{255A}` | Double bottom-left corner |
| `╝` | `\u{255D}` | Double bottom-right corner |

**Ratatui Borders**: the standard `Block::default().borders(Borders::ALL)` uses ratatui's
default thin box-drawing set (rounded corners by default in ratatui 0.28+).

---

## 2. Consistency Analysis

### 2.1 CONSISTENT: Status Icon Vocabulary

The four core status states use a stable symbol vocabulary across all views:

| State | Symbol | Color (Theme) | Consistent? |
|-------|--------|---------------|-------------|
| Done/Success | `\u{2713}` (checkmark) | SAGE / success | YES |
| Failed/Error | `\u{2717}` (X mark) | EMBER / danger | YES |
| Active/Running | `\u{25b6}` (play) | WARNING / warning | YES |
| Pending/Idle | `\u{25cb}` (open circle) OR `\u{00b7}` (middle dot) | TEXT_GHOST / muted | MOSTLY (see below) |

### 2.2 INCONSISTENT: Pending/Idle Uses Two Different Symbols

The "pending" or "idle" state alternates between two symbols with no clear rule:

- `\u{25cb}` (open circle): plan_tree (pending plan), plans_view (pending plan/task),
  agent_status_grid (idle agent), marketplace_view (open job), agent_serve (idle),
  dashboard (circuit open)
- `\u{00b7}` (middle dot): plan_tree (inactive wave), plans_view (wave pending),
  agents_view (idle agent), task_progress (pending task), gate_output (MIDDLE_DOT const),
  plan_tree (verify placeholder)

**Pattern observed**: `\u{25cb}` tends to mean "explicitly pending/waiting" while
`\u{00b7}` tends to mean "nothing here/inactive/placeholder." But this is not
enforced and several spots use them interchangeably.

### 2.3 INCONSISTENT: Failure Uses Two X Mark Variants

- `\u{2717}` (ballot X): used everywhere as the standard failure icon
- `\u{2718}` (heavy ballot X): used only in `error_digest.rs` for gate failure

These are visually near-identical in most terminal fonts. The error_digest uses a
distinct "heavy" variant but this distinction is unlikely visible to users.

### 2.4 INCONSISTENT: Heartbeat Animation Differs Between Header and Status Bar

The header bar uses a 2-frame filled/empty circle (`● ○`) with brightness pulsing,
while the status bar uses a 4-frame `· ° . ●` sequence at a different rate. Both
convey "alive" but look different on screen.

### 2.5 INCONSISTENT: Middle Dot as Separator vs Status

`\u{00b7}` serves double duty:
1. **Status**: pending/inactive (plan_tree, agents_view, task_progress)
2. **Separator**: between text items (plans_view title sections, agents_view output
   header, task_progress summary, output_format event lines, phase_compact title)

This overloading does not cause confusion in practice because the separator usage
includes surrounding spaces while the status usage is standalone, but a dedicated
separator character would be cleaner.

### 2.6 CONSISTENT: Progress Bars

Two patterns, both consistently applied:

1. **Primary**: `\u{2588}` (full block) for filled + `\u{2500}` (box horizontal)
   for empty. Used in header_bar, wave_progress, phase_compact, plan_tree, plans_view,
   sys_metrics, token_sparkline, agents_view.

2. **Alternate**: `\u{2588}` for filled + `\u{2591}` (light shade) for empty. Used
   in task_progress, context_view, marketplace_view, dream_view, dashboard_view.

The primary pattern is more common. The alternate uses shade blocks that give a
different visual density.

### 2.7 CONSISTENT: Tree Drawing

The agents topology tree uses standard `├──`, `└──`, `│` characters consistently.
The context_view signal DAG also uses `\u{2514}\u{2500}` for tree branches.

### 2.8 CONSISTENT: Braille Sparklines

The braille sparkline module is used uniformly wherever sparklines appear (sys_metrics,
app.rs braille range check). The learning_view uses a different sparkline approach
(vertical block elements `\u{2581}`..`\u{2588}`) for model pass-rate charts.

---

## 3. Missing Symbols / Gaps

### 3.1 No Tab Icons

Tabs are rendered as plain text (`F1 Dashboard`, `F2 Plans`, etc.) with no iconic
prefix. Many TUI applications use tab icons for quick visual recognition.

### 3.2 No Pause/Stop/Retry Icons

The `PAUSED` state in the status bar is shown as a text badge, not an icon.
There is no `\u{23F8}` (pause), `\u{23F9}` (stop), or `\u{21BB}` (retry) symbol
anywhere in the codebase.

### 3.3 No Warning Icon in Standard Widgets

The `\u{26a0}` (warning triangle) appears only in error_digest (agent errors) and
plans_view (old-format plans). There is no systematic warning icon for degraded or
approaching-threshold states.

### 3.4 No Lock/Security Icons

No `\u{1F512}` (lock) or security indicators for authentication states, MCP
connections, or safety layer status.

### 3.5 No Clock/Timer Icons

The stopwatch `\u{23F1}` is used only in task_progress for elapsed time. Other
time-related displays (ETA, plan age, git commit age) use plain text formatting.

### 3.6 No Keyboard/Shortcut Icons

Keybind hints in the status bar use plain arrow text (`\u{2191}\u{2193}:nav`)
rather than visual key representations.

### 3.7 No Notification Bell

Badge counts on tabs are shown as `(3)` text. No bell or notification icon.

---

## 4. Nerd Font / Powerline Compatibility

**Not used.** The entire TUI uses only standard Unicode code points from blocks
that are present in all modern monospace terminal fonts. There are no Nerd Font
glyphs (U+E000..U+F8FF private use area), no Powerline separator triangles
(U+E0A0, U+E0B0..U+E0B3), and no Material Design icons.

**Assessment**: This is the correct design choice. Nerd Font dependency would
break the TUI for any user without a patched font. The current symbol set works
on stock macOS Terminal, iTerm2, Alacritty, Kitty, WezTerm, Windows Terminal,
and GNOME Terminal without font configuration.

---

## 5. Box-Drawing Character Audit

### Sets in Use

1. **Thin/Light set** (primary): `\u{2500}` `\u{2502}` `\u{2514}` `\u{251C}` -- used
   directly in widget code for separators, tree branches, and scrollbar tracks.

2. **Double set** (one location): `\u{2550}` `\u{2551}` `\u{2554}` `\u{2557}` `\u{2560}`
   `\u{2563}` `\u{255A}` `\u{255D}` -- used only in `dashboard.rs` `render_boxed_panel`
   for learning subsystem detail boxes.

3. **Ratatui default** (implicit): ratatui `Block::borders(Borders::ALL)` draws its
   own border set. Since ratatui 0.28, this defaults to plain (not rounded) corners
   unless `.border_type(BorderType::Rounded)` is specified.

### Not Used

- Heavy/thick set (U+2501, U+2503, etc.): absent
- Rounded corners (U+256D..U+2570): not explicitly used (ratatui may apply them)
- Dashed/dotted box-drawing: absent

### Assessment

The mix of thin + double is intentional: double-line boxes provide visual emphasis
for detail panels. However, only one function uses double-line, making it feel
orphaned. Consider either using double-line more broadly or eliminating it.

---

## 6. Braille Pattern Usage

### Current: Sparklines Only

The braille block (U+2800..U+28FF) is used exclusively for sparkline rendering in
`widgets/braille.rs`. Each 2x4 dot cell encodes two data samples, giving 2x
horizontal density compared to block elements.

### Potential Additional Uses

- **Mini-gauges**: 2-row braille could render CPU/MEM bars at 4x vertical
  resolution in a single line
- **Dot plots**: scatter-style visualization for cost vs. time data
- **Activity heatmaps**: braille dots as mini-heatmap cells
- **Connection maps**: braille patterns for topology/network diagrams

### Assessment

Current usage is well-implemented and consistent. The algorithmic encoding is
correct (LEFT_COL_BITS/RIGHT_COL_BITS properly map to braille dot positions).
Additional uses would be valuable but are not urgent.

---

## 7. Progress Indicators

### Current System

Two styles, somewhat inconsistent:

| Style | Filled | Empty | Where |
|-------|--------|-------|-------|
| A (primary) | `\u{2588}` full block | `\u{2500}` horizontal line | 10 widgets |
| B (alternate) | `\u{2588}` full block | `\u{2591}` light shade | 5 widgets |

Additionally, `plans_view` defines a `BLOCKS` constant with 10 fractional left-block
characters (`' '` through `\u{2588}`) for sub-character-width progress precision.
This BLOCKS array is referenced but used only in `plans_view`.

### Verdict

The primary style (full block + horizontal line) should be standardized. The
alternate style (full block + light shade) provides slightly more visual density
for the empty portion, which can be useful in data-heavy views but creates
inconsistency. The fractional blocks in `BLOCKS` are a nice touch for smooth
animation but only one view uses them.

---

## 8. Status Indicator Visual Language

### Current Mapping

| Semantic State | Icon | Color | Bold? | Usage Count |
|----------------|------|-------|-------|-------------|
| Success/Done | checkmark | SAGE (green) | no | ~12 locations |
| Failed/Error | X mark | EMBER (red) | yes | ~11 locations |
| Active/Running | play triangle | WARNING (amber) | yes | ~8 locations |
| Pending/Idle | open circle / middle dot | TEXT_GHOST / muted | no | ~10 locations |
| Circuit Closed | filled circle | (varies) | no | 1 location |
| Circuit Half-Open | half circle | (varies) | no | 1 location |
| Circuit Open | open circle | (varies) | no | 1 location |
| Assigned | quarter circle | info | no | 2 locations |
| Submitted | half circle | info | no | 2 locations |

### Missing States with No Icon

| State | Current Display | Better Option |
|-------|----------------|---------------|
| Paused | Text badge "PAUSED" | `\u{23F8}` (pause icon) |
| Queued | Same as pending | Distinct from pending |
| Retrying | No specific icon | `\u{21BB}` (clockwise arrow) |
| Stalled | Shares "active" icon | Different visual treatment |
| Skipped | No representation | `\u{23ED}` (skip forward) |
| Warning/Degraded | `\u{26a0}` (rare) | Consistent use of `\u{26a0}` |

---

## 9. Spinner Animation Analysis

### Primary Spinner (Braille Dots)

```
Frame 0: ⠋   Frame 1: ⠙   Frame 2: ⠹   Frame 3: ⠸
Frame 4: ⠼   Frame 5: ⠴   Frame 6: ⠦   Frame 7: ⠧
Frame 8: ⠇   Frame 9: ⠏
```

This is the classic "rotating dots" spinner. All characters are within the Braille
Patterns block (U+2800..U+28FF). At 4 frames per advance with a typical 30fps
render rate, one full rotation takes ~1.3 seconds. This is a good choice: subtle,
low visual weight, widely supported.

### Ethereal Spinner (Quarter Circles)

```
Frame 0: ◜   Frame 1: ◝   Frame 2: ◞   Frame 3: ◟
```

These are Miscellaneous Technical characters (U+25DC..U+25DF). At 8 frames per
advance, one rotation takes ~1.1 seconds at 30fps. Used only for the active phase
indicator in `phase_compact`.

### Assessment

Both spinners are well-chosen: monospace-compatible, widely supported, and
visually distinct from data characters. The braille spinner is the better primary
choice because braille patterns have guaranteed fixed width in all terminals.

---

## 10. Proposed Complete Icon System for Roko

### 10.1 Core Status Icons (standardize existing)

| Semantic | Symbol | Codepoint | Notes |
|----------|--------|-----------|-------|
| Success | checkmark | `\u{2713}` | Already standard; keep |
| Failure | X mark | `\u{2717}` | Already standard; drop `\u{2718}` variant |
| Active | play | `\u{25b6}` | Already standard; keep |
| Pending | open circle | `\u{25cb}` | Standardize as THE pending icon |
| Idle/Placeholder | middle dot | `\u{00b7}` | Reserve for "no data" / placeholder only |
| Warning | warning triangle | `\u{26a0}` | Promote to systematic warning icon |
| Paused | pause bars | `\u{23F8}` | New; replace text "PAUSED" |
| Retrying | clockwise arrow | `\u{21BB}` | New; for retry states |
| Skipped | skip forward | `\u{23ED}` | New; for skipped tasks |
| Queued | hourglass | `\u{231B}` | New; distinguish from pending |

### 10.2 Tab Icons (new -- all from safe Unicode blocks)

| Tab | Suggested Icon | Codepoint | Rationale |
|-----|---------------|-----------|-----------|
| F1 Dashboard | `\u{25A3}` white square with rounded corners | `\u{25A3}` | Grid/dashboard feel |
| F2 Plans | `\u{25B7}` white right-pointing triangle | `\u{25B7}` | Execution/flow |
| F3 Agents | `\u{2687}` white circle with dot | `\u{2687}` | Identity/entity (alt: `\u{25C9}` fisheye) |
| F4 Git | `\u{2442}` OCR branch symbol | `\u{2442}` | Branch (alt: simple `*`) |
| F5 Logs | `\u{2261}` identical to | `\u{2261}` | Stacked lines (alt: `\u{2630}` trigram) |
| F6 Config | `\u{2699}` gear | `\u{2699}` | Settings |
| F7 Inspect | `\u{2315}` telephone recorder | `\u{2315}` | Magnifying glass feel (alt: `\u{2316}` position indicator) |
| F8 Market | `\u{2616}` white shogi piece | `\u{2616}` | Exchange (alt: `\u{2261}`) |
| F9 Atelier | `\u{270E}` lower-right pencil | `\u{270E}` | Editing/crafting |
| F10 Learning | `\u{2248}` almost equal to | `\u{2248}` | Approximation/learning (alt: `\u{223F}` sine wave) |

**Caveat**: Tab icons should be tested on all target terminals before adoption.
Some of these glyphs (especially U+2442, U+2687) may render poorly in certain
fonts. A safer minimalist approach uses only block/geometric shapes:

| Tab | Safe Icon | Codepoint |
|-----|-----------|-----------|
| F1 Dashboard | `\u{25A0}` black square | Standard |
| F2 Plans | `\u{25B6}` play triangle | Standard |
| F3 Agents | `\u{25CF}` filled circle | Standard |
| F4 Git | `\u{25C6}` black diamond | Standard |
| F5 Logs | `\u{25AC}` black rectangle | Standard |
| F6 Config | `\u{25C8}` diamond in diamond | Standard |
| F7 Inspect | `\u{25CE}` bullseye | Standard |
| F8 Market | `\u{25CA}` lozenge | Standard |
| F9 Atelier | `\u{25C7}` white diamond | Standard |
| F10 Learning | `\u{25B2}` up triangle | Standard |

### 10.3 Action Icons (new)

| Action | Symbol | Codepoint |
|--------|--------|-----------|
| Fix suggestion | right arrow | `\u{2192}` (already used) |
| Separator | vertical bar | `\u{2502}` (already used) |
| Text separator | middle dot | `\u{00b7}` (already used) |
| Truncation | ellipsis | `\u{2026}` (already used) |
| Missing value | em dash | `\u{2014}` (already used) |
| Timer/elapsed | stopwatch | `\u{23F1}` (already used) |
| Network down | down arrow | `\u{2193}` (already used) |
| Network up | up arrow | `\u{2191}` (already used) |
| Cost delta | delta | `\u{0394}` (already used in CLI) |
| Notification | asterisk/star | `\u{2731}` (new; for badge counts) |

### 10.4 Level/Severity Indicators (new)

| Level | Symbol | Codepoint |
|-------|--------|-----------|
| Critical | double exclamation | `\u{203C}` |
| Error | X mark | `\u{2717}` (existing) |
| Warning | warning sign | `\u{26a0}` (existing, underused) |
| Info | info circle (text) | `\u{2139}` |
| Debug | middle dot | `\u{00b7}` (existing) |
| Trace | ellipsis | `\u{2026}` (existing) |

### 10.5 Progress Bar Standardization

**Recommendation**: Standardize on one bar style. Two options:

**Option A (clean/minimal)**: full block + horizontal line (current primary)
```
[████████──────]  56%
```

**Option B (denser/visible)**: full block + light shade
```
[████████░░░░░░]  56%
```

The `BLOCKS` fractional array from `plans_view` should be promoted to a shared
constant if sub-character precision is desired elsewhere. If not, it should stay
view-local.

### 10.6 Error Category Icons (keep from error_digest)

The error_digest icon set is well-designed and distinctive. It should be promoted
to a shared constant module and reused wherever error categories appear:

| Category | Symbol | Codepoint |
|----------|--------|-----------|
| Gate | heavy X | `\u{2718}` |
| Compile | hammer and pick | `\u{2692}` |
| Agent | warning triangle | `\u{26a0}` |
| Preflight | flag | `\u{2691}` |
| Runtime | lightning | `\u{26a1}` |

---

## Summary of Recommendations

### Priority 1 (Consistency fixes -- low risk)

1. **Standardize pending icon**: Use `\u{25cb}` (open circle) for "pending/waiting"
   everywhere; reserve `\u{00b7}` (middle dot) for separators and placeholder text.
   Affects: agents_view, task_progress, gate_output, plan_tree placeholders.

2. **Unify X mark**: Replace the single `\u{2718}` usage in error_digest with
   `\u{2717}` to match the rest of the codebase, OR keep `\u{2718}` and define it
   as the "gate failure" variant explicitly. The visual difference is negligible.

3. **Unify heartbeat animation**: Both header and status bar should use the same
   heartbeat frames and rate, or make the difference intentionally semantic (e.g.,
   header = system health, status bar = connection health).

### Priority 2 (Standardization -- medium effort)

4. **Standardize progress bar empty character**: Pick either `\u{2500}` or
   `\u{2591}` and use it everywhere. `\u{2500}` (horizontal line) is recommended
   because it is visually lighter and 10 widgets already use it.

5. **Create a shared `icons` module**: Define all icon constants in one place
   (`tui/icons.rs` or similar) so that:
   - Every status icon is defined once
   - Error category icons are reusable
   - Separator characters are centralized
   - Spinner frame arrays live in one location

6. **Add `\u{26a0}` (warning) to status vocabulary**: Currently underused.
   Should appear in header bar for degraded health, status bar for approaching
   thresholds, and alongside any "flailing" agent counts.

### Priority 3 (New features -- higher effort)

7. **Tab icons**: Add a single-character icon before each tab label in the F-key
   strip. Use the "safe" geometric shapes from section 10.2.

8. **Pause icon**: Replace the text "PAUSED" badge with `\u{23F8}` icon + text.

9. **Promote `BLOCKS` fractional array**: Move to a shared location if sub-cell
   progress bars are wanted in more views.

### Not Recommended

- Nerd Font / Powerline symbols: would break compatibility for users without
  patched fonts
- Emoji (U+1F3xx..): terminal rendering of emoji is unreliable (double-width,
  color vs text presentation varies by terminal)
- Heavy box-drawing set: no visual benefit over thin set in terminal rendering
- Rounded box-drawing corners: leave to ratatui's built-in border types

---

## Appendix: Unicode Block Reference

Blocks used or recommended, all with excellent modern terminal support:

| Block | Range | Characters Used |
|-------|-------|-----------------|
| Latin-1 Supplement | U+0080..U+00FF | `\u{00b7}` (middle dot), `\u{00b0}` (degree), `\u{00d7}` (multiply) |
| General Punctuation | U+2000..U+206F | `\u{2014}` (em dash), `\u{2026}` (ellipsis), `\u{202F}` (narrow nbsp) |
| Superscripts and Subscripts | U+2070..U+209F | `\u{207b}\u{2074}` (superscript -4) |
| Letterlike Symbols | U+2100..U+214F | `\u{2139}` (info) |
| Arrows | U+2190..U+21FF | `\u{2191}` (up), `\u{2192}` (right), `\u{2193}` (down), `\u{21BB}` (retry) |
| Mathematical Operators | U+2200..U+22FF | `\u{2212}` (minus), `\u{221E}` (infinity) |
| Miscellaneous Technical | U+2300..U+23FF | `\u{2315}`, `\u{23ED}`, `\u{23F1}`, `\u{23F8}`, `\u{231B}` |
| Box Drawing | U+2500..U+257F | `\u{2500}`..`\u{2502}`, `\u{2514}`, `\u{251C}`, doubles |
| Block Elements | U+2580..U+259F | `\u{2581}`..`\u{2588}`, `\u{2589}`..`\u{258F}`, `\u{2591}` |
| Geometric Shapes | U+25A0..U+25FF | circles, triangles, squares, diamonds |
| Miscellaneous Symbols | U+2600..U+26FF | `\u{2691}` (flag), `\u{2692}` (hammer), `\u{2699}` (gear), `\u{26a0}` (warning), `\u{26a1}` (lightning) |
| Dingbats | U+2700..U+27BF | `\u{2713}` (check), `\u{2717}` (X), `\u{2718}` (heavy X) |
| Braille Patterns | U+2800..U+28FF | Sparklines (algorithmic), spinners (specific frames) |

---

## Implementation Status (2026-09-02 swarm)

Unicode and typography improvements (task #24): icon consistency, box-drawing, text density.
