# 130 — Content-Aware Tab Badges (Error/Agent/Gate Counts)

**Priority**: P3 — When focused on one tab, operators miss important changes on other tabs; count badges on inactive tab labels provide at-a-glance awareness without requiring tab switching.
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli/src/tui/tabs.rs`, `crates/roko-cli/src/tui/app.rs`
**Depends on**: #121 (TUI data model unification so badge counts come from a single source)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.10

---

## Background

When a plan fails on F2 while the operator is reading the F7 inspect view, they have no visual signal that something changed. Mori's tab strip solved this with count badges: inactive tab labels showed counts of important items. The Mori reference showed:

```
[F1]Dash [F2]Plans [F3]Agents(2) [F4]Gates [F5]Appr [F6]Cfg [F7]Insp [F8]Log [F9]KG [F10]e:Errors(3)
```

The `(2)` after Agents means 2 agents are currently running. The `e:Errors(3)` means 3 unresolved errors exist (the `e:` prefix is used because the tab is F10, distinct from the F-key abbreviation).

This is a pure rendering change: the badge counts come from `TuiModel` fields that already exist (or will exist after #121 and #126).

## Current State

- `crates/roko-cli/src/tui/tabs.rs` — tab strip renders static labels (`F1:Dash`, `F2:Plans`, etc.).
- No count badges exist on any tab label.
- The data needed for badges is available in `TuiModel`:
  - Active agent count: `model.agents.iter().filter(|a| a.status == AgentStatus::Running).count()`
  - Unresolved error count: `model.error_digest.iter().filter(|e| !e.resolved).count()` (from #126)
  - Pending approvals: `model.pending_approvals.len()`
  - Running gates: `model.gates_running.len()`

## Implementation Plan

1. **Badge rendering helper**: Add a function `render_tab_label(tab: Tab, badge: Option<usize>) -> String` in `tabs.rs`:
   - If `badge` is `Some(n)` and `n > 0`: return `"[F2]Plans(3)"`.
   - If `badge` is `None` or `badge = Some(0)`: return `"[F2]Plans"`.

2. **Compute badges from `TuiModel`**: In the tab strip rendering function, compute:
   - F3:Agents badge: count of running agents.
   - F5:Approvals badge: count of pending approvals.
   - F10:Learn badge: labeled `e:Errors(N)` where N is unresolved error count (from #126).
   - F4:Gates badge: count of currently running gate checks.

3. **Threshold for display**: Only show the badge when the count > 0. When the count is 0, render the label without a badge (not `Plans(0)`).

4. **Active tab**: The active tab label is highlighted by the existing active-tab style; do not add a badge to the active tab (the operator can see its contents directly).

5. **Color coding**: Badge text uses `theme::EMBER` when errors exist (F10), `theme::SAGE` for agents (F3), `theme::WARNING` for pending approvals (F5), `theme::DREAM` for gates (F4).

## Acceptance Criteria

1. When 2 agents are running, the F3:Agents tab label shows `Agents(2)`.
2. When 3 unresolved errors exist, the F10 tab shows `e:Errors(3)`.
3. When 0 agents are running, the F3 tab shows just `Agents` (no badge).
4. The active tab's label does not show a badge.
5. Badge text uses EMBER color for errors and SAGE for active agents.

## Verification Checklist

- [ ] Start a plan with 2 concurrent agents; verify F3 label shows `(2)`.
- [ ] Trigger a gate failure; navigate to F1; verify F10 label shows `e:Errors(1)`.
- [ ] After the failing task is retried and succeeds, verify the F10 badge disappears.
- [ ] Verify the active tab (the one you're looking at) has no badge.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/tabs.rs` | Add badge rendering to tab strip; compute badge counts from `TuiModel` |
| `crates/roko-cli/src/tui/app.rs` | Expose badge count accessors on `TuiModel` |
