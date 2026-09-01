# 22 -- Scrolling, Focus Management, and Selection Behavior Audit

**Audited**: 2026-09-01
**Files examined**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs` (FocusZone, TuiAction, key dispatch)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` (scroll offsets, clamp methods, selection indices)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` (scroll_focused, set_focused_scroll, mouse handler, dispatch_action)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/scroll.rs` (ScrollAccel)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/plan_tree.rs` (scrollbar rendering)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/task_progress.rs` (scrollbar + selection)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/plans_view.rs` (scrollbar rendering)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/logs_view.rs` (auto-tail, scroll/TAIL label)

---

## 1. Independent Scroll States

**Count: 15+ independent scroll states.**

All are fully independent `usize` fields on `TuiState`. None are shared or aliased.

| Field | Used By | Notes |
|---|---|---|
| `plan_scroll_offset` | Plan tree (F1, F2 PlanTree focus) | Index into plan list |
| `log_scroll` | Logs tab (F5) | Offset into filtered log entries |
| `agent_scroll: Option<usize>` | Agent output (F1 AgentOutput, F3) | `None` = auto-tail (follow latest) |
| `agent_topology_scroll_offset` | Agent topology overlay (Ctrl-T) | Separate from agent_scroll |
| `diff_scroll` | Diff panel / right panel | Also used as generic fallback for per-tab zones |
| `procs_scroll` | Processes sub-tab | Independent from diff_scroll |
| `task_scroll` | Task progress list | Task checklist position |
| `command_output_scroll` | Command output / bottom pane | Dashboard bottom panel |
| `plan_detail_scroll` | Plan detail modal | Modal-specific |
| `help_scroll` | Help modal | Modal-specific |
| `config_scroll_offset` | Config tab (F6) | Config key list |
| `gate_output_scroll` | Gate output lines | Declared but no dispatch wiring found |
| Modal `scroll_offset` | WaveOverview, AgentPool, TaskPicker, TaskDetail, QueueOverview | Each modal carries its own scroll_offset in the ModalState enum variant |
| `App::scroll_offset: HashMap<PageId, u16>` | Legacy page-based scroll | Per-page legacy scroll (not used in Mori-style tabs) |

**Assessment**: Good separation. Each panel/modal has its own scroll state. The `diff_scroll` field does double duty as a generic fallback for per-tab zones that have not been given dedicated offsets (see the `_ =>` branch in `scroll_focused`), which is functional but could cause cross-panel scroll bleed if a user moves between tabs that both fall through to `diff_scroll`.

**Gap**: `gate_output_scroll` is declared and initialized to 0 but has no `TuiAction` dispatch or key binding wiring -- it exists in state but is never actually scrolled by user input.

---

## 2. Focus Zones: Tab/Shift-Tab Cycling

**FocusZone** is a 20-variant enum. `next()` and `prev()` are `const fn` methods that define per-tab cycling order.

### Cycling completeness by tab

| Tab | Zones in cycle | Complete? |
|---|---|---|
| **Dashboard** | PlanTree -> TaskProgress -> AgentOutput -> CommandOutput -> RightPanel -> (wrap) | Yes, 5-zone cycle |
| **Plans** | PlanTree <-> RightPanel | Yes, 2-zone toggle |
| **Agents** | AgentOutput <-> RightPanel | Yes, 2-zone toggle |
| **Git** | GitBranches <-> GitDetail | Yes, 2-zone toggle |
| **Logs** | LogList <-> LogDetail | Yes, 2-zone toggle |
| **Config** | ConfigKeys <-> ConfigValues | Yes, 2-zone toggle |
| **Inspect** | InspectTree <-> InspectDetail | Yes, 2-zone toggle |
| **Marketplace** | MarketList <-> MarketDetail | Yes, 2-zone toggle |
| **Atelier** | AtelierList <-> AtelierDetail | Yes, 2-zone toggle |
| **Learning** | LearningMetrics <-> LearningDetail | Yes, 2-zone toggle |

**Bindings**: `Tab` -> `FocusNext`, `BackTab` (Shift-Tab) -> `FocusPrev`. Both are in the global key handler and work from any tab.

