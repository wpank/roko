# Complete TUI Keybinding Audit

**Date:** 2026-09-01
**Source files:**
- `crates/roko-cli/src/tui/input.rs` (dispatch logic, ~1100 lines)
- `crates/roko-cli/src/tui/app.rs` (action handling, ~3000+ lines)
- `crates/roko-cli/src/tui/modals/help.rs` (help overlay, 113 lines)
- `crates/roko-cli/src/tui/widgets/status_bar.rs` (footer hints, 397 lines)
- `crates/roko-cli/src/tui/tabs.rs` (tab enum + F-key mapping)
- `crates/roko-cli/src/tui/hit_test.rs` (mouse click zones)

**Architecture:** Input flows through `input::handle_key()` (input.rs:481) which
returns a `TuiAction` enum variant. App::dispatch_action() (app.rs:1130) executes
it. Priority chain: Ctrl-C > modal intercept > confirm dialog > text input modes
> global keys > per-tab keys.

**Input modes:** Normal, Inject, Filter, Confirm, ConfigEdit, LogSearch, PlanFilter
(input.rs:18-34).

---

## 1. Complete Keybinding Reference Table

### 1.1 Always-Active (Highest Priority)

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Ctrl-C | Force quit (no confirm) | `QuitConfirmed` | input.rs:488 | YES | Bypasses ALL modes/modals. Immediate exit. |

### 1.2 Global Keys (All Tabs, Normal Mode)

These fire from `handle_global_key()` (input.rs:701) BEFORE per-tab dispatch.

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| F1 | Switch to Dashboard | `SwitchTab(Dashboard)` | tabs.rs:94 | YES | |
| F2 | Switch to Plans | `SwitchTab(Plans)` | tabs.rs:95 | YES | |
| F3 | Switch to Agents | `SwitchTab(Agents)` | tabs.rs:96 | YES | |
| F4 | Switch to Git | `SwitchTab(Git)` | tabs.rs:97 | YES | |
| F5 | Switch to Logs | `SwitchTab(Logs)` | tabs.rs:98 | YES | |
| F6 | Switch to Config | `SwitchTab(Config)` | tabs.rs:99 | YES | |
| F7 | Switch to Inspect | `SwitchTab(Inspect)` | tabs.rs:100 | YES | |
| F8 | Switch to Marketplace / Queue overview | `SwitchTab(Marketplace)` then overridden to `ShowQueueOverview` | tabs.rs:101, input.rs:771 | CONFLICT | See Section 3 |
| F9 | Switch to Atelier | `SwitchTab(Atelier)` | tabs.rs:102 | YES | |
| F10 | Switch to Learning | `SwitchTab(Learning)` | tabs.rs:103 | YES | |
| 1-9, 0 | Switch tabs (same as F1-F9, F10) | `SwitchTab(...)` | input.rs:712-728 | PARTIAL | Suppressed on Agents (1-7 used for role tabs) and Logs (1-4 used for filter levels). See Section 3 |
| Alt+1-9 | Switch sub-view within tab | `SwitchSubView(idx)` | input.rs:732-737 | YES | Tab-specific sub-views |
| q | Quit (opens confirm modal) | `Quit` | input.rs:740 | YES | |
| ? | Toggle help modal | `ShowHelp` | input.rs:741 | YES | |
| n | Dismiss notification | `DismissNotification` | input.rs:742 | CONFLICT | Overridden by Logs tab (NextLogMatch) and Marketplace (SwitchSubView(2)). See Section 3 |
| v | Verify/re-verify plan | `ReverifyPlan` | input.rs:765 | YES | Global, works on any tab |
| u | Queue overview | `ShowQueueOverview` | input.rs:771 | YES | Also F8 |
| Tab | Cycle focus forward | `FocusNext` | input.rs:772 | YES | |
| Shift+Tab | Cycle focus backward | `FocusPrev` | input.rs:773 | YES | |
| Ctrl-r | Refresh data | `Refresh` | input.rs:743-745 | YES | |
| Ctrl-a | Approve all pending | `ApproveAll` | input.rs:747-749 | YES | |
| Ctrl-t | Toggle agent topology | `ToggleAgentTopology` | input.rs:751 | YES | Switches to Agents tab if not there |
| Ctrl-x | Force advance (confirm) | `ForceAdvance` | input.rs:753-755 | YES | |
| Ctrl-d | Reset selected plan (confirm) | `ResetPlanState` | input.rs:757-759 | YES | |
| Ctrl-e | Toggle screen postfx | `ToggleScreenPostFx` | input.rs:761-763 | YES | |
| Ctrl-g | Git reconcile (confirm) | `RequestConfirm(GitReconcile)` | input.rs:767-769 | YES | |

