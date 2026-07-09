# Roko TUI — Full Mori Parity Audit

**Reference**: Mori at `/Users/will/dev/uniswap/bardo/apps/mori/src/`
**Target**: Roko at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`

---

## 1. Tabs

### Mori: 7 tabs (F1-F7 + F8 queue overlay)

| Key | Tab | View file | Description |
|-----|-----|-----------|-------------|
| F1 | Dashboard | `views/dashboard.rs` | Master-detail: plan tree + phase + tasks (left 38%), 7-sub-tab right panel |
| F2 | Plans | `views/plans.rs` | Wave/plan hierarchical browser (left) + plan detail (right) |
| F3 | Agents | `views/agents.rs` | Parallel pool or role roster (left) + agent output (right) |
| F4 | Git | `views/git_view.rs` | Branch tree + worktrees (left), commit graph + branch info (right) |
| F5 | Logs | `views/logs.rs` | Scrollable log tail |
| F6 | Config | `views/config.rs` | Live config editor |
| F7 | Inspect | `views/context.rs` | MCP summary + token burn sparklines + 3-column server/index/tool panel |
| F8 | Queue | `modals/queue_overview.rs` | Milestone progress browser (modal overlay) |

### Roko: 6 tabs (F1-F6)

| Key | Tab | View file | Status |
|-----|-----|-----------|--------|
| F1 | Dashboard | `views/mori_dashboard.rs` | Present — sub-tabs limited (5 vs Mori's 7) |
| F2 | Plans | `views/plans.rs` | Present — flat/wave view only, NO right-panel plan detail |
| F3 | Agents | `views/agents.rs` | Present — stub (31 lines), no parallel pool |
| F4 | Logs | `views/logs.rs` | Present — errors + gate verdicts only |
| F5 | Signals | `views/signals.rs` | Present — roko-specific (no Mori equivalent) |
| F6 | Config | `views/config.rs` | Present — read-only stats, not a live editor |

### Missing tabs:
- **Git (F4 in Mori)** — branch tree, worktrees, commit graph, branch info panels
- **Inspect (F7 in Mori)** — MCP status, token burn sparklines, server/index/tool panels
- **Queue overlay (F8 in Mori)** — milestone progress browser

### Sub-tabs (Dashboard right panel):

| Key | Mori sub-tab | Roko sub-tab | Status |
|-----|-------------|--------------|--------|
| a | Agents | Agents | Present |
| o | Output | Output | Present |
| d | Diff | Diff | Present (shows gate output as fallback) |
| e | Errors | Errors | Present |
| g | Git | Git | Present (minimal placeholder) |
| m | Context/MCP | — | **MISSING** |
| P | Processes | — | **MISSING** |

---

## 2. Modals / Overlays

### Mori: 13 modals

| Modal | File | Description | Roko status |
|-------|------|-------------|-------------|
| Help | `modals/help.rs` | 2-column keybinding reference, 7 sections, ~30 keys | Present but thin (~8 keys) |
| Plan Detail | `modals/plan_detail.rs` | Scrollable plan detail with Summary/PlanDetails tab bar | Present but uses old types, no tab bar |
| Task Detail | `modals/task_detail.rs` | Task metadata + live efficiency + fixture + support status | Present but stub |
| Approval | `modals/approval.rs` | Agent command approval: `[y]approve [n]reject` | **MISSING** |
| Inject | `modals/inject.rs` | Free-text message injection to active agent | **MISSING** |
| Confirm | `modals/confirm.rs` | Generic destructive action confirm (10+ action types) | **MISSING** |
| Notification/Toast | `modals/notification.rs` | Stacked bottom-right toasts with LogLevel coloring | **MISSING** |
| Wave Overview | `modals/wave_overview.rs` | Wave progress popup | **MISSING** |
| Queue Overview | `modals/queue_overview.rs` | Milestone progress browser (left list + right detail) | **MISSING** |
| Agent Pool Modal | `modals/agent_pool_modal.rs` | Full agent roster with all columns | **MISSING** |
| Task Picker | `modals/task_picker.rs` | Scrollable searchable task list with Enter-to-ingest | **MISSING** |
| Batch Review | `modals/batch_review.rs` | Batch-pause review modal wrapping review view | **MISSING** |
| Quit Confirm | `modals/quit.rs` | Quit confirmation | **MISSING** (roko quits immediately) |

### Score: 3/13 present (all 3 thin/incomplete)

---

## 3. Widgets

### Mori: 26 widgets

| Widget | File | LOC | Roko status |
|--------|------|-----|-------------|
| agent_grid | `widgets/agent_grid.rs` | ~3,800 | Present |
| agent_output | `widgets/agent_output.rs` | ~60,200 | Present but **THIN** — no segment parsing, no render cache, no nerv_viz, no scrollbar |
| agent_pool | `widgets/agent_pool.rs` | ~7,400 | Present — misses ParallelAgentState path |
| braille | `widgets/braille.rs` | ~3,000 | Present |
| branch_tree | `widgets/branch_tree.rs` | ~6,100 | **MISSING** |
| command_output | `widgets/command_output.rs` | ~7,400 | Present |
| context_gauge | `widgets/context_gauge.rs` | ~3,600 | **MISSING** |
| diff_panel | `widgets/diff_panel.rs` | ~5,000 | **MISSING** |
| error_digest | `widgets/error_digest.rs` | ~12,800 | Present |
| header_bar | `widgets/header_bar.rs` | ~14,100 | Present |
| parallel_pool | `widgets/parallel_pool.rs` | ~9,900 | **MISSING** |
| phase_bar | `widgets/phase_bar.rs` | ~11,200 | Present |
| phase_compact | `widgets/phase_compact.rs` | ~13,300 | Present |
| phase_timeline | `widgets/phase_timeline.rs` | ~5,400 | **MISSING** |
| plan_list | `widgets/plan_list.rs` | ~13,200 | **MISSING** |
| plan_tree | `widgets/plan_tree.rs` | ~37,300 | Present |
| scrollbar | `widgets/scrollbar.rs` | ~1,300 | Present |
| status_badge | `widgets/status_badge.rs` | ~2,300 | **MISSING** |
| status_bar | `widgets/status_bar.rs` | ~8,000 | Present |
| sys_metrics | `widgets/sys_metrics.rs` | ~10,300 | Present |
| tab_bar | `widgets/tab_bar.rs` | ~1,100 | **MISSING** |
| task_progress | `widgets/task_progress.rs` | ~23,500 | Present |
| token_bar | `widgets/token_bar.rs` | ~3,800 | Present |
| token_sparkline | `widgets/token_sparkline.rs` | ~6,600 | Present |
| wave_bar | `widgets/wave_bar.rs` | ~4,700 | **MISSING** |
| wave_progress | `widgets/wave_progress.rs` | ~4,200 | Present |

### Score: 17/26 present, 9 missing

### Present but significantly incomplete:
- **agent_output** — Mori's is 60K LOC with ANSI segment parsing, CachedRender, nerv_viz background, integrated scrollbar, parallel agent tabs. Roko's is a fraction.
- **agent_pool** — Missing ParallelAgentState-aware parallel pool rendering
- **plan_tree** — Mori's is 37K LOC with filter overlay, expand/collapse, rich task rows. Roko's is simpler.

---

## 4. Key Bindings

### Mori input system: `tui/input.rs`

Mori has a full input routing system with:
- **4 input modes**: Normal, Inject, Filter, Confirm
- **Modal intercepts** (highest priority): task picker, task detail, queue overview each consume keys
- **Per-tab key routing**: each tab has its own key handler
- **Focus-aware routing**: keys behave differently based on which panel has focus
- **Mouse support**: click + scroll routing via hit_test.rs

### Full Mori key inventory:

#### Global (all modes):
| Key | Action | Roko |
|-----|--------|------|
| Ctrl-C | Quit | Missing (roko uses q/Esc) |
| Ctrl-r | Restart all plans (confirm) | **MISSING** |
| Ctrl-x | Force advance (confirm) | **MISSING** |
| Ctrl-d | Reset selected plan (confirm) | **MISSING** |
| Ctrl-g | Git reconcile (confirm) | **MISSING** |
| Ctrl-a | Approve all pending | **MISSING** |
| Ctrl-t | Open task picker | **MISSING** |
| F1-F7 | Switch tabs | F1-F6 present |
| F8/u | Queue overview | **MISSING** |
| 1-7 | Switch tabs | 1-6 present |
| q | Quit | Present |
| ? | Help | Present |
| p | Pause/resume | **MISSING** |
| w | Wave overview | **MISSING** |
| i | Inject message | **MISSING** |
| y | Approve command | **MISSING** |
| n | Reject command | **MISSING** |

#### Dashboard (F1) — focus-aware:
| Key | Action | Roko |
|-----|--------|------|
| j/k/Up/Down | Scroll focused panel | Present (plan tree only) |
| Tab | Cycle focus | Present |
| Shift-Tab | Cycle focus backward | Present |
| a/o/d/e/g | Sub-tab switch | Present |
| m | Context/MCP sub-tab | **MISSING** |
| P | Processes sub-tab | **MISSING** |
| h/l/Left/Right | Collapse/expand or switch detail tab | **MISSING** (roko uses for page switching) |
| Enter | Plan detail (on Plans) or task detail (on Tasks) | Partial |
| Esc | Close plan detail | Present |
| End/Space | Resume auto-scroll in agent output | **MISSING** |
| PageUp/PageDown | Focus-aware page scroll | Partial (not focus-aware) |
| v | Toggle verification pane | **MISSING** |
| Alt+1-7 | Switch agent role tabs | **MISSING** |
| backtick | Cycle agent tabs | **MISSING** |

#### Plans (F2):
| Key | Action | Roko |
|-----|--------|------|
| j/k | Navigate | Present |
| Left/Right | Wave navigation | **MISSING** |
| Enter | Drill into plan | **MISSING** |
| Esc | Drill out | **MISSING** |
| / | Filter mode | **MISSING** |
| m | Prepare merge | **MISSING** |
| M | Merge selected plan | **MISSING** |
| s | Soft retry | **MISSING** |
| z | Diagnose | **MISSING** |
| S | Repair preserve | **MISSING** |
| R | Repair clean | **MISSING** |
| c | Reverify | **MISSING** |

#### Agents (F3):
| Key | Action | Roko |
|-----|--------|------|
| j/k | Navigate | Present |
| Tab/Shift-Tab | Panel cycling | Present |
| End | Scroll to end | **MISSING** |
| backtick | Cycle agent tabs | **MISSING** |
| [/] | Cycle process output tabs | **MISSING** |

#### Git (F4 — entirely missing tab in roko):
| Key | Action |
|-----|--------|
| j/k | Navigate |
| Enter | Drill into branch |

#### Inject mode (entirely missing in roko):
| Key | Action |
|-----|--------|
| Esc | Cancel |
| Enter | Submit message |
| Backspace | Delete char |
| chars | Type message |

#### Filter mode (entirely missing in roko):
| Key | Action |
|-----|--------|
| Esc | Cancel |
| Enter | Apply filter |
| Backspace | Delete char |
| chars | Type filter |

#### Confirm mode (entirely missing in roko):
| Key | Action |
|-----|--------|
| y/Enter | Confirm |
| n/Esc | Cancel |

#### Mouse (entirely missing in roko):
| Event | Action |
|-------|--------|
| Click | Route to widget via hit_test |
| ScrollUp | Scroll focused panel up |
| ScrollDown | Scroll focused panel down |

---

## 5. Data Flows / TuiState Gaps

### Mori RunState fields vs Roko TuiState:

| Mori field | Purpose | Roko equivalent | Status |
|------------|---------|-----------------|--------|
| `agents: HashMap<AgentRole, AgentState>` | Per-role agent with full output, diff, tokens, render cache | `agents: Vec<AgentEntry>` with only `last_output_line` | **THIN** |
| `parallel_agents: Vec<ParallelAgentState>` | Per-instance parallel output, cost, model | — | **MISSING** |
| `token_burn_history` | Per-role cumulative token history for sparklines | `token_history` | Present but never populated |
| `agent_efficiency` | Per-instance efficiency data | — | **MISSING** |
| `pending_approval: Option<PendingApproval>` | Queued approval from orchestrator | — | **MISSING** |
| `pending_confirm: Option<ConfirmAction>` | Confirm dialog state | — | **MISSING** |
| `notifications: Vec<Notification>` | Toast stack | — | **MISSING** |
| `input_mode: InputMode` | Normal/Inject/Filter/Confirm state machine | — | **MISSING** |
| `message_input: String` | In-progress inject text | — | **MISSING** |
| `filter_text: String` | Plan filter query | — | **MISSING** |
| `git_branch_tree` | Branch hierarchy for Git tab | — | **MISSING** |
| `git_commit_graph` | Rendered commit graph | — | **MISSING** |
| `git_worktree_list` | Active worktrees | — | **MISSING** |
| `conductor_history` | Watcher intervention log | — | **MISSING** |
| `branch_diff: String` | Current branch diff text | — | **MISSING** |
| `mcp: McpRuntimeState` | MCP server health + tool call counts | — | **MISSING** |
| `active_fixtures` | Running test fixtures | — | **MISSING** |
| `queue_source_label` | Queue milestone source label | — | **MISSING** |
| `queue_overview_selected` | Cursor for queue overview | — | **MISSING** |
| `pipeline_header_selected` | Pipeline header row selection | — | **MISSING** |
| `agent_scroll: Option<usize>` | Pinned vs auto-scroll | `output_scroll: usize` | **THIN** — no pinned/auto-tail |
| `plan_summary_content` | AI-generated plan summary | — | **MISSING** |
| `plan_detail_tab` | Summary vs Details tab | — | **MISSING** |
| `parallel_run: bool` | Whether run is parallel mode | — | **MISSING** |
| `steer_target` | Target agent for inject | — | **MISSING** |
| `render_cache` | Per-agent render cache | — | **MISSING** |
| `agent_list_cursor` | Selection cursor in agent list | — | **MISSING** |
| `show_*: bool` (7 flags) | Modal visibility flags | `overlay: Option<OverlayState>` (2 variants) | **THIN** |

### Critical data flow gap:
Roko's orchestrator does NOT stream agent output into TuiState. The DashboardSnapshot channel carries plan/task state transitions but NOT agent output text. Mori's orchestrator pushes full agent output into RunState in real-time. **This is the #1 blocker.**

---

## 6. Interactive Features

| Feature | Mori | Roko | Gap |
|---------|------|------|-----|
| Scroll acceleration | ScrollAccel — hold-key 1x→8x with 300ms reset | 1 line per keypress | **MISSING** |
| Agent output auto-tail | None=auto-tail, Some(n)=pinned; End/Space resume | Always absolute offset | **MISSING** |
| Render cache | CachedRender — parse once per output change | Re-render every frame | **MISSING** |
| ANSI segment parsing | parse_segments/group_segments/render_groups | Raw text rendering | **MISSING** |
| Filter mode | / → InputMode::Filter → live plan tree filtering | — | **MISSING** |
| Inject mode | i → InputMode::Inject → send text to active agent | — | **MISSING** |
| Approval flow | y/n + approval modal + pending_approval state | — | **MISSING** |
| Confirm dialog | All destructive ops gated by RequestConfirm | — | **MISSING** |
| Wave navigation | h/l on Plans tab | — | **MISSING** |
| Drill in/out | Enter/Esc hierarchical navigation | — | **MISSING** |
| Task picker | Ctrl-t scrollable task list | — | **MISSING** |
| Queue overview | F8 milestone progress browser | — | **MISSING** |
| Wave overview | w popup | — | **MISSING** |
| Agent pool modal | Full agent roster | — | **MISSING** |
| Verification toggle | v switches impl vs verify tabs | — | **MISSING** |
| Parallel agent tabs | Dynamic per-plan tabs | — | **MISSING** |
| Pause/resume | p toggles pipeline | — | **MISSING** |
| Mouse support | EnableMouseCapture + hit_test.rs | — | **MISSING** |
| Toast notifications | Bottom-right stacked toasts | — | **MISSING** |
| Conductor alerts | Alert banner for interventions | — | **MISSING** |
| Plan operations | s/z/S/R/c retry/repair/diagnose/reverify | — | **MISSING** |
| Merge operations | m/M batch-to-main merge | — | **MISSING** |
| Detail tab cycling | Tab switches Summary/PlanDetails | — | **MISSING** |
| Responsive padding | 1-cell outer margin on large terminals | — | **MISSING** |

---

## 7. Layout & Rendering

| Feature | Mori | Roko | Gap |
|---------|------|------|-----|
| Responsive outer margin | 1-cell margin when >=120w x >=50h | Always full area | **MISSING** |
| Alert banner row | Dynamic 0-1 line between header and content | — | **MISSING** |
| Panel shadows | postfx::drop_shadow() on each panel | — | **MISSING** |
| Background visualization | Plasma/noise cellular bg driven by task progress | — | **MISSING** |
| Atmospheric bloom | Atmosphere::apply() post-processing bloom | tick() only | **MISSING** |
| Dim overlay | postfx::dim_overlay() at 0.45 before modals | Clear widget only | **MISSING** |
| Drop shadow on modals | All 13 modals call drop_shadow() | — | **MISSING** |
| VFX module | vfx.rs — plasma, noise, field generators | — | **MISSING** |
| nerv_viz | Real-time state vis in agent output backgrounds | — | **MISSING** |
| PostFX pipeline | postfx_pipeline.rs + effects_config.rs | postfx.rs is 23-line stub | **MISSING** |
| hit_test module | 340-line coordinate-to-widget routing | — | **MISSING** |
| Plans right panel | Two-column: wave list + plan detail | Single-column flat list | **MISSING** |
| Pipeline header row | Selectable row above wave list | — | **MISSING** |

---

## 8. Views — Detailed Feature Comparison

### F1: Dashboard

| Feature | Mori | Roko |
|---------|------|------|
| Left: plan tree | 37K LOC with filter overlay, expand/collapse, rich task rows | Present, simpler |
| Left: phase compact | Present | Present |
| Left: task progress | 23K LOC with all status types | Present, simpler |
| Right: Agents sub-tab | agent_pool + agent_output + bottom strip | Present |
| Right: Output sub-tab | agent_output with ANSI segments | Present (raw text) |
| Right: Diff sub-tab | diff_panel with per-role tabs | Stub (gate output fallback) |
| Right: Errors sub-tab | error_digest | Present |
| Right: Git sub-tab | Git info panel | Present (minimal) |
| Right: Context sub-tab | MCP status + token sparklines | **MISSING** |
| Right: Processes sub-tab | Process table + fixture health | **MISSING** |
| Wave progress ribbon | 1-line ribbon | Present |

### F2: Plans

| Feature | Mori | Roko |
|---------|------|------|
| Left: wave browser | Hierarchical wave→plan list, expand/collapse | Flat/wave list (simpler) |
| Right: plan detail | Full detail with task list, phase timeline | **MISSING** — single column |
| Pipeline header | Selectable overview row | **MISSING** |
| Wave navigation (h/l) | Move between waves | **MISSING** |
| Drill in/out (Enter/Esc) | Hierarchical navigation | **MISSING** |
| Filter (/) | Live filtering | **MISSING** |
| Plan ops (s/z/S/R/c/m/M) | Retry, repair, diagnose, merge | **MISSING** |

### F3: Agents

| Feature | Mori | Roko |
|---------|------|------|
| Layout | Two-panel: roster + output | Two-panel (agent_pool + agent_output) |
| Parallel mode | ParallelAgentState with per-plan tabs | **MISSING** |
| Output parsing | ANSI segment parsing → styled blocks | Raw text |
| Agent tabs | Per-role tabs, backtick/Alt+N cycling | **MISSING** |
| Scroll pinning | Auto-tail + manual pin | **MISSING** |

### F4: Git — entirely missing in roko

| Feature | Mori |
|---------|------|
| Branch tree | branch_tree widget with hierarchy + connectors |
| Worktree list | Active worktrees panel |
| Commit graph | Rendered commit graph |
| Branch info | Branch + remote tracking |
| Diff panel | Full diff viewer |

### F5: Logs

| Feature | Mori | Roko |
|---------|------|------|
| Scrollable log tail | Full scrollable log | Present |
| Log levels | Colored by level | Present |
| Live streaming | Real-time log stream | Static disk refresh |

### F6: Config

| Feature | Mori | Roko |
|---------|------|------|
| Live config editor | Interactive key-value editor | Read-only stats |
| Config editing | Modify values in-place | **MISSING** |

### F7: Inspect — entirely missing in roko

| Feature | Mori |
|---------|------|
| MCP summary | Server health + tool call counts |
| Token burn sparklines | Per-agent cumulative token burn |
| Server panel | Connected MCP servers |
| Index panel | Code index status |
| Tool panel | Tool usage stats |
| Monitors sub-tab | Conductor watchers + intervention history |

---

## 9. PostFX & Visual Effects

| Component | File | Description | Roko |
|-----------|------|-------------|------|
| DimOverlay | postfx.rs | Screen dimming at 0.45 before modals | Exists but never called |
| drop_shadow() | postfx.rs | 1-cell shadow on panels | **MISSING** |
| postfx_pipeline | postfx_pipeline.rs | Configurable per-tab pipeline | **MISSING** |
| effects_config | effects_config.rs | Per-tab intensity settings | **MISSING** |
| vfx | vfx.rs | Plasma, noise, field generators | **MISSING** |
| nerv_viz | nerv_viz.rs | State visualization in agent output | **MISSING** |
| Atmosphere::apply() | atmosphere.rs | Full-frame bloom post-processing | **MISSING** (tick() only) |

---

## 10. Architecture / Infrastructure

| Component | Mori | Roko | Gap |
|-----------|------|------|-----|
| Input handler | tui/input.rs — per-tab/per-mode/per-focus, ~2K LOC | Flat handle_key() in app.rs, ~40 lines | **MASSIVE** |
| Hit testing | tui/hit_test.rs — coordinate routing, 340 LOC | — | **MISSING** |
| RunState | state/mod.rs — 87K LOC shared state | TuiState — ~630 LOC | **MASSIVE** |
| Agent output streaming | Orchestrator pushes live output via channels | DashboardSnapshot has no agent output | **CRITICAL** |
| Event bus | TuiAction enum with 50+ variants | Event enum with 3 variants | **MASSIVE** |
| Render caching | Per-agent CachedRender | Re-render everything every frame | **MISSING** |
| Scroll state | ScrollAccel with hold-key acceleration | Fixed 1-line scroll | **MISSING** |

---

## 11. Priority Tiers

### Tier 1 — Required for useful live dashboard:
1. Agent output streaming into DashboardSnapshot/TuiState
2. InputMode state machine (Normal/Inject/Filter/Confirm)
3. Full per-tab/per-focus key handler (port from Mori input.rs)
4. Plans view right-panel plan detail
5. Approval modal (needed for interactive runs)

### Tier 2 — Required for full interactive operation:
6. Confirm modal (gates destructive ops)
7. Inject modal (send messages to agents)
8. Plan operations (retry/repair/diagnose/reverify/merge)
9. Pause/resume (p key)
10. Filter mode (/ key)
11. Toast notifications
12. Agent output ANSI parsing (styled blocks)
13. Auto-tail / scroll pinning (End/Space)

### Tier 3 — Feature completeness:
14. Git tab (branch_tree + diff_panel + git_view)
15. Inspect tab (context view + MCP state + monitors)
16. Parallel agent mode (ParallelAgentState + parallel_pool)
17. Queue overview modal (F8)
18. Wave overview modal (w key)
19. Task picker modal (Ctrl-t)
20. Agent pool modal
21. Batch review modal

### Tier 4 — Polish:
22. PostFX dim_overlay for modals
23. PostFX drop_shadow for panels/modals
24. Responsive layout padding
25. Alert banner row
26. Scroll acceleration
27. Render caching
28. Mouse support
29. VFX / nerv_viz / bloom
30. Config editing (live)

---

## 12. Mori TuiAction Enum — All 76 Variants

From `tui/input.rs`:

```
Quit, SwitchTab(usize), SelectPlanUp, SelectPlanDown, ScrollLogUp, ScrollLogDown,
SwitchAgentTab(usize), ApproveCommand, ApproveAll, RejectCommand, StartInject,
SubmitInject(String), CancelInject, InputChar(char), InputBackspace, ShowHelp,
FocusNext, FocusPrev, ScrollFocusedUp, ScrollFocusedDown, ExpandCollapse,
ShowPlanDetail, ClosePlanDetail, ScrollDetailUp, ScrollDetailDown,
ScrollDetailPageUp, ScrollDetailPageDown, ScrollAgentUp, ScrollAgentDown,
ScrollAgentEnd, ScrollDiffUp, ScrollDiffDown, RestartPhase, RestartPlan,
SwitchDetailTab, ToggleAgentPaneGroup, DismissNotification, ConfigUp, ConfigDown,
ConfigLeft, ConfigRight, ConfigSelect, ForceAdvance, ResetPlanState, ReverifyPlan,
RequestConfirm(ConfirmAction), ConfirmYes, ConfirmNo, TogglePause,
VerifyTabNext, VerifyTabPrev, ProcOutputTabNext, ProcOutputTabPrev,
ShowWaveOverview, ShowAgentPoolModal, ShowQueueOverview, QueueOverviewNext,
QueueOverviewPrev, SwitchDetailSubTab(usize), StartFilter, AcceptFilter,
CancelFilter, CollapseExpand, ShowTaskDetail, CloseTaskDetail,
ScrollTaskDetailUp, ScrollTaskDetailDown, OpenTaskPicker, CloseTaskPicker,
TaskPickerUp, TaskPickerDown, TaskPickerConfirm, PrepareMergeBatchToMain,
MergeSelectedPlan, MergeAllDonePlans, NavigateUp, NavigateDown,
NavigatePageUp, NavigatePageDown, WaveNext, WavePrev, DrillIn, DrillOut,
MouseClick { x, y }, MouseScrollUp { x, y }, MouseScrollDown { x, y }, None
```

Roko's `TuiAction` enum has 17 variants (all dead code — defined but never dispatched).

---

## 13. Mori InputMode + ConfirmAction

### InputMode (4 variants):
```rust
Normal    // standard key dispatch
Inject    // text input: typing message to inject into agent
Filter    // text input: fuzzy filter for plan tree
Confirm   // waiting for y/n on destructive action
```

### ConfirmAction (14 variants):
```rust
RestartAllPlans
RestartPhase
ResetSelectedPlan(plan_base)
ForceAdvance(plan_base)
ReverifyPlan(plan_base)
DiagnosePlan(plan_base)
RepairPlanPreserve(plan_base)     // repair keeping completed work
RepairPlanClean(plan_base)        // full clean slate repair
SoftRetryPlan(plan_base)          // diagnose first, retry keeping completed
GitReconcile
IngestTask { plan_num, task_id }
MergeBatchToMain { batch_branch, plan_count, failed_count, last_commit }
MergePlan { plan_base, feasibility: MergeFeasibility }
MergeAllDone { plan_count, plan_names: Vec<(String, MergeFeasibility)> }
```

---

## 14. Mori RunState — All 120+ Fields

From `state/mod.rs` lines 937-1227:

### Core orchestrator:
- `orchestrator_state: String`, `plans: Vec<RunPlanEntry>`, `current_plan_idx`, `current_iteration`, `current_phase`

### Per-role agents (sequential):
- `agents: HashMap<AgentRole, AgentState>` — output (String, full turn), diff, input/output_tokens, active, thread_id, turn_count, current_plan/task, last_reported_cost_usd, render_cache: RefCell<CachedRender>

### Parallel agents:
- `parallel_agents: Vec<ParallelAgentState>` — instance_id, role, plan, task, output (512B tail), tool_output (8KB), tokens, cost, active, spawned_at, finished_at, model, turn_started, render_cache

### TUI navigation:
- `active_tab`, `selected_plan_idx`, `selected_agent_tab`, `focus: FocusZone`

### Input:
- `input_mode: InputMode`, `message_input`, `filter_text`, `filter_active`

### Scroll positions (8):
- `agent_scroll: Option<usize>` (None=auto-tail), `diff_scroll: Option<usize>`, `task_scroll`, `command_output_scroll: Option<usize>`, `plan_detail_scroll`, `plan_summary_scroll`, `plan_scroll_offset`, `log_scroll`, `task_detail_scroll`

### Modal visibility (7 bools):
- `show_plan_detail`, `show_help`, `show_wave_overview`, `show_agent_pool_modal`, `show_queue_overview`, `show_task_detail`, `show_task_picker`

### Approval / confirm:
- `pending_approval: Option<PendingApproval>` (role, command, approval_id)
- `pending_confirm: Option<ConfirmAction>`

### Git (10 fields):
- `git_branch`, `git_last_commit_secs`, `git_view_mode`, `git_branch_tree: Vec<GitTreeNode>`, `git_commit_graph: Vec<String>`, `git_worktree_list: Vec<WorktreeEntry>`, `git_safety: GitSafetyReport`, `git_reconcile_in_progress`, `git_branch_cursor`

### Notifications:
- `notifications: Vec<Notification>` (message, created: Instant, ttl_secs, level: LogLevel)
- `log_messages: Vec<LogEntry>` (timestamp, source, message, level)

### Plan detail:
- `plan_detail_content`, `plan_detail_tab: PlanDetailTab`, `plan_summary_content`

### Conductor:
- `conductor_history: Vec<ConductorHistoryEntry>`, `last_periodic_conductor_consult`, `steer_target`, `pending_agent_kills`

### Pipeline control:
- `pipeline_run_state: PipelineRunState`, `conductor_reset_brief`, `pending_phase_validation`

### Parallel mode plan tracking (15+ fields):
- `plan_task_cache`, `plan_verify_task_cache`, `plan_start_times`, `plan_gate_outputs`, `plan_active_task_ids`, `plan_phase_started`, `plan_pending_reviews`, `plan_review_stage`, `plan_doc_revisions`, `plan_agent_retries`, `pipeline_header_selected`, `executor_completed_tasks`, `plan_status_map`, `plan_recovery_cache`, `executor_state_summary`, `task_started_at`

### Cost/tokens:
- `cumulative_cost_usd`, `cost_per_plan`, `cost_per_task`, `cumulative_input_tokens`, `cumulative_output_tokens`

### MCP / Learning:
- `mcp: McpRuntimeState`, `learning: LearningRuntimeState`, `active_fixtures: Vec<FixtureRuntimeState>`

### Queue:
- `queue_source_label`, `queue_skipped_specs`, `plan_milestones`, `milestone_info: Vec<MilestoneProgress>`, `queue_overview_selected`

### VFX/animation:
- `terminal_height`, `particle_burst_pending`, `scroll_accel: ScrollAccel`, `smooth: SmoothedValues`, `token_prev`, `token_flash`

### Misc:
- `context_limit`, `branch_diff`, `command_output`, `verify_entries`, `selected_verify_idx`, `agent_pane_group`, `proc_output_tab`, `gate_progress_tx`, `tasks_checklist_dirty`, `instance_tool_calls`, `instance_write_calls`, `instance_turn_errors`, `committing_in_progress`, `main_merges`

---

## 15. Mori PostFX — Exact Functions

From `tui/postfx.rs`:

```rust
pub fn bloom(area: Rect, buf: &mut Buffer, threshold: u8, radius: u16, intensity: f64)
pub fn vignette(area: Rect, buf: &mut Buffer, intensity: f64)
pub fn dim_overlay(area: Rect, buf: &mut Buffer, factor: f64)
pub fn modal_glow(area: Rect, buf: &mut Buffer, full_area: Rect, glow_color: Color, intensity: f64)
pub fn ambient_orbs(area: Rect, buf: &mut Buffer, elapsed: f64, count: usize, brightness: f64)
pub fn dream_atmosphere(area: Rect, buf: &mut Buffer, elapsed: f64, frame_seed: u64)
pub fn amber_color_grade(area: Rect, buf: &mut Buffer, intensity: f64)
pub fn ambient_fill(area: Rect, buf: &mut Buffer, elapsed: f64)  // no-op
pub fn drop_shadow(buf: &mut Buffer, area: Rect)  // 1-cell right+bottom shadow
```

From `tui/postfx_pipeline.rs`:
```rust
pub fn apply_pipeline(tab: &Tab, area: Rect, buf: &mut Buffer, elapsed: f64, frame: u64, fx: &EffectsConfig)
fn self_glow(area: Rect, buf: &mut Buffer, threshold: u16, intensity: f64)
```
Pipeline: applies `self_glow(threshold=200, intensity=0.12)` on Dashboard/Agents/Plans tabs only, gated by `fx.screen_postfx`.

---

## 16. Mori Render Loop Details

From `app/parallel.rs`:

- Tick interval: 16ms (~60fps), `MissedTickBehavior::Skip`
- Message throttle: `MAX_MESSAGES_PER_TICK = 20` — prevents agent output streams from starving render
- Adaptive frame rate: when agents active + user idle >3s, only draws every 3rd frame (~20fps)
- Every tick: `atmosphere.tick_with_degraded()`, update sys metrics, smooth values, token histories
- After keypress: immediate draw (not waiting for next tick)
- Toast expiry: `notifications.retain(|n| n.created.elapsed() < ttl)`
- Conductor tick: every 60 frames (~1s)
- Agent output: 512-byte rolling tail per ParallelAgentState
- Tool output: 8KB rolling tail per instance
- `SmoothedValues` — EMA over cost/CPU/mem/tokens for fluid animation

Roko differences:
- Roko has 16ms tick ✓ but no message throttle, no adaptive frame rate, no immediate-after-keypress draw
- No SmoothedValues — values jump instead of smooth transitions
- No render cache — re-parses everything every frame
- No agent output streaming at all (standalone mode, no StateHub)

---

## 17. Roko Data Population Gaps (fields exist but never filled)

| TuiState field | Widget that reads it | Why it's empty |
|---------------|---------------------|----------------|
| `AgentEntry.input_tokens` | agent_pool context gauge | `update_from_snapshot()` only sets `active` and `role` |
| `AgentEntry.output_tokens` | agent_pool context gauge | Same |
| `AgentEntry.model` | agent_pool model tag | Same |
| `AgentEntry.current_task` | agent_pool task column | Same |
| `AgentEntry.last_output_line` | agent_output body | Same |
| `token_history` | token_sparkline | `push_token_sample()` exists but never called |
| `token_rate` | token_sparkline rate display | Never computed |
| `cost_dollars` | header_bar cost display | Never set |
| `PhaseStep.pct` | phase_compact pct display | `rebuild_phase_pipeline()` sets to 0.0 |
| `PhaseStep.elapsed_secs` | phase_compact elapsed display | Same |
| `TaskRow.elapsed_secs` | task_progress elapsed tag | `rebuild_task_checklist()` sets to 0.0 |
| `sys.net_down_bytes_sec` | sys_metrics NET display | `update_sys_metrics()` doesn't take net |
| `sys.disk_read_bytes_sec` | sys_metrics DSK display | Same |
| `filter` | plan_tree filter indicator | No `/` key handler |
| `task_scroll` | task_progress scroll | No key binding updates it |

---

## 18. Roko Dead Code (built but not called)

| File | What | Why dead |
|------|------|---------|
| `views/dashboard.rs` | Old master-detail view | `draw_new()` routes to `mori_dashboard`, not this |
| `widgets/agent_grid.rs` | Old AgentState grid | Takes raw AgentState, not TuiState |
| `widgets/token_bar.rs` | Old gauge widget | Takes raw AgentState, not TuiState |
| `widgets/phase_bar.rs` | Old horizontal phase bar | Takes raw phase string, not TuiState |
| `widgets/scrollbar.rs` | Old scrollbar wrapper | Legacy only |
| `widgets/mod.rs:render_dashboard` | 2656-line legacy dashboard | Only via `draw_legacy()` which is never triggered |
| `TuiAction` enum in `event.rs` | 17-variant action enum | Defined but never dispatched |
| `plan_detail` modal | Scrollable plan detail | Enter key doesn't open it on F1/F2 |
| `task_detail` modal | Task metadata viewer | Never triggered by any key |

---

## 19. Roko Key Handling Bugs

1. **q with overlay open quits immediately** — should close overlay first, quit on second q
2. **1-6 keys select legacy PageId slots** — out of sync with F1-F6 tab system
3. **h/l keys cycle pages** — on Dashboard tab should collapse/expand or switch detail sub-tab
4. **Enter does nothing on F1/F2** — plan_detail and task_detail modals exist but are never opened
5. **PageUp/PageDown not focus-aware** — scrolls legacy page offset instead of focused panel
6. **task_scroll never updated** — widget reads it but no key changes it
7. **output_scroll targeting wrong** — scroll keys on non-Dashboard tabs adjust legacy scroll, not output_scroll

---

## 20. Mori File Inventory (66 files)

Roko has ~46 files in its TUI. Mori has 66. Files Mori has that roko doesn't:

### Views (5 missing):
- `views/git_view.rs`, `views/context.rs`, `views/processes.rs`, `views/review.rs`, `views/monitors.rs`

### Modals (9 missing):
- `modals/approval.rs`, `modals/inject.rs`, `modals/confirm.rs`, `modals/notification.rs`
- `modals/wave_overview.rs`, `modals/queue_overview.rs`, `modals/agent_pool_modal.rs`
- `modals/task_picker.rs`, `modals/batch_review.rs`, `modals/quit.rs`

### Widgets (9 missing):
- `widgets/branch_tree.rs`, `widgets/context_gauge.rs`, `widgets/diff_panel.rs`
- `widgets/parallel_pool.rs`, `widgets/phase_timeline.rs`, `widgets/plan_list.rs`
- `widgets/status_badge.rs`, `widgets/tab_bar.rs`, `widgets/wave_bar.rs`

### Infrastructure (7 missing):
- `input.rs` (full key handler), `hit_test.rs` (mouse routing), `math.rs` (math helpers)
- `effects_config.rs`, `postfx_pipeline.rs`, `vfx.rs`, `nerv_viz.rs`
