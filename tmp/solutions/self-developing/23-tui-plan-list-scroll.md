# 23: TUI Plan List Scroll Bug

## Symptom

In the F1:dash tab, pressing down arrow in the Plans list moves the selection cursor below the visible area instead of scrolling the viewport to follow. The selected plan becomes invisible.

## Root Cause

Two state values are independent and never coordinated:
- `selected_plan_idx` — updated by arrow keys in `app.rs:879-899`
- `plan_scroll_offset` — never updated by arrow keys, only clamped during render in `plan_tree.rs:175`

When `selected_plan_idx > plan_scroll_offset + visible_height`, the selection goes off-screen.

## Bug Locations

### 1. `crates/roko-cli/src/tui/app.rs` lines 879-899

```rust
TuiAction::SelectPlanDown => {
    let max = self.tui_state.plans.len().saturating_sub(1);
    if self.tui_state.selected_plan_idx < max {
        self.tui_state.selected_plan_idx += 1;
        // BUG: plan_scroll_offset is never updated here
    }
}
```

### 2. `crates/roko-cli/src/tui/widgets/plan_tree.rs` lines 175-182

```rust
// "Scroll to keep selected visible" — but doesn't actually do this
let max_scroll = total_lines.saturating_sub(visible_height);
let scroll_offset = state.plan_scroll_offset.min(max_scroll);  // just clamps, no auto-scroll
```

## State

- `crates/roko-cli/src/tui/state.rs:1040` — `pub selected_plan_idx: usize`
- `crates/roko-cli/src/tui/state.rs:1084` — `pub plan_scroll_offset: usize`

## Fix

In `app.rs`, after updating `selected_plan_idx` in both SelectPlanUp and SelectPlanDown, add scroll adjustment:

```rust
// After updating selected_plan_idx:
if self.tui_state.selected_plan_idx < self.tui_state.plan_scroll_offset {
    self.tui_state.plan_scroll_offset = self.tui_state.selected_plan_idx;
} else if self.tui_state.selected_plan_idx >= self.tui_state.plan_scroll_offset + visible_height {
    self.tui_state.plan_scroll_offset =
        self.tui_state.selected_plan_idx.saturating_sub(visible_height - 1);
}
```

**Complication**: `visible_height` isn't available in `app.rs` — it's computed during render in `plan_tree.rs`. Either:
- Store last-known visible_height in TuiState (simple, works for 99% of cases)
- Use a conservative default like 10 lines
- Move the scroll logic into plan_tree.rs render (requires mutable state ref)

## Working Reference

The Task Picker modal (`app.rs:901-924`) correctly coordinates both values — follow that pattern.

## Files

| File | What |
|------|------|
| `crates/roko-cli/src/tui/app.rs:879-899` | SelectPlanUp/Down handlers (missing scroll update) |
| `crates/roko-cli/src/tui/widgets/plan_tree.rs:175-182` | Render (doesn't auto-scroll) |
| `crates/roko-cli/src/tui/state.rs:1040,1084` | State fields |