### 1.3 Dashboard Tab (F1) -- handle_dashboard_key() input.rs:792

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Scroll up (focus-aware) | `SelectPlanUp` / `ScrollAgentUp` / `ScrollFocusedUp` | input.rs:795-798 | YES | |
| Down / j | Scroll down (focus-aware) | `SelectPlanDown` / `ScrollAgentDown` / `ScrollFocusedDown` | input.rs:800-803 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:805 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:806 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:807 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:808 | YES | |
| Enter | Show plan detail modal | `ShowPlanDetail` | input.rs:811 | YES | |
| Esc | Close plan detail | `ClosePlanDetail` | input.rs:812 | YES | |
| Shift+Left | Previous wave | `WavePrev` | input.rs:813 | YES | |
| Shift+Right | Next wave | `WaveNext` | input.rs:814 | YES | |
| Left / h | Drill out (collapse) | `DrillOut` | input.rs:815 | YES | |
| Right / l | Drill in (expand) | `DrillIn` | input.rs:816 | YES | |
| a | Sub-tab: Agents | `SwitchDetailTab(0)` | input.rs:819 | YES | |
| o | Sub-tab: Output | `SwitchDetailTab(1)` | input.rs:820 | YES | |
| d | Sub-tab: Diff | `SwitchDetailTab(2)` | input.rs:821 | YES | |
| e | Sub-tab: Errors | `SwitchDetailTab(3)` | input.rs:822 | YES | |
| g | Sub-tab: Git | `SwitchDetailTab(4)` | input.rs:823 | YES | |
| m | Sub-tab: MCP | `SwitchDetailTab(5)` | input.rs:824 | YES | |
| L | Sub-tab: Learning | `SwitchDetailTab(6)` | input.rs:825 | YES | |
| P | Sub-tab: Processes | `SwitchDetailTab(7)` | input.rs:826 | YES | |
| w | Wave overview modal | `ShowWaveOverview` | input.rs:829 | YES | |
| p | Toggle pause | `TogglePause` | input.rs:830 | YES | |
| i | Start inject message | `StartInject` | input.rs:831 | YES | |
| y | Approve command | `ApproveCommand` | input.rs:832 | YES | |
| ` (backtick) | Cycle agent role tabs | `SwitchAgentTab(MAX)` | input.rs:834 | YES | |

### 1.4 Plans Tab (F2) -- handle_plans_key() input.rs:840

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Select plan up | `SelectPlanUp` / `ScrollFocusedUp` | input.rs:842-844 | YES | Focus-aware |
| Down / j | Select plan down | `SelectPlanDown` / `ScrollFocusedDown` | input.rs:846-848 | YES | Focus-aware |
| Enter | Show plan detail | `ShowPlanDetail` | input.rs:850 | YES | |
| Esc | Close plan detail | `ClosePlanDetail` | input.rs:851 | YES | |
| e | Expand/collapse plan | `ExpandCollapse` | input.rs:852 | YES | |
| w | Wave overview modal | `ShowWaveOverview` | input.rs:853 | YES | |
| o | Queue overview | `ShowQueueOverview` | input.rs:854 | YES | |
| t | Open task picker | `OpenTaskPicker` | input.rs:855 | YES | |
| [ | Previous wave | `WavePrev` | input.rs:856 | YES | |
| ] | Next wave | `WaveNext` | input.rs:857 | YES | |
| Shift+Left | Previous wave | `WavePrev` | input.rs:858 | YES | |
| Shift+Right | Next wave | `WaveNext` | input.rs:859 | YES | |
| Left / h | Drill out | `DrillOut` | input.rs:860 | YES | |
| Right / l | Drill in | `DrillIn` | input.rs:861 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:862 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:863 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:864 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:865 | YES | |
| / | Start plan tree filter | `StartPlanFilter` | input.rs:868 | YES | |
| d | Diagnose plan (confirm) | `RequestConfirm(DiagnosePlan)` | input.rs:871 | YES | |
| m | Merge plan (confirm) | `RequestConfirm(MergePlan)` | input.rs:872-875 | YES | |
| M | Merge all done (confirm) | `RequestConfirm(MergeAllDone)` | input.rs:876-878 | YES | |
| s | Soft retry failed tasks | `SoftRetry` | input.rs:881 | YES | |
| z | Diagnose selected (modal) | `DiagnoseSelected` | input.rs:882 | YES | |
| S | Repair with context | `RepairWithContext` | input.rs:883 | YES | |
| R | Reset plan (confirm) | `RequestConfirm(ResetSelectedPlan)` | input.rs:884-886 | YES | |
| c | Reverify gates only | `ReverifyGatesOnly` | input.rs:887 | YES | |
| F | Force advance | `ForceAdvance` | input.rs:888 | YES | |
| V | Reverify plan | `ReverifyPlan` | input.rs:889 | YES | |

### 1.5 Agents Tab (F3) -- handle_agents_key() input.rs:894

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Navigation (focus-aware) | `ScrollAgentUp` / `ScrollDiffUp` / `SelectPlanUp` | input.rs:897-900 | YES | |
| Down / j | Navigation (focus-aware) | `ScrollAgentDown` / `ScrollDiffDown` / `SelectPlanDown` | input.rs:902-905 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:907 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:908 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:909 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:910 | YES | |
| G | Resume auto-scroll | `ScrollAgentEnd` | input.rs:911 | YES | |
| ` (backtick) | Cycle agent role tabs | `SwitchAgentTab(MAX)` | input.rs:914 | YES | |
| 1-7 | Switch agent role tab | `SwitchAgentTab(0-6)` | input.rs:915-921 | YES | Overrides global 1-7 tab switch |
| a | Approve command | `ApproveCommand` | input.rs:924 | YES | |
| A | Approve all | `ApproveAll` | input.rs:925 | YES | |
| x | Reject command | `RejectCommand` | input.rs:926 | YES | |
| i | Start inject | `StartInject` | input.rs:927 | YES | |
| g | Toggle agent pane grouping | `ToggleAgentPaneGroup` | input.rs:928 | YES | |
| t | Toggle agent topology | `ToggleAgentTopology` | input.rs:929 | YES | |

### 1.6 Git Tab (F4) -- handle_git_key() input.rs:934

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:936 | YES | |
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:937 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:938 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:939 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:940 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:941 | YES | |
| Left / h | Drill out | `DrillOut` | input.rs:942 | YES | Moves git branch cursor down |
| Right / l | Drill in | `DrillIn` | input.rs:943 | YES | Moves git branch cursor up |
| Enter | Expand/collapse | `ExpandCollapse` | input.rs:944 | YES | |

