# 148 — TUI God Objects Decomposition (`app.rs` / `state.rs` / `dashboard.rs`)

**Priority**: P3 — Three TUI files (app.rs at 4,576 LOC, state.rs at 5,290 LOC, dashboard.rs at 7,445 LOC) are god objects that make the TUI impossible to modify safely without reading the entire file; decomposition is a prerequisite for sustainable TUI evolution.
**Size**: L (3-5 days each, tracked as three sub-items)
**Crates**: `crates/roko-cli/src/tui/`
**Depends on**: #121 (TUI data model unification should precede this to avoid compounding refactors), #122 (legacy page system removal reduces dead code)
**Sources**: `tmp/backlog/_checklist-gaps.md` §5.1

---

## Background

The TUI module has three god-object files that violate single-responsibility at the extreme end:

- **`app.rs` (4,576 LOC)**: Mixes key input handling, action dispatching, I/O coordination (StateHub subscription, file watching), and snapshot management. A new contributor cannot find "where does pressing `p` do?" without searching through thousands of lines.

- **`state.rs` (5,290 LOC)** (or an equivalent state management file): Mixes plan state, agent state, learning state, and system metrics in one monolithic struct with hundreds of accessors.

- **`dashboard.rs` (7,445 LOC)**: Mixes header rendering, plan panel, agent panel, metrics panel, modal overlays, and sub-tab management in one file.

These files slow every TUI change: touching a bug in the header requires navigating 7,000 lines of dashboard code. Decomposing them into focused modules with clear responsibilities makes each change local and auditable.

Backlog #20 (Event Loop Decomposition) covers `event_loop.rs`; this item covers the TUI-side equivalents.

## Current State

- `crates/roko-cli/src/tui/app.rs` (4,576 LOC) — god object.
- `crates/roko-cli/src/tui/tabs.rs` or `dashboard.rs` (7,445 LOC) — god object.
- Some decomposition may have begun (e.g., separate tab files for F2, F3, etc.); the exact state requires inspection.
- No explicit split plan has been documented.

## Implementation Plan

This item tracks three decomposition efforts as sub-items. Each sub-item follows the same pattern: extract a logical sub-module, move functions and types, fix compilation, verify tests pass, and verify no rendering regression via snapshot comparison.

### Sub-item A: `app.rs` Decomposition

Target modules:
- `tui/key_handler.rs` — key event → `TuiAction` translation; no state mutation.
- `tui/action_dispatcher.rs` — `TuiAction` → state mutation + channel sends.
- `tui/io_coordinator.rs` — StateHub subscription, file watcher, event polling.
- `tui/snapshot_manager.rs` — headless screenshot capture (from #111).

`app.rs` becomes a thin orchestrator that wires the four modules.

### Sub-item B: `state.rs` Decomposition (if it exists as a monolith)

Target modules:
- `tui/plan_state.rs` — plan list, task list, wave data.
- `tui/agent_state.rs` — agent roster, context usage, turn counts.
- `tui/learning_state.rs` — efficiency data, cascade router, gate thresholds.
- `tui/system_state.rs` — CPU/memory/network metrics.

### Sub-item C: `dashboard.rs` / `tabs.rs` Decomposition

If `dashboard.rs` is a monolith, extract:
- `tui/header.rs` — header bar widget (from #124).
- `tui/plan_panel.rs` — plan tree widget (from #125).
- `tui/agent_panel.rs` — agent roster panel.
- `tui/metrics_panel.rs` — cost and metric charts.
- `tui/modal_manager.rs` — modal overlay management.

`dashboard.rs` becomes a thin layout compositor that calls these modules.

## Acceptance Criteria

1. After Sub-item A: `app.rs` is ≤ 500 LOC. Key handling, action dispatch, and I/O coordination are in separate files.
2. After Sub-item B: No single state file exceeds 1,500 LOC. Plan, agent, learning, and system state are in separate files.
3. After Sub-item C: No single rendering file exceeds 2,000 LOC. Header, plan panel, agent panel, and metrics are in separate files.
4. All ten TUI tabs render correctly after each sub-item (verified with snapshot comparison from #111).
5. `cargo test -p roko-cli` passes after each sub-item.
6. No functional changes — only file organization changes.

## Verification Checklist

- [ ] After each sub-item: `roko screenshot` before and after; verify rendered text is identical.
- [ ] After each sub-item: `cargo clippy -p roko-cli -- -D warnings` passes.
- [ ] After Sub-item A: `wc -l crates/roko-cli/src/tui/app.rs` shows ≤ 500.
- [ ] After Sub-item C: `wc -l crates/roko-cli/src/tui/dashboard.rs` (or equivalent) shows ≤ 2,000.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/app.rs` | Thin orchestrator; delegate to sub-modules |
| `crates/roko-cli/src/tui/key_handler.rs` | New file: key event translation |
| `crates/roko-cli/src/tui/action_dispatcher.rs` | New file: action dispatch |
| `crates/roko-cli/src/tui/io_coordinator.rs` | New file: StateHub/file watcher |
| `crates/roko-cli/src/tui/tabs.rs` or `dashboard.rs` | Thin compositor; delegate to panel modules |
| `crates/roko-cli/src/tui/header.rs` | New file: header widget |
| `crates/roko-cli/src/tui/plan_panel.rs` | New file: plan panel (merges plan_tree.rs from #125) |
| `crates/roko-cli/src/tui/modal_manager.rs` | New file: modal overlay management |
