# 07 — TUI Visual Polish & Animations

**Status**: spec
**Scope**: `crates/roko-cli/src/tui/`, `crates/roko-cli/src/inline/`

## Overview

The TUI dashboard works but several widgets feel static. These enhancements add
motion, feedback, and visual depth to make the dashboard feel alive without adding
external dependencies.

---

## Feature 7A: Animated Counter Roll-Up

When cost, token counts, or task counts change, animate the transition
instead of snapping to the new value.

```
Before: $0.0041 → $0.0312 (instant jump)
After:  $0.0041 → $0.0089 → $0.0156 → $0.0234 → $0.0312 (over ~300ms)
```

- Linear interpolation over 10 frames (~300ms at 30fps)
- Track `display_value` vs `target_value` per metric
- Apply to: cost meter, token counts, task completion counts, gate pass rates

**Implementation**: `AnimatedValue<f64>` struct with `set_target()` and `current()`.
~30 lines. Used in `CostMeter`, `header_bar`, `task_progress`.

---

## Feature 7B: Status Transition Flash

When a task/gate changes status (pending → running → passed/failed), flash
the status indicator briefly.

```
Task "compile" status changes:
  Frame 0-3: ▶ compile     (BRIGHT ROSE, bold)
  Frame 4+:  ▶ compile     (normal ROSE)
```

- 4-frame bright flash (~130ms) on status change
- Apply to: task progress items, gate rungs, agent status
- Track `last_status_change: Instant` per item

**Implementation**: ~15 lines per widget. Check elapsed since status change,
brighten color if < 150ms.

---

## Feature 7C: Progress Bar Shimmer

Active progress bars (tasks in progress) get a subtle shimmer effect — a
bright highlight that sweeps left-to-right periodically.

```
━━━━━━━░░░░░   62%     (static, current)
━━━━╍━━░░░░░   62%     (shimmer moves across filled portion)
```

- Use tick counter to compute shimmer position
- Only on bars with `0 < progress < 1`
- One bright cell that moves across the filled portion every 2 seconds
- Disable for completed (100%) or empty (0%) bars

**Implementation**: ~10 lines in `progress_bar` rendering. Add tick param,
compute highlight position from `(tick / 3) % filled_width`.

---

## Feature 7D: Panel Focus Glow

When keyboard focus moves between panels (PlanTree ↔ TaskProgress ↔ RightPanel),
the newly focused panel border brightens for a moment.

```
Focused:   ROSE_BRIGHT border (first 500ms) → ROSE border (steady)
Unfocused: TEXT_DIM border
```

- Track `focus_changed_at: Instant` in `TuiState`
- Lerp from ROSE_BRIGHT → ROSE over 500ms
- Combined with existing focused/unfocused border styles

**Implementation**: ~10 lines in the view render functions that draw Block borders.

---

## Feature 7E: Heartbeat Pulse Enhancement

The header bar has a heartbeat indicator. Enhance it:

- Pulse rate reflects system health:
  - Healthy (all gates pass): slow pulse (1 Hz)
  - Warning (some failures): medium pulse (2 Hz)
  - Error (build broken): fast pulse (4 Hz)
- Color shifts with health: SAGE → WARNING → EMBER

**Implementation**: Already have `heartbeat_frame` calculation. Just vary the
divisor based on health state. ~5 lines.

---

## Feature 7F: Token Sparkline Enhancement

**Current**: Basic sparkline showing token throughput.
**Target**: Dual-layer sparkline: input tokens (dim) + output tokens (bright),
stacked or overlaid.

```
Current:  ▃▅▇▅▃▁▃▇▅   (single series)
Enhanced: ▃▅▇▅▃▁▃▇▅   output (ROSE_BRIGHT)
          ▁▂▃▂▁▁▂▃▂   input  (ROSE_DIM, underneath)
```

- Two data series rendered in the same sparkline area
- Input tokens dimmer, output tokens brighter
- Hover (if mouse enabled): show exact values

**Implementation**: ~20 lines. Render two passes into the same Rect,
second pass only draws if value > first pass value at that column.

---

## Feature 7G: Keyboard Shortcut Overlay (Shift+?)

Show a cheat sheet overlay with all keyboard shortcuts for the current context.

```
┌─ Keyboard Shortcuts ────────────────┐
│                                      │
│  Navigation                          │
│  F1-F7    Switch tabs                │
│  Tab      Cycle focus                │
│  j/k      Scroll list               │
│                                      │
│  Actions                             │
│  Enter    Expand/collapse            │
│  /        Filter                     │
│  r        Refresh data               │
│  q        Quit                       │
│                                      │
│  Press any key to close              │
└──────────────────────────────────────┘
```

- Modal overlay (centered, 40x16)
- Context-aware: shows different shortcuts for chat vs dashboard
- Dismisses on any keypress

**Implementation**: New modal variant in `tui/modals/`. ~60 lines.

---

## Feature 7H: Responsive Compact Mode

When terminal width < 100, automatically switch to compact layout:

- Hide description columns in tables
- Abbreviate status labels (Running → Run, Failed → Fail)
- Collapse two-panel layouts to single-panel with tab switching
- Reduce padding from 2 cells to 1

When width < 80:
- Ultra-compact: single column, no borders, minimal chrome

**Implementation**: Check `area.width` at the start of each view's `render()`.
~20 lines per view to select between layouts.

---

## Priority Order

1. **7A** Counter roll-up — most noticeable polish, easy to implement
2. **7C** Progress shimmer — subtle but premium feel
3. **7B** Status flash — immediate feedback for state changes
4. **7G** Shortcut overlay — discoverability win
5. **7D** Panel focus glow — subtle spatial feedback
6. **7E** Heartbeat enhancement — communicates system health
7. **7H** Responsive compact — accessibility for small terminals
8. **7F** Sparkline enhancement — data density improvement
