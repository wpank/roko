# TUI Data Flow Gaps

Exhaustive audit of every data path between the orchestrator, disk files, background threads, and the TUI rendering pipeline. Every TuiState field traced from source to consumer.

**Audit date**: 2026-04-14
**Files audited**: `state.rs` (1366L), `dashboard.rs` (~5200L), `app.rs` (1899L), `orchestrate.rs`

---

## BLOCKING — Fields actively read by widgets but never populated

| ID | Gap | Where | Data Source | Consumer | Notes |
|----|-----|-------|-------------|----------|-------|
| D1 | `token_history` never populated — HashMap stays empty forever | `state.rs:492` declared, `:611` default `HashMap::new()`, never assigned in `update_from_snapshot()` or `drain_background_channels()` | Should come from efficiency events bucketed over time | `token_sparkline.rs:60,128,134,140` iterates over empty HashMap — sparkline always flat zero line | |
| D2 | `token_rate` never computed — always 0.0 | `state.rs:484` declared, `:607` default 0.0, never assigned anywhere | Should be computed from token_history deltas or efficiency event timestamps | `token_sparkline.rs:89-99,168` reads `state.token_rate` — displays "idle" forever | |
| D3 | `token_burn_history` never populated — HashMap stays empty forever | `state.rs:490` declared, `:610` default `HashMap::new()`, never assigned | Should aggregate `TokenBurnEntry` per-role from efficiency events | `TokenBurnEntry` struct at `state.rs:185-191` is dead code | |
| D4 | `gate_results` (Vec\<GateResultEntry\>) never bridged from DashboardData | `state.rs:344` declared, `:537` default empty Vec, never written in `update_from_snapshot()` | `DashboardData.gate_results` is `Vec<GateResultSummary>` (different type) — never bridged to `Vec<GateResultEntry>` | `command_output.rs:23,46` reads `state.gate_results` — renders empty, always "no output" | Type mismatch between two structs |
| D5 | `pending_approval` never set to `Some(...)` — only cleared | `state.rs:426` declared, `:578` default None. `app.rs:658,661,664` only set to `None` | Should come from orchestrator when agent requests shell/tool approval | Approval modal, `ApproveCommand`/`RejectCommand` handlers all dead code | |
| D6 | `log_messages` (Vec\<LogEntry\>) never populated | `state.rs:454` declared, `:592` default empty Vec, never written anywhere | `logs_view.rs` has its OWN `LogEntry` type and builds log directly from `DashboardData`, bypassing `TuiState.log_messages` entirely | Nothing reads `state.log_messages` — the field is dead weight | Type duplication |
| D7 | `parallel_agents` never populated | `state.rs:352` declared, `:541` default empty Vec, never written anywhere | Should come from executor state when multiple agents run in parallel | `parallel_pool.rs` renders parallel agent cards — always empty | |

## HIGH — Fields permanently stuck at defaults, never updated with real data

