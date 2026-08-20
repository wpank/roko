# 157 — Context-Sensitive Keybind Hints

**Priority**: P2 — discoverability; operators don't know what actions are available on each tab
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/tui/`
**Depends on**: #119 (TUI recovery keybindings — adds the recovery-specific keys that need context-sensitive hints)
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` §2.9, `tmp/mori-old/01-MORI-TUI-ARCHITECTURE.md`

---

## Background

Mori's TUI showed different keybind hints in the status bar depending on which tab was active AND what was selected within that tab. For example:
- On F1:Dashboard with no selection: `q:Quit  p:Pause  ?:Help  F1-F8:Tabs`
- On F2:Plans with a failed plan selected: `s:Retry  z:Diagnose  S:Repair  R:Reverify  c:Clean-Slate`
- On F3:Agents with an agent selected: `k:Kill  r:Restart  l:Logs`

This context-sensitivity is critical for discoverability — operators learn available actions by seeing them appear when relevant, rather than memorizing a static help page.

Backlog #119 adds 5 recovery keybindings (s/z/S/R/c) that only make sense when a failed task is selected. This item extends that pattern to ALL tabs and selections, creating a general-purpose context-sensitive hint system.

## Current State

The bottom status bar is rendered by `crates/roko-cli/src/tui/widgets/status_bar.rs`. It has five sections: git info (branch + commit + age), heartbeat + pause indicator, plan progress + health summary, cost/budget utilization, and keybind hints.

The keybind hints are produced by a private `key_hints_for_tab(tab: Tab, has_failures: bool) -> &'static str` function (lines 160-179). This function returns a single static string per tab with only one level of context-sensitivity: the Dashboard tab conditionally shows `R:retry  D:diag` when `has_failures` is true. All other tabs return a fixed string regardless of what is selected within the tab.

The `render_status_bar` function is called by `App::render_status_footer` in `app.rs` (line 2534) and receives only the shared `TuiState` — it does not receive the per-view `ViewState` that tracks sub-tab selection and scroll position.

Each tab has its own key handler in `crates/roko-cli/src/tui/input.rs`:
- `handle_dashboard_key` — sub-tabs (a/o/d/e/g/m/L/P), wave nav, inject, approve, pause
- `handle_plans_key` — plan operations (d:diagnose, m:merge, s:retry, z:reverify, S:repair, R:restart, c:reverify, F:force, V:reverify), wave nav, filter, drill, expand
- `handle_agents_key` — role tabs (1-7, backtick), approve/reject (a/A/x), inject (i), topology (t), group (g)
- `handle_git_key` — drill in/out, expand/collapse
- `handle_logs_key` — level filter (1-4), all (a), search (/), tail (G)
- `handle_config_key` — nav, toggle, cycle left/right, Ctrl-S save
- `handle_inspect_key` — drill in/out, expand/collapse
- `handle_marketplace_key` — new job (n), refresh (r), Ctrl-S submit
- `handle_atelier_key` — publish (p), generate plan (g)
- `handle_learning_key` — navigation only

Selection state is tracked in `TuiState` (in `state.rs`): `active_tab: Tab`, `selected_plan_idx: usize`, `focus: FocusZone`, and `is_paused: bool`. Per-view state lives in `ViewState` (in `views/mod.rs`): `scroll`, `selected`, `sub_tab`, `secondary_selected`, `auto_tail`, `search_query`.

The fundamental limitation is that `key_hints_for_tab` cannot reflect selection-dependent actions (e.g., recovery keys only when a failed plan is selected in F2) because it only branches on `has_failures` globally, not on whether the current selection is a failed item. It also cannot reflect sub-view context (e.g., different keys when the F6:Config Providers sub-view is active vs the Config Editor sub-view).

## Implementation Plan

1. **Replace `key_hints_for_tab` with a `KeyHints` struct**: Define a struct that holds a list of `(key_label, action_description)` pairs and can render itself as a formatted `Span` list:
   ```rust
   pub struct KeyHints {
       contextual: Vec<(&'static str, &'static str)>,  // e.g., ("s", "Retry")
       global: Vec<(&'static str, &'static str)>,       // e.g., ("q", "Quit")
   }
   ```
   The contextual hints are left-aligned and change per tab/selection. The global hints are right-aligned and always present.

