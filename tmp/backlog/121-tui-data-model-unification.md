# 121 — TUI Data Model Unification (`DashboardData` + `TuiState` → `TuiModel`)

**Priority**: P2 — Two parallel data models bridged by a conversion function cause subtle bugs where the TUI in standalone mode (reading from disk) and connected mode (reading from StateHub) can disagree on the same runtime state.
**Size**: L (3-5 days)
**Crates**: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/mod.rs`
**Depends on**: None (foundational refactor that unblocks #122 and #125)
**Sources**: `tmp/backlog/_checklist-gaps.md` §2.1, `tmp/backlog/_mori-old-gaps.md` MO-06

---

## Background

The roko TUI has two parallel data models:
1. `DashboardData` — populated from `DashboardSnapshot` (the StateHub push-based data model used when connected to a running plan).
2. `TuiState` — the TUI's internal state struct used by all rendering code.

A conversion function bridges them (`dashboard_data_to_tui_state()` or equivalent). This bridging approach was necessary when TUI rendering was done before the StateHub architecture was finalized. Now it is a maintenance burden: any field added to `DashboardData` must also be added to the bridge function, or it will silently be absent in the TUI.

Backlog item #110 (Deprecate JSONL / StateHub-Only) implicitly requires this unification (Phase 3 of that spec removes JSONL readers from the TUI), but does not spec the structural merge as a discrete step. This item covers that explicit structural refactor.

The `app.rs` file is 4,576 LOC and `state.rs` (or the equivalent in the TUI module) is 5,290 LOC. The refactor will touch both but should produce a net reduction in code by eliminating the bridge layer.

## Current State

- `crates/roko-cli/src/tui/app.rs` (4,576 LOC) — holds `TuiState` and the update logic.
- `crates/roko-cli/src/tui/mod.rs` — TUI entry point; wires StateHub subscription.
- `DashboardData` — snapshot struct from the StateHub projection.
- Conversion bridge: a function that maps `DashboardData → TuiState` fields; exact location needs inspection.
- Two modes: standalone (reads files directly) and connected (StateHub push). Both paths converge through `TuiState` but via different conversion functions.

## Implementation Plan

1. **Audit the existing types**: Read `app.rs` and `mod.rs` to identify every field in `TuiState`, every field in `DashboardData`, and every field that exists in one but not the other.

2. **Design `TuiModel`**: Create a single struct `TuiModel` that is a superset of both `DashboardData` and `TuiState`. Fields that are TUI-only (cursor position, tab index, modal state) remain in `TuiModel` alongside data fields (plan list, agent list, metrics).

3. **Single update function**: Replace the two-path update logic (JSONL reader path + StateHub path) with a single `TuiModel::apply_snapshot(snapshot: &DashboardSnapshot) -> Self` function. For standalone mode (no live StateHub), the snapshot is loaded from disk once and applied.

4. **Eliminate the bridge function**: After all rendering code is updated to reference `TuiModel` fields directly, delete the bridge function and the old `TuiState` type alias or struct.

5. **Staged migration**: To avoid a big-bang refactor, migrate one tab at a time:
   - Phase A: introduce `TuiModel` alongside existing types; make new tabs use it.
   - Phase B: port each existing tab's rendering code from `TuiState` to `TuiModel`.
   - Phase C: delete `TuiState` and the bridge.

6. **Verify no behaviour change**: Run the TUI against the same snapshot data before and after the refactor and compare rendered output using the snapshot engine (#111). The rendered text should be byte-identical.

## Acceptance Criteria

1. A single `TuiModel` struct serves both standalone and connected TUI modes.
2. The bridge conversion function is deleted.
3. All ten tabs render correctly from `TuiModel` fields.
4. Adding a new field to `DashboardSnapshot` requires only adding it to `TuiModel::apply_snapshot()` — not also to a bridge function.
5. Rendered TUI output is identical before and after the refactor (verified with snapshot comparison).
6. `cargo test -p roko-cli` passes after the refactor.

## Verification Checklist

- [ ] Before refactor: capture TUI snapshot with `roko screenshot`.
- [ ] After refactor: capture TUI snapshot and compare with before; verify no rendering differences.
- [ ] `cargo clippy -p roko-cli -- -D warnings` passes after the refactor.
- [ ] Run the TUI in connected mode (against a running plan) and verify all tabs show live data.
- [ ] Run the TUI in standalone mode (no plan running) and verify all tabs show disk data.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/app.rs` | Replace `TuiState` with `TuiModel`; remove bridge function |
| `crates/roko-cli/src/tui/mod.rs` | Update StateHub subscription to produce `TuiModel` |
| `crates/roko-cli/src/tui/tabs.rs` | Update all tab rendering to use `TuiModel` fields |
| `crates/roko-cli/src/tui/` (all tab files) | Update field references from `TuiState` to `TuiModel` |