| ID | Gap | Where | Data Source | Consumer | Notes |
|----|-----|-------|-------------|----------|-------|
| D8 | `orchestrator_state` permanently "idle" | `state.rs:320` declared, `:528` default "idle", never reassigned | Should reflect real orchestrator state ("running"/"paused"/"error") from executor state JSON or DashboardEvent | `status_bar`, `header_bar` read for state indicator | |
| D9 | `current_iteration` permanently 0 | `state.rs:326` declared, `:530` default 0, never assigned | Should reflect task retry iteration from executor state | header_bar iteration counter always "0" | |
| D10 | `current_phase` permanently empty string | `state.rs:328` declared, `:532` default empty, never assigned | Should reflect current plan execution phase from active task status | header_bar, phase_compact show nothing for current phase | |
| D11 | `plan_detail_content` permanently empty string | `state.rs:458` declared, `:594` default empty, never assigned | Should be populated when plan detail overlay opens | Plan detail overlay body always blank | |
| D12 | `plan_summary_content` permanently empty string | `state.rs:462` declared, `:596` default empty, never assigned | Should contain rendered plan summary text | Plan summary view always blank | |
| D13 | `cost_per_plan` never populated | `state.rs:474` declared, `:602` default empty HashMap, never written | Should compute by bucketing efficiency events by plan_id | Cost breakdown panels expect it | |
| D14 | `cost_per_task` never populated | `state.rs:476` declared, `:603` default empty HashMap, never written | Should compute by bucketing efficiency events by task_id | Cost breakdown panels expect it | |
| D15 | `git_branch_tree` never populated | `state.rs:438` declared, `:584` default empty Vec. `GitBgData` carries `view_data` but never extracts into `GitBranchNode` format | Should be extracted from background git thread | `branch_tree.rs` renders empty tree | |
| D16 | `git_commit_graph` never populated | `state.rs:440` declared, `:585` default empty Vec, never written | Should come from git log data | Git tab commit graph empty | |
| D17 | `git_worktree_list` never populated | `state.rs:442` declared, `:586` default empty Vec, never written | Should come from `git worktree list` output | Git tab worktree section empty | |
| D18 | `TuiState.notifications` vs `App.notifications` — two separate notification systems | `state.rs:452` has `Vec<Notification>` with `level: NotificationLevel`. `App` has `Vec<modals::Notification>` with `kind: NotificationKind` | `App.notifications` IS populated (by inject/confirm/save). `TuiState.notifications` is NEVER populated | `TuiState.notifications` is dead; real toast system lives in `App.notifications` | Type mismatch |
| D19 | `PlanEntry.tasks` always empty Vec | `state.rs:754` always set to `Vec::new()` in `update_from_snapshot()` | Should be populated from `TasksFile` tasks for expanded plan tree | `plan_tree.rs` renders nested task entries — always empty when expanded | |
| D20 | `PlanEntry.tasks_failed` always 0 | `state.rs:748` always set to 0 in `update_from_snapshot()` | Should count from `TaskTrackerSnapshot.completed` failures | header_bar, plan_tree failure counts always "0" | |
| D21 | `PlanEntry.elapsed_secs` always 0.0 | `state.rs:749` always set to 0.0 | Should come from episode durations or wall_ms per plan | Plan tree timing column always "--" | |
| D22 | `PlanEntry.wave` always None | `state.rs:750` always set to `None` | Should come from plan metadata or executor wave assignment | `build_execution_waves()` falls to single-wave grouping | |

## MEDIUM — Data bridged but heuristic or incomplete

| ID | Gap | Where | Data Source | Consumer | Notes |
|----|-----|-------|-------------|----------|-------|
| D23 | Phase pipeline uses position-based heuristic, not real phase data | `state.rs:925-971` `build_phase_pipeline()` uses `len/3` as midpoint. Phases before=Done, midpoint=Active, rest=Pending | Should map actual task statuses and orchestrator phase to canonical names | `phase_compact.rs`, `phase_timeline.rs` render inaccurate progress | |
| D24 | `AgentRow.model` falls back to empty string | `state.rs:768` sets `model: String::new()` initially, only populated IF matching episode exists AND `episode.model` non-empty | Should come from agent spawn config or task tier | `agent_pool.rs` model column blank for agents without episodes | |
| D25 | `AgentRow.context_limit` hardcoded to 200,000 for all agents | `state.rs:771` | Should come from model configuration (Haiku/Opus/Sonnet limits differ) | `agent_pool.rs:149` context fill % always relative to 200K | |
| D26 | `AgentRow.input_tokens`/`output_tokens` only from last matching episode | `state.rs:810-811` uses `max()` across episodes matching agent_id | Misses tokens from efficiency events without matching episodes | Context gauge may undercount token usage | |
| D27 | `PlanEntry.phase` is binary "done"/"pending" | `state.rs:734-738` only checks `p.completed` boolean | Should use `current_phase_label()` from dashboard.rs which extracts real phase | plan_tree, status_bar never show "implementing"/"gating"/etc | |
| D28 | `PlanEntry.tasks_done` is all-or-nothing (0 or total) | `state.rs:739` sets `tasks_done = p.task_count` if completed, else 0 | Should count from `TaskTrackerSnapshot.completed.len()` for partial progress | Progress bars jump 0% to 100% with no intermediate | |
| D29 | `TaskRow.elapsed_secs` parsed via duration string with precision loss | `state.rs:1073,1106-1122` `parse_duration_to_secs()` parses "--"/"5s"/"2m 30s" | Original data is in milliseconds — string roundtrip loses precision | Minor rendering glitch | |
| D30 | `GitBgData.age` always empty string after initial load | `app.rs:305` sets `age: String::new()` in bg thread. `populate_git_info()` only sets on first empty | Subsequent bg thread updates never provide fresh age | Status bar git age goes stale | |
| D31 | `DashboardData.agents` depends on `assigned_agents` field in executor state | `dashboard.rs:1683-1714` `load_agents()` only finds agents if executor state has `plan_states.*.assigned_agents` | If orchestrator doesn't persist assigned_agents, agents vec empty during runs | Agent tab empty during active runs | |
| D32 | `DashboardData.event_log` expects `.roko/state/events.json` — never written by orchestrate.rs | `dashboard.rs:564-566` loads from events_path | orchestrate.rs writes signals to `.roko/signals.jsonl`, not events.json | Event log always empty | File path mismatch |