2. **Add `fn key_hints(state: &TuiState, view_state: &ViewState) -> KeyHints` per tab**: Replace the single `key_hints_for_tab` match with per-tab functions that inspect both `TuiState` and `ViewState` to determine which keys are relevant. Specific context rules:
   - **F1:Dashboard**: Always show `p:Pause`, `Tab:Panel`, sub-tab keys. If paused, show `p:Resume` instead.
   - **F2:Plans**: Always show navigation and drill keys. If `selected_plan_idx` points to a plan with `tasks_failed > 0`, add recovery keys (s/z/S/R/c from #119). If filter mode is active, show filter editing keys instead.
   - **F3:Agents**: Always show navigation. If focus is `AgentOutput`, show `G:Bottom`. Show approval keys (a/A/x) only when an approval is pending. Show `i:Inject` when an agent is selected.
   - **F4:Git**: Show `h/l:Drill`, `Enter:Expand`.
   - **F5:Logs**: Show level keys (1-4), `a:All`, `/:Search`. If auto-tail is active, show `G:Tail ON`.
   - **F6:Config**: Show nav keys. If config edit mode is active, show `Enter:Save`, `Esc:Cancel`.
   - **F7:Inspect**: Show `h/l:Drill`, `Enter:Expand`.
   - **F8:Marketplace**: Show `n:New`, `r:Refresh`.
   - **F9:Atelier**: Show `p:Publish`, `g:Gen Plan`.
   - **F10:Learning**: Show navigation only.

3. **Pass `ViewState` into `render_status_bar`**: Update the signature of `render_status_bar` to accept `&ViewState` alongside `&TuiState`. Update the call site in `App::render_status_footer` (app.rs line 2534) to pass the current view state.

4. **Render hints as a styled `Span` list**: Replace the single `format!(" {keys}")` span (line 150-153) with two groups of spans:
   ```
   s:Retry  z:Diagnose  S:Repair  │  q:Quit  ?:Help  F1-F10:Tabs
   ```
   Use `Theme::BONE` for key labels and `Theme::FG_DIM` for descriptions. Separate contextual from global with a `│` divider in `Theme::ROSE_DIM`. Right-align global keys by computing the total width of contextual + global and padding with spaces.

5. **Truncation for narrow terminals**: If the combined hint width exceeds the available space (the remaining width after git info + heartbeat + progress + cost), drop hints from right to left within the contextual group first, then within the global group, preserving `q:Quit` as the last to be dropped.

6. **Global keys constant**: Define a constant list of global keys that always appear:
   ```rust
   const GLOBAL_HINTS: &[(&str, &str)] = &[("q", "Quit"), ("?", "Help"), ("F1-F10", "Tabs")];
   ```

## Acceptance Criteria

1. The status bar shows different keybind hints when switching between tabs (F1 shows dashboard keys, F2 shows plan keys, etc.)
2. Within F2:Plans, the hints change when the selected plan has failed tasks vs when all tasks are passing — recovery keys (s/z/S/R/c) only appear in the former case
3. Global keys (q, ?, F1-F10) are always visible in the status bar footer regardless of tab
4. Contextual and global key groups are visually separated by a `│` divider
5. Hints update immediately when tab or selection changes (no stale hints from a previous tab)
6. Narrow terminals truncate hints gracefully — contextual keys drop first, global keys preserved as long as space permits, no panic or layout overflow

## Verification Checklist

- [ ] Launch TUI via `cargo run -p roko-cli -- dashboard` — status bar shows F1 dashboard keys including `p:Pause` and sub-tab hints
- [ ] Press F2 — status bar changes to F2 plan keys (navigation, drill, filter)
- [ ] In F2, arrow to a plan with failed tasks — recovery keys (s/z/S/R/c) appear in the hint bar
- [ ] In F2, arrow to a plan with all tasks passing — recovery keys disappear
- [ ] Press F3 — status bar shows agent keys (role tabs, inject, approve)
- [ ] Press F5 — status bar shows log level keys (1-4, a, /)
- [ ] Press F6 — status bar shows config nav keys
- [ ] Resize terminal to narrow width (<80 cols) — hints truncate without crashing or overflowing the status bar
- [ ] Press `?` — help overlay appears (unaffected by this change)
- [ ] Global keys (q, ?, F1-F10) visible on every tab

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/tui/widgets/status_bar.rs` | Replace `key_hints_for_tab` with `KeyHints` struct and per-tab builder functions; update `render_status_bar` signature to accept `&ViewState`; render contextual/global groups with styled spans and truncation |
| `crates/roko-cli/src/tui/app.rs` | Update `render_status_footer` call to pass the active `ViewState` into `render_status_bar` |
| `crates/roko-cli/src/tui/widgets/mod.rs` | Export `KeyHints` if it needs to be public (may stay private to `status_bar`) |