### 1.7 Logs Tab (F5) -- handle_logs_key() input.rs:949

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Scroll log up | `ScrollLogUp` | input.rs:951 | YES | |
| Down / j | Scroll log down | `ScrollLogDown` | input.rs:952 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:953 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:954 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:955 | YES | |
| End / G | Resume auto-tail | `ScrollLogEnd` | input.rs:956-957 | YES | |
| 1 | Toggle Info level | `ToggleLogFilter(Info)` | input.rs:958 | YES | Overrides global tab switch |
| 2 | Toggle Warn level | `ToggleLogFilter(Warn)` | input.rs:959 | YES | Overrides global tab switch |
| 3 | Toggle Error level | `ToggleLogFilter(Error)` | input.rs:960 | YES | Overrides global tab switch |
| 4 | Toggle Debug level | `ToggleLogFilter(Debug)` | input.rs:961 | YES | Overrides global tab switch |
| a | Show all log levels | `ShowAllLogFilters` | input.rs:962 | YES | |
| / | Start log search | `StartLogSearch` | input.rs:964 | YES | |
| n | Next search match | `NextLogMatch` | input.rs:965 | YES | Overrides global DismissNotification |
| N | Previous search match | `PrevLogMatch` | input.rs:966 | YES | |
| f | Toggle filter mode | `ToggleLogFilterMode` | input.rs:967 | YES | |

### 1.8 Config Tab (F6) -- handle_config_key() input.rs:972

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Config cursor up | `ConfigUp` | input.rs:978 | YES | Skips header rows |
| Down / j | Config cursor down | `ConfigDown` | input.rs:979 | YES | Skips header rows |
| Left / h | Cycle value left | `ConfigCycleLeft` | input.rs:980 | YES | |
| Right / l | Cycle value right | `ConfigCycleRight` | input.rs:981 | YES | |
| Enter / Space | Toggle / edit value | `ConfigToggle` | input.rs:982 | YES | Bool: toggles; Enum: cycles; String: opens edit |
| Ctrl-S | Save config changes | `ConfigSave` | input.rs:974-976 | YES | |

### 1.9 Inspect Tab (F7) -- handle_inspect_key() input.rs:998

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:1000 | YES | |
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:1001 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:1002 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:1003 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:1004 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:1005 | YES | |
| Left / h | Drill out | `DrillOut` | input.rs:1006 | YES | |
| Right / l | Drill in | `DrillIn` | input.rs:1007 | YES | |
| Enter | Expand/collapse | `ExpandCollapse` | input.rs:1008 | YES | |

### 1.10 Marketplace Tab (F8) -- handle_marketplace_key() input.rs:1013

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:1019 | YES | |
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:1020 | YES | |
| Enter | Expand/collapse | `ExpandCollapse` | input.rs:1021 | YES | |
| n | New job (CreateJob sub-view) | `SwitchSubView(2)` | input.rs:1022 | YES | Overrides global DismissNotification |
| r | Refresh | `Refresh` | input.rs:1023 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:1024 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:1025 | YES | |
| Ctrl-S | Submit job form | `SubmitJob` | input.rs:1015-1016 | YES | |

### 1.11 Atelier Tab (F9) -- handle_atelier_key() input.rs:1030

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:1032 | YES | |
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:1033 | YES | |
| Enter | Expand/collapse | `ExpandCollapse` | input.rs:1034 | YES | |
| r | Refresh | `Refresh` | input.rs:1035 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:1036 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:1037 | YES | |

### 1.12 Learning Tab (F10) -- handle_learning_key() input.rs:1042

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:1044 | YES | |
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:1045 | YES | |
| r | Refresh | `Refresh` | input.rs:1046 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:1047 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:1048 | YES | |

### 1.13 Modal: Help (?) -- handle_help_key() input.rs:566

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc / ? / q | Close help | `ShowHelp` (toggles off) | input.rs:568 | YES | |
| Up / k | Scroll up | `ScrollFocusedUp` | input.rs:569 | YES | |
| Down / j | Scroll down | `ScrollFocusedDown` | input.rs:570 | YES | |
| PgUp | Page scroll up | `ScrollPageUp` | input.rs:571 | YES | |
| PgDn | Page scroll down | `ScrollPageDown` | input.rs:572 | YES | |
| Home | Jump to top | `ScrollFocusedHome` | input.rs:573 | YES | |
| End | Jump to bottom | `ScrollFocusedEnd` | input.rs:574 | YES | |

### 1.14 Modal: Approval -- handle_approval_key() input.rs:579

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| y / Y / Enter | Approve | `ApproveCommand` | input.rs:581 | YES | |
| n / N / Esc | Reject | `RejectCommand` | input.rs:582 | YES | |
| Ctrl-a / A | Approve all | `ApproveAll` | input.rs:583-586 | YES | |

### 1.15 Modal: Wave Overview -- handle_wave_overview_key() input.rs:591

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc / w | Close | `ShowWaveOverview` (toggles) | input.rs:593 | YES | |
| Up / k | Scroll up | `ModalScrollUp` | input.rs:594 | YES | |
| Down / j | Scroll down | `ModalScrollDown` | input.rs:595 | YES | |

### 1.16 Modal: Plan Detail -- handle_plan_detail_key() input.rs:600

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc | Close | `ClosePlanDetail` | input.rs:602 | YES | |
| Up / k | Scroll up | `ScrollDetailUp` | input.rs:603 | YES | |
| Down / j | Scroll down | `ScrollDetailDown` | input.rs:604 | YES | |

### 1.17 Modal: Task Picker -- handle_task_picker_key() input.rs:609

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc | Close | `CloseTaskPicker` | input.rs:611 | YES | |
| Enter | Show task detail | `ShowTaskDetail` | input.rs:612 | YES | |
| Up / k | Cursor up | `TaskPickerUp` | input.rs:613 | YES | |
| Down / j | Cursor down | `TaskPickerDown` | input.rs:614 | YES | |

### 1.18 Modal: Task Detail -- handle_task_detail_key() input.rs:619

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc / q | Close | `CloseTaskDetail` | input.rs:621 | YES | |
| Up / k | Scroll up | `ScrollDetailUp` | input.rs:622 | YES | |
| Down / j | Scroll down | `ScrollDetailDown` | input.rs:623 | YES | |
| Tab | Switch detail sub-tab | `SwitchDetailTab(0)` | input.rs:624 | PARTIAL | Always passes 0; no cycling |

