# 125 — Plan Tree Wave Hierarchy Widget (F2:plans Left Panel)

**Priority**: P2 — The F2:plans left panel shows a flat plan list; Mori's equivalent showed a collapsible wave/plan/task tree with progress bars and health indicators per wave, which is essential for understanding multi-plan run state at a glance.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/tui/`
**Depends on**: #117 (wave computation), #116 (queue manifest provides milestone names), #121 (data model unification)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.5, `tmp/backlog/_mori-old-gaps.md` MO-07

---

## Background

When running 10-30 plans, the current flat plan list on the F2:plans tab becomes unwieldy. Plans in wave 3 look the same as plans in wave 0, and there is no visual grouping by milestone or wave. Mori's F2 left panel solved this with a collapsible hierarchical tree:

```
Wave 0: ▶ [████████] 3/3 complete
  ✓ 01-task-dag            [DONE] 2m14s
  ✓ 02-event-loop          [DONE] 4m32s
  ✓ 03-gate-pipeline       [DONE] 1m58s

Wave 1: ▶ [████░░░░] 1/3 running  ← ACTIVE
  ✓ 10-tui-tabs            [DONE] 5m01s
  ⟳ 11-header-bar          [RUN]  1m22s elapsed
  ○ 12-inspect-view        [WAIT]

Wave 2: — (blocked on wave 1)
  ○ 20-learning-loops      [WAIT]
```

Key visual elements: wave header with aggregate progress bar, plan rows with status icon (✓/⟳/✗/○) and timing, task-level expansion (press Space to expand a plan row to show its tasks), and wave blocker chains (which plan in wave N-1 is blocking wave N).

## Current State

- `crates/roko-cli/src/tui/tabs.rs` — F2:plans tab renders a flat table of plans.
- Wave data is absent (requires #117); plan rows do not have wave groupings.
- No collapsible tree widget; ratatui's `List` widget is used (flat).
- Status icons exist in some form but are not wave-grouped.
- `TuiModel` will carry wave data after #117 is integrated.

## Implementation Plan

1. **`PlanTreeWidget` struct**: Create a new widget in `crates/roko-cli/src/tui/plan_tree.rs`:
   - Input: `&TuiModel` (containing `PlanDag` wave data).
   - Output: ratatui `Widget` trait implementation.
   - State: `collapsed: HashSet<u32>` (which wave indices are collapsed).

2. **Wave header row**: For each wave, render a header row showing:
   - Wave index and name (e.g., `Wave 0: MVP`)
   - Aggregate progress bar (completed/total plans in that wave)
   - One-line status summary (e.g., `3/3 complete` or `1/3 running`)
   - Triangle toggle (▶ collapsed, ▼ expanded)

3. **Plan row (within expanded wave)**: Status icon + plan name + status + elapsed time. Icons:
   - `✓` green: completed
   - `⟳` cyan: running
   - `✗` red: failed
   - `○` muted: waiting
   - `⊘` amber: skipped

4. **Task-level expansion**: When a plan row is selected and Space is pressed, expand it to show individual task rows indented below the plan row. Tasks show their own status icon and name.

5. **Wave blocker info**: For waves not yet active (blocked on previous wave), show which plans in the previous wave are still running and blocking progress.

6. **Keyboard navigation**: `j`/`k` or arrow keys navigate rows (wave headers and plan rows). Space toggles plan expansion. `h`/`l` collapse/expand the selected wave. Tab order follows visual order.

7. **Replace the existing plan list**: In `tabs.rs` F2 rendering, replace the flat `List` widget with `PlanTreeWidget`. Keep the right panel (plan details) unchanged.

## Acceptance Criteria

1. F2:plans tab shows plans grouped under wave headers.
2. Each wave header has an aggregate progress bar and plan count.
3. Wave headers can be collapsed/expanded with Space or `h`/`l`.
4. Each plan row shows status icon, name, and elapsed time.
5. A plan row can be expanded with Space to show its task list.
6. Blocked waves show which plans are blocking them.
7. When wave data is unavailable (wave computation not done), fall back to the current flat list gracefully.

## Verification Checklist

- [ ] With a plan set that has two waves, verify wave headers appear on F2.
- [ ] Press `h` on a wave header and verify plans beneath it disappear.
- [ ] Press Space on a running plan and verify task rows appear.
- [ ] Running plan row shows elapsed time incrementing.
- [ ] Wave 1 header shows "blocked on: plan-name (wave 0)" when wave 0 is incomplete.
- [ ] With `--screenshots` flag, the F2 snapshot shows the wave hierarchy.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/plan_tree.rs` | New file: `PlanTreeWidget` struct and implementation |
| `crates/roko-cli/src/tui/tabs.rs` | Replace flat plan list with `PlanTreeWidget` on F2 |
| `crates/roko-cli/src/tui/mod.rs` | Export `plan_tree` module |
| `crates/roko-cli/src/tui/app.rs` | Add wave collapse state to `TuiModel` |