## LOW — Dead structures, duplicates, and cosmetic issues

| ID | Gap | Where | Notes |
|----|-----|-------|-------|
| D33 | `DashboardEvent::AgentOutput` defined but never consumed by TUI | `dashboard_snapshot.rs:49` variant, `orchestrate.rs:11534` conversion | TUI reads files on disk via polling, not event channels. DashboardEvent only used by serve/SSE path |
| D34 | `DashboardSnapshot` (legacy text renderer) duplicates data from `DashboardData` | `dashboard.rs:2691` | Two parallel data models load same files independently |
| D35 | `AgentState` (HashMap-based) populated but never read by any widget | `state.rs:31-53` struct, `:776-785` populated | Dead data structure — widgets only read `Vec<AgentRow>` |
| D36 | `state::Notification` vs `modals::Notification` — two incompatible notification types | `state.rs:138-151` vs modals module | Never interact; TuiState.notifications dead, App.notifications live |
| D37 | `state::LogEntry` vs `views::logs_view::LogEntry` — two incompatible log entry types | `state.rs:154-160` vs `logs_view.rs:25` (different fields, different `level` types) | logs_view builds its own from DashboardData, bypassing TuiState |
| D38 | `pipeline_run_state` toggled by TogglePause but never sent to orchestrator | `app.rs:637-643` toggles string "paused"/"running" | TUI-local only — no signal written to pause actual orchestrator |
| D39 | `parallel_run` never set to true | `state.rs:468` declared, `:599` default false, never written | Parallel execution indicator never lights up |
| D40 | `output_scroll` field never used (agent_scroll used instead) | `state.rs:388` declared, `:559` default 0, never read by any widget key handler modifies | Dead field |
| D41 | `selected_plan` and `selected_plan_idx` are duplicate fields | `state.rs:358` and `:360` | `selected_plan` never read by dispatch_action; `selected_plan_idx` is the real one |
| D42 | `plan_scroll` and `plan_scroll_offset` are duplicate scroll fields | `state.rs:400` and `:402` | `scroll_focused` writes `plan_scroll_offset`; `plan_tree.rs` reads `plan_scroll` — NEVER SYNCED |
| D43 | `show_agent_pool_modal` declared but never set to true | `state.rs:416` declared, `:573` default false, no TuiAction sets it | Agent pool modal can never be opened |
| D44 | `plan_summary_scroll` declared but never adjusted | `state.rs:398` declared, `:564` default 0, no key handler writes it | Plan summary view cannot be scrolled |

## Summary

| Severity | Count | Previous doc |
|----------|-------|--------------|
| BLOCKING | 7 | 3 |
| HIGH | 15 | 7 |
| MEDIUM | 10 | 6 |
| LOW | 12 | 7 |
| **Total** | **44** | **23** |