### 1.19 Modal: Queue Overview -- handle_queue_overview_key() input.rs:629

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc / q | Close | `ShowQueueOverview` (toggles) | input.rs:631 | YES | |
| Up / k | Cursor up | `QueueOverviewUp` | input.rs:632 | YES | |
| Down / j | Cursor down | `QueueOverviewDown` | input.rs:633 | YES | |

### 1.20 Modal: Agent Pool -- handle_agent_pool_key() input.rs:638

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Esc / q | Close | `CloseModal` | input.rs:640 | YES | |
| Up / k | Scroll up | `ModalScrollUp` | input.rs:641 | YES | |
| Down / j | Scroll down | `ModalScrollDown` | input.rs:642 | YES | |

### 1.21 Modal: Confirm Dialog -- handle_confirm_key() input.rs:647

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| y / Y / Enter | Confirm yes | `ConfirmYes` | input.rs:649 | YES | |
| n / N / Esc | Confirm no (cancel) | `ConfirmNo` | input.rs:650 | YES | |

### 1.22 Modal: Quit Confirm

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| y / Y / Enter | Quit confirmed | `ConfirmYes` -> `QuitConfirmed` | app.rs:1726-1729 | YES | Via Confirm mode |
| n / N / Esc | Cancel quit | `ConfirmNo` -> dismiss | app.rs:1780-1783 | YES | |

### 1.23 Input Mode: Inject -- handle_inject_key() input.rs:655

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Enter | Submit message | `SubmitInject` | input.rs:657 | YES | Writes to engrams.jsonl |
| Esc | Cancel | `CancelInject` | input.rs:658 | YES | |
| Backspace | Delete char | `InputBackspace` | input.rs:659 | YES | |
| Any char | Append char | `InputChar(c)` | input.rs:660 | YES | |

### 1.24 Input Mode: Filter -- handle_filter_key() input.rs:665

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Enter | Accept filter | `AcceptFilter` | input.rs:667 | YES | |
| Esc | Cancel filter | `CancelFilter` | input.rs:668 | YES | |
| Backspace | Delete char | `InputBackspace` | input.rs:669 | YES | |
| Any char | Append char | `InputChar(c)` | input.rs:670 | YES | |

### 1.25 Input Mode: Log Search -- handle_log_search_key() input.rs:676

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Enter | Accept search | `AcceptLogSearch` | input.rs:678 | YES | |
| Esc | Cancel search | `CancelLogSearch` | input.rs:679 | YES | |
| Backspace | Delete char | `InputBackspace` | input.rs:680 | YES | |
| Any char | Append char | `InputChar(c)` | input.rs:681 | YES | |

### 1.26 Input Mode: Plan Filter -- handle_plan_filter_key() input.rs:687

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Enter | Accept filter | `AcceptPlanFilter` | input.rs:689 | YES | |
| Esc | Cancel filter | `CancelPlanFilter` | input.rs:690 | YES | |
| Backspace | Delete char | `InputBackspace` | input.rs:691 | YES | |
| Any char | Append char | `InputChar(c)` | input.rs:692 | YES | |

### 1.27 Input Mode: Config Edit -- handle_config_edit_key() input.rs:988

| Key | Action | TuiAction | Source Line | Works? | Notes |
|-----|--------|-----------|-------------|--------|-------|
| Enter | Commit edit | `ConfigCommitEdit` | input.rs:990 | YES | |
| Esc | Cancel edit | `ConfigCancelEdit` | input.rs:991 | YES | |
| Backspace | Delete char | `InputBackspace` | input.rs:992 | YES | |
| Any char | Append char | `InputChar(c)` | input.rs:993 | YES | |

---

## 2. Dead Keybindings

These are keys bound in code that have no visible effect or produce no user-facing result.

| Key | Context | Issue | Source |
|-----|---------|-------|--------|
| `p` on Dashboard | `TogglePause` | Sets `is_paused` flag, shows "PAUSED" in status bar, but does NOT actually pause the runner pipeline. The flag is purely cosmetic -- it controls the heartbeat indicator style (status_bar.rs:67-75) but the runner event loop ignores it entirely. | input.rs:830, app.rs:1558-1560 |
| `p` on Plans | Same `TogglePause` | Listed in help as "pause/resume pipeline" -- misleading since it has no pipeline effect. | Same as above |
| `y` on Dashboard | `ApproveCommand` | Attempts approval but only works if there is an active Approval modal (app.rs:1590-1593). On Dashboard in Normal mode with no approval pending, this does nothing. | input.rs:832 |
| `Enter` on Agents | Falls through to `TuiAction::None` (not handled) | No expand/collapse behavior on Agents; only scrolling and approval. Enter key is consumed by the per-tab handler before reaching global. | input.rs:930 (catch-all) |
| `ExpandCollapse` on Git | Triggers `ExpandCollapse` but dispatch in app.rs (1549-1557) only operates on plans. Git entries have no `expanded` field. | input.rs:944, app.rs:1549-1557 |
| `ExpandCollapse` on Inspect | Same issue: dispatch only toggles plan expand. Inspect has no expandable nodes. | input.rs:1008, app.rs:1549-1557 |
| `ExpandCollapse` on Marketplace | Same as above. | input.rs:1021, app.rs:1549-1557 |
| `ExpandCollapse` on Atelier | Same as above. | input.rs:1034, app.rs:1549-1557 |
| `DrillIn` / `DrillOut` on Inspect | Dispatches but app.rs:1812 is a no-op for Inspect. | input.rs:1006-1007, app.rs:1812 |
| `Tab` in Task Detail | Always passes `SwitchDetailTab(0)` -- does not cycle through tabs. Pressing Tab repeatedly does nothing after first press. | input.rs:624 |
| `r` on Config (status bar hint) | Status bar shows "r:reload" but Config tab handler (input.rs:972-984) does NOT bind `r`. It falls through to global, which does not handle `r` either (global `Ctrl-r` is Refresh). The hint is a lie. | status_bar.rs:243, input.rs:972-984 |