**Reset on tab switch**: When switching tabs via F-key or number key, `dispatch_action(SwitchTab)` explicitly sets `focus` to the default zone for that tab (e.g., `PlanTree` for Dashboard/Plans, `AgentOutput` for Agents, etc.). This is correct behavior.

**Assessment**: Complete. All 10 tabs have defined focus cycles. Forward and backward cycling both work.

**Gap**: The per-tab focus zones in `scroll_focused()` do not have specialized logic for the split-pane `LogDetail`, `GitDetail`, `InspectDetail`, `ConfigValues`, `MarketDetail`, `AtelierDetail`, or `LearningDetail` zones -- all of these fall through to the generic `_ =>` branch which manipulates `diff_scroll`. This means keyboard scrolling in those detail zones is functionally correct (it scrolls something) but may not scroll the right content or may share scroll state inappropriately across different right-panel views.

---

## 3. Mouse Scroll

**Implementation**: `handle_mouse()` in `app.rs` translates `MouseEventKind::ScrollUp/Down` into `TuiAction::MouseScrollUp/Down`, which dispatch to `self.scroll_focused(-3)` / `self.scroll_focused(3)`.

The mouse scroll delta is fixed at 3 lines. No acceleration is applied to mouse events (the `scroll_accel` accelerator is only used for keyboard).

**Panel awareness**: Mouse scroll events carry `(x, y)` coordinates, but the dispatch **ignores** them and always uses `self.scroll_focused()`, which routes based on the current `(active_tab, focus)` pair. This means mouse scroll always targets the focused panel, NOT the panel the mouse cursor is physically over.

**Mouse capture**: The `capture_mouse` flag defaults to `false` and is set by `without_mouse_capture()`. Mouse capture (`EnableMouseCapture`) is only sent to the terminal when `capture_mouse` is `true`. In current code, `capture_mouse` defaults to `false` in the builder and tests confirm this, so **mouse scroll events may not be reported by the terminal at all** unless the caller explicitly enables mouse capture.

**Assessment**: Partially functional.

**Gaps**:
1. Mouse scroll targets the focused panel, not the panel under the cursor. This is a common UX limitation but inconsistent with how most modern TUI applications behave (e.g., Mori routes mouse scroll to the panel the cursor is over).
2. Mouse capture is off by default. Unless a calling path sets `capture_mouse = true`, the terminal will not report mouse events, making mouse scroll effectively dead.
3. Only `ScrollUp`, `ScrollDown`, and left-click `Down` are handled. No drag, hover, or right-click events.

---

## 4. Scroll Indicators

### Visual scrollbars

Three implementations exist:

1. **`plan_tree.rs::render_scrollbar`** -- Custom buffer-direct rendering. Draws a thin track (`│` in `TEXT_PHANTOM` color) with a block thumb (`█` in `ROSE_DIM`). Appears on the right edge of the plan tree panel. Correctly calculates proportional thumb size and position based on `total/visible/offset`.

2. **`plans_view.rs::render_scrollbar`** -- Nearly identical buffer-direct implementation. Uses theme accent color for thumb, dark gray for track. Also appears in the plans view right-panel task list.

3. **`task_progress.rs`** -- Uses ratatui's built-in `Scrollbar` widget (`ScrollbarOrientation::VerticalRight`) with up/down arrow symbols (`▲`/`▼`). Thumb styled in `ROSE`, track in `TEXT_PHANTOM`.

### Textual "more" indicators

- `task_progress.rs` renders `"▲ more"` when `start > 0` and `"▼ more"` when `end < tasks.len()`. These are inline text hints, not scroll UI elements.

### Position labels

- `logs_view.rs` renders a `TAIL` / `SCROLL` label in the logs header to indicate whether auto-tail is active or the user has scrolled away from the bottom.

**Assessment**: Scrollbars exist for plan tree, plans view task list, and task progress. Most other panels (agent output, log list, diff panel, config list, inspect panel, modals) have **no visual scrollbar**. The user must infer scrollability from content overflow alone.