---

## 3. Conflicting Keybindings

These are keys that do different things depending on context in potentially confusing ways.

| Key | Context A | Action A | Context B | Action B | Severity | Notes |
|-----|-----------|----------|-----------|----------|----------|-------|
| **F8** | Global (input.rs:771) | `ShowQueueOverview` | Tab::from_key (tabs.rs:101) | `SwitchTab(Marketplace)` | HIGH | `Tab::from_key` fires first in `handle_global_key()` (input.rs:703), so F8 always switches to Marketplace tab. The F8 -> ShowQueueOverview on line 771 is dead code -- it can never be reached because `Tab::from_key` already matched F8 on line 703. Help overlay says F8 is Marketplace (help.rs:39), but `u` is the real queue overview key. |
| **n** | Global (input.rs:742) | `DismissNotification` | Logs tab (input.rs:965) | `NextLogMatch` | MEDIUM | On Logs tab, per-tab handler fires first (input.rs:534), so `n` means "next match" not "dismiss notification". User cannot dismiss notifications while on Logs tab. |
| **n** | Global (input.rs:742) | `DismissNotification` | Marketplace (input.rs:1022) | `SwitchSubView(2)` | MEDIUM | On Marketplace, `n` means "new job" not "dismiss notification". |
| **1-7** | Most tabs | `SwitchTab` | Agents (input.rs:915-921) | `SwitchAgentTab(n)` | LOW | Intentional: number keys switch agent role tabs on F3. Global handler suppresses tab switching for Agents and Logs (input.rs:711). Well documented in help. |
| **1-4** | Most tabs | `SwitchTab` | Logs (input.rs:958-961) | `ToggleLogFilter` | LOW | Intentional: number keys toggle log levels on F5. Same suppression. |
| **d** | Dashboard | `SwitchDetailTab(2)` (Diff) | Plans (input.rs:871) | `RequestConfirm(DiagnosePlan)` | MEDIUM | Same key does completely different things on adjacent tabs. `d` is benign on Dashboard (sub-tab switch) but destructive on Plans (triggers confirm dialog). |
| **m** | Dashboard | `SwitchDetailTab(5)` (MCP) | Plans (input.rs:872) | `RequestConfirm(MergePlan)` | MEDIUM | Same key does completely different things on adjacent tabs. |
| **g** | Dashboard | `SwitchDetailTab(4)` (Git) | Agents (input.rs:928) | `ToggleAgentPaneGroup` | LOW | Different actions but both are non-destructive. |
| **e** | Dashboard | `SwitchDetailTab(3)` (Errors) | Plans (input.rs:852) | `ExpandCollapse` | LOW | Non-destructive, but unexpected. |
| **t** | Plans (input.rs:855) | `OpenTaskPicker` | Agents (input.rs:929) | `ToggleAgentTopology` | LOW | Same key on adjacent tabs, but both are reasonable. |
| **v** | Global (input.rs:765) | `ReverifyPlan` | Any tab | Always fires | MEDIUM | `v` is a global key that triggers a confirm dialog on any tab. This is unexpected on Config, Logs, Inspect, etc. where verification is irrelevant. |
| **a** | Dashboard (input.rs:819) | `SwitchDetailTab(0)` | Agents (input.rs:924) | `ApproveCommand` | MEDIUM | On Dashboard `a` switches sub-tab; on Agents `a` approves a command. Both override the global `DismissNotification` for `n`. |
| **s** | Plans (input.rs:881) | `SoftRetry` | No other tab | -- | LOW | But status bar on Plans shows `s:skip` (status_bar.rs:192) when the actual action is SoftRetry, not skip. Label mismatch. |

---

## 4. Missing Keybindings

Actions that have no keyboard shortcut but clearly should.

| Missing Feature | Expected Key | Tab(s) | Notes |
|-----------------|-------------|--------|-------|
| **No PgUp/PgDn in plan detail modal** | PgUp/PgDn | Plan Detail modal | Plan detail can get very long. Only Up/Down (1 line) works. The modal handler (input.rs:600-606) lacks PgUp/PgDn. |
| **No PgUp/PgDn in task detail modal** | PgUp/PgDn | Task Detail modal | Same issue. Only Up/Down works. |
| **No PgUp/PgDn in queue overview** | PgUp/PgDn | Queue Overview modal | Only Up/Down for cursor movement. No page jump for large queues. |
| **No Home/End in most modals** | Home/End | Wave Overview, Plan Detail, Task Detail, Queue Overview, Agent Pool | Only Help modal has Home/End. All others lack jump-to-top/bottom. |
| **No search/filter in Agents tab** | / | Agents (F3) | Dashboard has sub-tabs, Plans has filter, Logs has search. Agents has no way to filter by agent name or search output. |
| **No search/filter in Git tab** | / | Git (F4) | Cannot filter branches or search commits. |
| **No search/filter in Marketplace** | / | Marketplace (F8) | Cannot filter jobs. |
| **No search/filter in Atelier** | / | Atelier (F9) | Cannot filter PRDs. |
| **No p (publish) on Atelier** | p | Atelier (F9) | Status bar shows "p:publish" (status_bar.rs:259) but there is no `p` binding in handle_atelier_key() (input.rs:1030-1039). |
| **No g (gen plan) on Atelier** | g | Atelier (F9) | Status bar shows "g:gen plan" (status_bar.rs:260) but there is no `g` binding in handle_atelier_key(). |
| **No c (chat) on Agents** | c | Agents (F3) | Status bar shows "c:chat" for active agents (status_bar.rs:212) but there is no `c` binding in handle_agents_key(). |
| **No S (start) on Agents** | S | Agents (F3) | Status bar shows "S:start" for failed/idle agents (status_bar.rs:216,220) but there is no `S` binding in handle_agents_key(). |
| **No x (stop) on Agents** | x | Agents (F3) | Status bar shows "x:stop" for active agents (status_bar.rs:211) but `x` in Agents means `RejectCommand`, not stop. Label mismatch. |
| **No d (details) on Agents** | d | Agents (F3) | Status bar shows "d:details" (status_bar.rs:213,217,221) but there is no `d` binding in handle_agents_key(). |
| **No Enter on Learning** | Enter | Learning (F10) | Status bar shows "Enter:details" (status_bar.rs:264) but handle_learning_key() has no Enter binding. |
| **No Enter on Marketplace detail** | Enter | Marketplace (F8) | Status bar shows "Enter:detail" (status_bar.rs:252) -- Enter exists but mapped to ExpandCollapse, which is a no-op (see dead bindings). |
| **Copy to clipboard** | Ctrl-C (non-quit context) or y | Any | No way to copy agent output, log lines, diffs, or any text. |
| **Scroll acceleration toggle** | None | All | ScrollAccel exists (app.rs:73) but there is no way to toggle or configure it at runtime. |
| **No CycleEffectsPreset key** | None | All | `TuiAction::CycleEffectsPreset` exists (input.rs:317) but no key is bound to it. Help says Ctrl-E toggles postfx, but cycling presets has no binding. |
| **No i (inject) on Plans** | i | Plans (F2) | Inject works on Dashboard and Agents but not Plans, even though Plans is where you most want to message a running agent. |
| **No w (wave overview) on Agents** | w | Agents (F3) | Wave overview only on Dashboard and Plans. |

---

## 5. Mori Comparison

Reference: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/input.rs`

| Feature | Mori | Roko | Delta |
|---------|------|------|-------|
| Tab count | 7 (F1-F7) + F8 queue | 10 (F1-F10) | Roko has 3 more tabs |
| Tab switching | F1-F7 + 1-7 | F1-F10 + 1-9,0 | Roko adds 8/9/0 |
| Focus zones | 4 (Plans, Tasks, AgentOutput, CommandOutput) | 17 (expanded for per-tab zones) | Roko is significantly more granular |
| TuiAction variants | ~80 | ~70 (different set) | Roko consolidated some, added recovery/search/filter |
| Plan operations | s/z/S/R/c/d/m/M present | Same set present | Parity achieved |
| Agent approval | a/A/x | a/A/x | Parity |
| Queue overview | F8 | u (F8 conflicts with Marketplace tab) | Roko has the F8 conflict; Mori used F8 cleanly |
| Detail sub-tabs | a/o/d/e/g/m/P (7) | a/o/d/e/g/m/L/P (8) | Roko adds L (Learning) |
| Config editing | Up/Down/Left/Right/Enter/Space | Same | Parity |
| Log search | Not present | / n N f (full search) | Roko improvement |
| Plan tree filter | Not present | / (on Plans tab) | Roko improvement |
| Recovery keys | s/z/S/R/c | s/z/S/R/c/F/V | Roko adds F (ForceAdvance) and V (ReverifyPlan) |
| Inject mode | i -> type -> Enter/Esc | Same | Parity |
| Config text edit | Not present | Enter on string field -> type -> Enter/Esc | Roko improvement |
| Agent pane grouping | Not present in Mori input.rs | g on Agents tab | Roko addition |
| Agent topology | Not present | Ctrl-T / t on Agents | Roko addition |
| Effects cycling | Present (presets) | Ctrl-E toggle only; CycleEffectsPreset has no key | Roko regression: preset cycling lost its keybinding |

Key Mori features NOT in roko:
- Mori's `PrepareMergeBatchToMain` had its own `m` key on Plans tab. Roko has `m` -> `MergePlan` but no batch-to-main variant.
- Mori had `MergeSelectedPlan` (`M` on Plans) and `MergeAllDonePlans` (separate). Roko has `M` -> `MergeAllDone` which is the latter.
- Mori's queue overview was cleanly on F8 with no tab conflict.

---

## 6. Discoverability Assessment

### How can users discover keybindings?

1. **Help overlay (`?`)**: Primary discovery mechanism. Always available via `?` key.
2. **Status bar hints**: Bottom bar shows 3-5 context-sensitive hints per tab.
3. **Modal titles**: Some modals include key hints in their title bar (e.g., help: "scroll: Up/Down/PgUp/PgDn").

### Discoverability Rating: **POOR-TO-MODERATE**

**Problems:**
- The help overlay lists only ~60 bindings out of ~150+ total. Missing entirely:
  - All Marketplace/Atelier/Learning/Inspect tab-specific keys
  - All modal-specific keys except Help scrolling
  - All sub-view switching (Alt+1-9)
  - All config edit mode keys
  - Recovery keybindings are listed but incomplete
  - CycleEffectsPreset (no key anyway)
- The status bar shows at most 5 hints per tab, so most keys are invisible.
- No tooltip system, no command palette, no key echo.
- No visual indicator for which keys are available in the current context.
- Sub-view switching (Alt+1-9) is not documented anywhere in-app.
- Config edit mode (typing a value after Enter on a string field) has no visible instructions.
- The LogSearch mode keys (n/N/f after /) are listed in help but easy to miss.

**What works:**
- The `?` key is universal and well-known.
- Status bar hints are context-sensitive and change per tab/selection state.
- Modal intercept means keys don't leak across contexts.

---

## 7. Help Overlay vs. Actual Bindings (help.rs:29-113)

### Accurate entries:

| Help text | Actual binding | Match? |
|-----------|---------------|--------|
| "F1-F10 switch tabs" | Yes, via tabs.rs:92-106 | CORRECT |
| "1-9 / 0 switch tabs" | Yes, input.rs:712-728 | CORRECT |
| "u queue overview" | Yes, input.rs:771 | CORRECT |
| "Tab cycle focus" | Yes, input.rs:772 | CORRECT |
| "Shift+Tab cycle focus backward" | Yes, input.rs:773 | CORRECT |
| "j/k Up/Dn scroll" | Yes, per-tab handlers | CORRECT |
| "PgUp/PgDn page scroll" | Yes, per-tab handlers | CORRECT |
| "Home/End jump" | Yes, per-tab handlers | CORRECT |
| "Enter expand/drill" | Partially correct | CORRECT for Dashboard/Plans |
| "Esc close overlay / drill out" | Yes | CORRECT |
| "q close overlay or quit" | Yes | CORRECT |
| "a/o/d/e/g/m agents/output/diff/errors/git/mcp" | Yes, input.rs:819-824 | CORRECT |
| "L Learning panel" | Yes, input.rs:825 | CORRECT |
| "P Processes panel" | Yes, input.rs:826 | CORRECT |
| "? toggle help" | Yes, input.rs:741 | CORRECT |
| "w wave overview" | Yes, input.rs:829 | CORRECT |
| "p pause/resume pipeline" | Cosmetic only; see dead bindings | MISLEADING |
| "i inject message" | Yes, input.rs:831, 927 | CORRECT |
| "/ search (Logs) / filter (Plans)" | Yes | CORRECT |
| "n / N next/prev match" | Yes, Logs only | CORRECT |
| "f toggle search filter mode" | Yes, Logs only | CORRECT |
| "Ctrl-t task picker / agent topology" | Yes, input.rs:751 | CORRECT |
| "Ctrl-e toggle screen postfx" | Yes, input.rs:761-763 | CORRECT |
| "v verify/re-verify" | Yes, input.rs:765 | CORRECT |
| "Ctrl-r refresh" | Yes, input.rs:743-745 | CORRECT |
| "Ctrl-a approve all" | Yes, input.rs:747-749 | CORRECT |
| "Ctrl-x force advance" | Yes, input.rs:753-755 | CORRECT |
| "Ctrl-d reset selected plan" | Yes, input.rs:757-759 | CORRECT |
| "Ctrl-g git reconcile" | Yes, input.rs:767-769 | CORRECT |

### Inaccurate or misleading entries:

| Help text | Issue |
|-----------|-------|
| "p pause/resume pipeline" | `TogglePause` only affects the cosmetic heartbeat indicator. The actual runner pipeline is unaffected. This is actively misleading. |
| "1-7 switch agent role tab (F3 only)" | Correct in behavior but the help says "1-7" while the actual binding is 1-7 (input.rs:915-921), not 1-9. If on F3 and you press 8 or 9, those DO switch top tabs (because Agents only suppresses 1-7 for role tabs, and global handler checks `Tab::Agents` exclusion only for digit-to-tab, not for 8/9). This edge case is not explained. |
| "`\`` cycle agent role tabs" | Correct. |
| "g toggle agent pane grouping" | Correct for Agents tab. |
| "t toggle agent topology" | Correct for Agents tab. |
| "G/End resume auto-scroll" | Correct for Agents tab. |
| "e expand/collapse plan" | Only on Plans tab. On Dashboard, `e` switches to Errors sub-tab. Help doesn't clarify. |
| "[/] wave prev/next" | Only on Plans tab. Not globally available. Correct. |
| "h/l Left/Right drill out/in" | On Plans. Correct. |
| "V / c re-verify plan / gates only" | Correct for Plans tab. But `V` is also globally available as `v` (lowercase). |