**Gap**: Missing scrollbars on: log list, agent output, diff panel, config key list, inspect panels, marketplace/atelier lists, help modal, and all modal overlays (WaveOverview, QueueOverview, TaskDetail, AgentPool). Adding scrollbar indicators to these would improve discoverability.

---

## 5. Auto-Scroll (Log Tail / Agent Follow)

### Log auto-tail

- **Field**: `log_auto_tail: bool` (default `true`).
- **Activation**: Pressing `G` or `End` on the Logs tab sets `log_auto_tail = true` and resets `log_scroll = 0`.
- **Deactivation**: Scrolling up (`k`, `Up`, `PageUp`) sets `log_auto_tail = false` and assigns a fixed scroll offset.
- **Smart re-engage**: Scrolling down past the end (`scroll_logs_by` with positive delta hitting `max_scroll`) automatically re-engages `log_auto_tail = true`.
- **Visual indicator**: The logs header shows `TAIL` when following, `SCROLL` when pinned.
- **Rendering**: `logs_view.rs` checks `view_state.auto_tail` and if true, pins the view to `max_scroll`.
- **Lock position**: The user can "lock" by scrolling up; the TUI will not auto-scroll until the user hits End/G or scrolls to the bottom.

### Agent output auto-follow

- **Field**: `agent_scroll: Option<usize>` (default `None`).
- **Semantics**: `None` = auto-tail (follow latest output). `Some(offset)` = pinned at offset.
- **Activation**: Pressing `G` on the Agents tab sets `agent_scroll = None` (resume auto-tail).
- **Deactivation**: Scrolling up sets `agent_scroll = Some(fixed_offset)`.
- **Smart re-engage**: Scrolling down past `max_scroll` in `scroll_agent_output_by` sets `agent_scroll = None`.

**Assessment**: Both log and agent auto-scroll are well-implemented with intuitive engage/disengage behavior. The `Option`-based agent scroll and `bool`-based log auto-tail are both correct patterns.