### Completely missing from help:

- All Marketplace-specific keys (n, r, Ctrl-S)
- All Atelier-specific keys (r)
- All Learning-specific keys (r)
- All Inspect-specific keys (h/l/Enter)
- Alt+1-9 sub-view switching
- Ctrl-S config save
- Config edit mode (Enter on string -> type -> Enter/Esc)
- Plan filter mode keys (/ on Plans)
- Mouse support description
- Wave navigation Shift+Left/Right
- Quit confirm modal keys (y/n)
- Any modal-specific scrolling (PgUp/PgDn in Help, etc.)

---

## 8. Mouse Support

### What works (app.rs:2862-2879):

| Mouse Action | Behavior | Source |
|-------------|----------|--------|
| Left click | Focus zone detection via hit_test | app.rs:2863-2864, dispatches `MouseClick{x,y}` |
| Scroll up | Scroll focused panel by 3 lines | app.rs:2868-2870, dispatches `MouseScrollUp` -> `scroll_focused(-3)` |
| Scroll down | Scroll focused panel by 3 lines | app.rs:2871-2874, dispatches `MouseScrollDown` -> `scroll_focused(3)` |

### Click hit testing (hit_test.rs:42-157):

| Zone | Clickable? | Effect |
|------|-----------|--------|
| Header tabs | YES | Switches focus to PlanTree (but does NOT switch tabs) |
| Detail tabs | YES | Switches focus to RightPanel (but does NOT switch detail tab) |
| Plan tree panel | YES | Sets focus to PlanTree |
| Task progress panel | YES | Sets focus to TaskProgress |
| Agent output panel | YES | Sets focus to AgentOutput |
| Command output panel | YES | Sets focus to CommandOutput |
| Right content panel | YES | Sets focus to RightPanel |

### Mouse issues:

| Issue | Severity | Details |
|-------|----------|---------|
| **Header tab clicks don't switch tabs** | HIGH | Clicking a tab header only changes focus to PlanTree (hit_test.rs mapping), not to the actual tab. The `HeaderTab(idx)` variant is received but mapped to `FocusZone::PlanTree` instead of dispatching `SwitchTab`. See app.rs:2047. |
| **Detail tab clicks don't switch detail tabs** | MEDIUM | Same issue: `DetailTab(idx)` maps to `FocusZone::RightPanel`, not `SwitchDetailTab(idx)`. See app.rs:2048. |
| **No mouse capture by default** | LOW | `capture_mouse` defaults to `false` (app.rs:656). Mouse events work via crossterm polling but terminal mouse capture (`EnableMouseCapture`) is not enabled. Some terminals may not report mouse events without capture. |
| **No click-to-select in lists** | MEDIUM | Clicking on a plan, agent, or task in a list does not select it. Only focus zone changes. |
| **No right-click context menus** | LOW | Expected for a TUI but worth noting. |
| **No drag support** | LOW | No panel resizing via mouse. |
| **Scroll delta fixed at 3** | LOW | Mouse scroll always scrolls 3 lines regardless of terminal scroll speed settings. |