**Gap**: No visual indicator for agent output tail mode (unlike the Logs tab's `TAIL`/`SCROLL` label). Users cannot tell at a glance whether agent output is following or pinned.

---

## 6. Vim-Style Navigation

### Key binding audit

| Key | Supported? | Where | Notes |
|---|---|---|---|
| `j` | Yes | All tabs | Maps to ScrollDown/SelectDown per context |
| `k` | Yes | All tabs | Maps to ScrollUp/SelectUp per context |
| `h` | Partial | Dashboard, Plans, Git, Inspect | DrillOut; NOT bound in Logs, Config, Marketplace, Atelier, Learning |
| `l` | Partial | Dashboard, Plans, Git, Inspect | DrillIn; same coverage as `h` |
| `g` | No (as `gg`) | -- | Lowercase `g` is repurposed: Git sub-tab (Dashboard), agent pane group toggle (Agents), Ctrl-G = git reconcile. No `gg` go-to-top binding. `Home` key is available as substitute. |
| `G` | Partial | Agents, Logs | `ScrollAgentEnd` / `ScrollLogEnd`. NOT bound in Dashboard, Plans, Git, Config, Inspect, Marketplace, Atelier, Learning. `End` key works universally. |
| `Ctrl-d` | No (repurposed) | Global | Mapped to `ResetPlanState` (destructive action with confirm dialog). NOT half-page-down. |
| `Ctrl-u` | No | -- | Not bound anywhere. |
| `/` | Partial | Logs, Plans | Log search mode (Logs tab), plan tree filter (Plans tab). Not available in other tabs. |
| `n` / `N` | Partial | Logs only | Next/prev search match. `n` is also `DismissNotification` globally (takes precedence outside Logs tab since global handler runs first, but global only matches non-Logs tabs). |

**Assessment**: Basic `j`/`k` are universal. `h`/`l` for drill navigation work where applicable. Full vim motions (`gg`, `G`, `Ctrl-d`, `Ctrl-u`) are incomplete. `Ctrl-d` and `Ctrl-u` are occupied by other bindings, and `G` only works in Agents and Logs.

**Gap**: No `Ctrl-u` half-page-up anywhere. `Ctrl-d` is consumed by `ResetPlanState`. No `gg` (go to top). These would need new keybindings or modifier combinations to avoid conflicts.

---

## 7. Selection Visual Distinction

### Plan tree

The plan tree widget in `plan_tree.rs` applies per-line rendering with the selected plan index. Selected items use **bold text on a highlight background** (`Theme::BG_HIGHLIGHT` background, `Theme::BONE` foreground with `Modifier::BOLD`). Active plans also get accent-colored status indicators.

### Task progress

`task_progress.rs` lines 148-158: selected tasks get `Style::default().fg(Theme::BONE).add_modifier(Modifier::BOLD).bg(Theme::BG_HIGHLIGHT)`. The status icon style also gets the highlight background applied.

### Log list

`logs_view.rs` lines 215-240: The selected row gets `theme.selection()` style and a `"▶"` prefix marker. Non-selected rows get a space prefix. Search matches additionally get a `highlight_style` with `DREAM` background.

### Agent list

`parallel_pool.rs` line 57-58: Selected agent row uses `theme.selection()`.

### Config keys

`config_view.rs` line 258-259: Selected config item gets `theme.selection().add_modifier(Modifier::BOLD)`.

### Marketplace / Atelier

`marketplace_view.rs` line 194: Selected job uses `theme.selection()`.
`atelier_view.rs` line 286: Selected PRD uses `theme.selection()`.

### Modal selections

- Task picker: `theme.selection()` for selected row
- Queue overview: `"> "` prefix and `theme.selection()` for selected milestone
- Cost table: `row_highlight_style(theme.selection())` via ratatui Table

**Assessment**: Selection is consistently communicated via `theme.selection()` style (typically background highlight + bold text) and sometimes with a prefix marker (`▶` or `>`). The pattern is uniform across most widgets.

---

## 8. Multi-Select

**No multi-select capability exists anywhere in the TUI.**

Grepping for `multi_select`, `selection_set`, `selected_items`, or `select_all` returns zero matches. Every view uses a single `usize` index for selection:
- `selected_plan_idx`
- `selected_agent`
- `marketplace_selected_job`
- `atelier_selected_prd`
- `task_scroll` (used as selection index for tasks)

There is no `HashSet<usize>` or `Vec<bool>` tracking multiple selections, no Shift+click or Ctrl+click handling, and no bulk action on selected items.

**Assessment**: Single-select only. Multi-select is not implemented and no current use case demands it (actions are always on the single focused item).

---

## 9. Page Up/Down Consistency

### Bindings

`PageUp` -> `TuiAction::ScrollPageUp` and `PageDown` -> `TuiAction::ScrollPageDown` are mapped in **every per-tab key handler**: Dashboard, Plans, Agents, Git, Logs, Config (via ScrollFocused fallback), Inspect, Marketplace (via Home/End only -- PageUp/PageDown are NOT explicitly bound), Atelier (NOT bound), and Learning (NOT bound).

Wait -- checking more carefully:

| Tab | PageUp/Down bound? |
|---|---|
| Dashboard | Yes, explicit |
| Plans | Yes, explicit |
| Agents | Yes, explicit |
| Git | Yes, explicit |
| Logs | Yes, explicit |
| Config | No (only j/k/h/l/Enter/Space) |
| Inspect | Yes, explicit |
| Marketplace | No (only j/k/Enter/Home/End) |
| Atelier | No (only j/k/Enter/Home/End) |
| Learning | No (only j/k/Home/End) |

### Delta calculation

`page_scroll_lines()` returns `terminal_height - 4`, clamped to a minimum of 1. This is applied via `scroll_focused()`, which routes through the same tab+focus dispatch as regular scrolling.

### Help modal

Help modal has dedicated `ScrollPageUp` / `ScrollPageDown` handling with a fixed delta of 10 (not the terminal-height-based value).

**Assessment**: Inconsistent. PageUp/PageDown is missing from Config, Marketplace, Atelier, and Learning tab key handlers. The delta is consistent where bound (terminal height - 4 for views, 10 for help modal).

**Gap**: Config, Marketplace, Atelier, and Learning tabs should add `PageUp`/`PageDown` bindings for consistency.

---

## 10. Scroll Bounds: Edge Behavior

### Stops at edges (no wrapping)

All scroll implementations use `saturating_sub` for upward movement and `.min(max)` / `.clamp(0, max)` for downward movement. **Scrolling never wraps around.**

Specific implementations:
- `scroll_focused()` uses `(current + delta).max(0) as usize` for PlanTree, TaskProgress, CommandOutput, RightPanel -- stops at 0, no upper clamp in this method (relies on render-time clamping).
- `scroll_agent_output_by()` uses `saturating_sub` upward and `.min(max_scroll)` downward.
- `scroll_logs_by()` uses `saturating_sub` upward and `.min(max_scroll)` downward, with auto-tail re-engage at bottom.
- Selection indices (plan, agent, marketplace, atelier) use `.saturating_sub(1)` up and `.min(max)` down.

### Upper bound clamping

`clamp_scroll_state_to_view()` runs on each render frame and clamps agent scroll, agent topology scroll, git diff scroll, and log scroll to their current maximums. This prevents stale scroll positions from exceeding content after data refresh.

However, `plan_scroll_offset`, `task_scroll`, `command_output_scroll`, `diff_scroll` (outside Git tab), `config_scroll_offset`, and `procs_scroll` do NOT have render-time clamping in `clamp_scroll_state_to_view()` -- the Plans, Config, Inspect, Marketplace, Atelier, and Learning tabs are in the `_ => {}` fallthrough branch that does nothing.

**Assessment**: Scrolling stops at edges; no wrap-around. Lower bound (0) is universally enforced. Upper bound is enforced for Dashboard/Agents/Git/Logs tabs at render time, but NOT for Plans, Config, Inspect, Marketplace, Atelier, or Learning -- stale scroll positions could exceed content length after data refresh.

**Gap**: `clamp_scroll_state_to_view()` should be extended to cover Plans, Config, Inspect, Marketplace, Atelier, and Learning tabs to prevent scroll overflow after data changes.

---

## Summary of Gaps

| # | Gap | Severity | Location |
|---|---|---|---|
| G1 | `gate_output_scroll` declared but never wired to input dispatch | Low | `state.rs:1280` |
| G2 | Mouse scroll targets focused panel, not panel under cursor | Medium | `app.rs:2053-2054` |
| G3 | Mouse capture is off by default; mouse events may be dead | Medium | `app.rs:656`, `app.rs:3754` |
| G4 | Many panels lack visual scrollbars (agent output, log list, diff, config, inspect, modals) | Low | Render paths |
| G5 | No visual indicator for agent output tail/scroll mode | Low | Views: agents_view |
| G6 | `G` (go-to-end) only bound in Agents and Logs; missing from 8 other tabs | Low | `input.rs` per-tab handlers |
| G7 | No `Ctrl-u` half-page-up; `Ctrl-d` repurposed for ResetPlanState | Low | `input.rs:756-758` |
| G8 | PageUp/PageDown missing from Config, Marketplace, Atelier, Learning tabs | Medium | `input.rs:972-1051` |
| G9 | `clamp_scroll_state_to_view()` does not clamp Plans/Config/Inspect/Marketplace/Atelier/Learning | Medium | `app.rs:2700-2706` |
| G10 | `diff_scroll` used as fallback for all per-tab zones without dedicated scroll state (GitDetail, LogDetail, InspectDetail, etc.) | Low | `app.rs:2420-2424` |
| G11 | `scroll_focused` detail-zone fallthrough: scrolling in e.g. LogDetail or InspectDetail writes to `diff_scroll`, which may conflict | Low | `app.rs:2420-2424` |
| G12 | No multi-select capability anywhere (single-select only) | Informational | Architecture |
| G13 | Duplicate scrollbar rendering code in plan_tree.rs and plans_view.rs | Low | `plan_tree.rs:746`, `plans_view.rs:1200` |