---

## 9. Status Bar Accuracy

The status bar is rendered by `widgets/status_bar.rs` with context-sensitive hints
generated by `context_key_hints()` (status_bar.rs:163-276).

### Per-tab accuracy:

| Tab | Hints shown | Accuracy |
|-----|-------------|----------|
| **Dashboard** | `Up/Dn:nav a/o/d/e/g:sub-tab Tab:panel ?:help` | CORRECT. When failures present adds `R:retry D:diag` -- but `R` is not bound on Dashboard (it is on Plans); `D` is not bound at all (Ctrl-d is ResetPlanState). |
| **Dashboard (failures)** | adds `R:retry D:diag` | WRONG. `R` on Dashboard has no special binding. `D` (uppercase) has no binding anywhere. |
| **Plans** | `Up/Dn:nav Enter:expand h/l:drill /:filter ?:help` | CORRECT for default state. |
| **Plans (failed task)** | `Up/Dn:nav Enter:expand r:retry s:skip d:details` | WRONG: `r` is not bound on Plans tab (lowercase `r` is not in handle_plans_key). `s` maps to `SoftRetry` not "skip". `d` maps to `DiagnosePlan` not "details". Labels are misleading. |
| **Plans (active task)** | `Up/Dn:nav Enter:expand d:details` | PARTIALLY WRONG: `d` is `DiagnosePlan`, not "details". |
| **Agents** | `Up/Dn:nav` | CORRECT base. |
| **Agents (active)** | adds `x:stop c:chat d:details` | WRONG: `x` is `RejectCommand` not stop. `c` has no binding. `d` has no binding. |
| **Agents (failed/idle)** | adds `S:start d:details` | WRONG: `S` has no binding. `d` has no binding. |
| **Agents (no agent)** | adds `\`:cycle Ctrl+T:topology i:inject` | CORRECT. |
| **Git** | `Up/Dn:nav h/l:drill Enter:expand` | PARTIALLY CORRECT. Enter maps to ExpandCollapse which is a no-op for git (see dead bindings). |
| **Logs** | `Up/Dn/PgUp/PgDn:scroll 1-4:levels a:all /:search` | CORRECT. |
| **Config** | `j/k:nav Enter:toggle r:reload` | WRONG: `r:reload` has no binding on Config tab. |
| **Inspect** | `Up/Dn:nav Tab:panel Enter:details` | PARTIALLY WRONG: Tab is global focus cycle (correct), but Enter maps to ExpandCollapse which is a no-op for Inspect. |
| **Marketplace** | `j/k:nav Enter:detail n:new r:refresh` | CORRECT. |
| **Atelier** | `j/k:nav Enter:detail p:publish g:gen plan` | WRONG: `p` and `g` have no bindings on Atelier tab. |
| **Learning** | `Up/Dn:nav Enter:details` | WRONG: `Enter` has no binding on Learning tab. |

### Status bar accuracy summary:

- **Fully correct:** Logs, Marketplace, Plans (default state)
- **Partially wrong:** Dashboard (failure hints), Plans (failure/active hints), Git, Inspect, Agents (general)
- **Significantly wrong:** Agents (with selected agent), Config, Atelier, Learning

---

## 10. Summary of Issues by Severity

### Critical (user will be confused or misled):

1. **F8 conflict**: F8 is dead as queue overview; always switches to Marketplace. Help says F8 is Marketplace AND the code has F8 mapped to ShowQueueOverview, but the tab switch wins.
2. **Status bar lies about Agents keys**: `x:stop`, `c:chat`, `d:details`, `S:start` are all fictitious -- none of these bindings exist.
3. **Status bar lies about Atelier keys**: `p:publish` and `g:gen plan` do not exist.
4. **Status bar lies about Config key**: `r:reload` does not exist.
5. **`p` (pause) is cosmetic only**: Help and status bar say "pause/resume pipeline" but only the heartbeat animation changes.

### High:

6. **Mouse header-tab clicks don't switch tabs**: Users expect clicking a tab to switch to it.
7. **Status bar "R:retry" and "D:diag" on Dashboard failures**: Neither is bound on Dashboard.
8. **Plans tab status bar says "s:skip"** but `s` is `SoftRetry`.
9. **Status bar "Enter:details" on Learning**: No Enter binding exists.

### Medium:

10. **Global `n` (DismissNotification) unreachable on Logs, Marketplace**: Per-tab handlers shadow it.
11. **`d` does completely different things on Dashboard vs Plans**: Sub-tab switch vs diagnose confirm.
12. **`m` does completely different things on Dashboard vs Plans**: Sub-tab switch vs merge confirm.
13. **Global `v` (ReverifyPlan) fires on all tabs**: Unexpected on Config, Logs, etc.
14. **Task Detail Tab key always passes 0**: No cycling.
15. **ExpandCollapse is dead on Git, Inspect, Marketplace, Atelier**: Enter triggers it but dispatch is plan-only.
16. **Mouse detail-tab clicks don't switch detail tabs**.
17. **CycleEffectsPreset has no keybinding**: TuiAction exists but is unreachable.

### Low:

18. Various missing search/filter on Agents, Git, Marketplace, Atelier.
19. Missing PgUp/PgDn in most modals.
20. Missing Home/End in most modals.
21. No copy-to-clipboard support.
22. Fixed mouse scroll delta of 3.
