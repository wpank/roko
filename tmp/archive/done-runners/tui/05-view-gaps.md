# TUI View/Tab Gaps (Exhaustive Audit)

Updated 2026-04-14 by exhaustive code audit. Corrections to previous version noted inline.

## Summary

- 7 views dispatched from `views/mod.rs:render_tab_content()`: Dashboard, Plans, Agents, Git, Logs, Config, Inspect
- **CORRECTION**: Previous doc (06-dead-code.md) listed agents_view (X4), context_view (X5), logs_view (X6) as dead code. ALL 7 views ARE wired and called via `render_tab_content()` at `views/mod.rs`. None are dead.
- All views render visual structure but most display static/empty data because `TuiState` fields are initialized to defaults and only partially populated by background threads.

## F1: Dashboard (`views/dashboard_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V1 | **Sub-tab MCP (index 5) shows placeholder** | `dashboard_view.rs:~180` | HIGH | Sub-tab "MCP" exists in the tab bar but renders "MCP servers: 0" with no data. `ContextViewData.mcp_servers` always empty. |
| V2 | **Sub-tab Procs (index 6) shows placeholder** | `dashboard_view.rs:~190` | HIGH | Sub-tab "Procs" renders "No process data" -- no process supervisor integration. |
| V3 | **Agent output is raw text** | `dashboard_view.rs` | MEDIUM | Output sub-tab (index 1) renders `agent.output_lines` as plain text. No ANSI segment parsing, no styled blocks from `parse_segments/group_segments`. |
| V4 | **Agent output no auto-tail** | `dashboard_view.rs` | MEDIUM | Always absolute scroll offset. Mori has None=auto-tail, Some(n)=pinned, End/Space resumes. |
| V5 | **Agent output single view** | `dashboard_view.rs` | MEDIUM | Single output pane. Mori has dynamic per-plan tabs for parallel agent output. |
| V6 | **Agent output re-renders every frame** | `dashboard_view.rs` | LOW | No `CachedRender` -- parse/layout recalculated every 16ms tick. `AgentState.render_cache` field exists in `state.rs:48` but never used. |
| V7 | **Plan tree no filter overlay** | `dashboard_view.rs` | LOW | No `/` filter mode for plan tree. `filter_text` field exists in TuiState but not wired to plan tree rendering. |
| V8 | **Plan tree basic expand/collapse** | `dashboard_view.rs` | LOW | `PlanEntry.expanded` field exists but no collapse-all/expand-all keyboard shortcut. |
| V9 | **Wave progress ribbon minimal** | `dashboard_view.rs` | LOW | `wave_progress` widget renders but shows basic counts. No ETA, no wave number detail. |
| V10 | **Verification toggle missing** | `dashboard_view.rs` | LOW | No `v` key to switch between impl vs verify tabs. |

## F2: Plans (`views/plans_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V11 | **Single-column flat list** | `plans_view.rs` | HIGH | No right-panel plan detail. Mori has two-column: wave list + plan detail pane. Plans view renders only a flat list of plan entries. |
| V12 | **No pipeline header row** | `plans_view.rs` | HIGH | No selectable overview row above wave list. Missing entirely. |
| V13 | **No wave navigation** | `plans_view.rs` | MEDIUM | No `h`/`l` keys to move between waves. `selected_wave_idx` exists in TuiState (`state.rs:504`) but no key handler wires it. |
| V14 | **No drill in/out** | `plans_view.rs` | MEDIUM | No `Enter`/`Esc` hierarchical navigation. Enter toggles plan_detail boolean but plan_detail modal is orphaned (see M5). |
| V15 | **No filter** | `plans_view.rs` | LOW | No `/` live plan filtering in Plans tab. |
| V16 | **No plan operations** | `plans_view.rs` | MEDIUM | No `s`/`z`/`S`/`R`/`c` keys for retry, repair, diagnose, reverify. `ConfirmAction` enum has variants for these but no key bindings on Plans tab. |
| V17 | **No merge operations** | `plans_view.rs` | LOW | No `m`/`M` keys for prepare merge, merge selected/all. |
| V18 | **No plan detail tab cycling** | `plans_view.rs` | LOW | No Summary vs Details tabs in plan detail. `plan_detail_tab` field exists (`state.rs:460`) but plan_detail modal is orphaned. |

## F3: Agents (`views/agents_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V19 | **No parallel mode** | `agents_view.rs` | HIGH | `ParallelAgentState` struct exists (`state.rs:86-92`) and `parallel_agents` vec in TuiState (`state.rs:352`) but never populated. Agents view has no per-plan tabs. |
| V20 | **Raw text output** | `agents_view.rs` | MEDIUM | Agent output rendered as plain text lines. No ANSI segment parsing. |
| V21 | **Fixed 7 role tabs** | `agents_view.rs` | MEDIUM | Hard-coded role tabs (strategist, implementer, auditor, etc.). No dynamic per-role tabs, no backtick/Alt+N cycling. |
| V22 | **No scroll pinning** | `agents_view.rs` | MEDIUM | No auto-tail + manual pin. `agent_scroll` field exists (`state.rs:386`) with None=auto-tail semantics but rendering always uses absolute offset. |
| V23 | **No process output tabs** | `agents_view.rs` | LOW | No `[`/`]` keys to cycle through process outputs. |
| V24 | **Gradient bars and context gauges inline** | `agents_view.rs:~100-200` | INFO | Agent roster renders gradient bars with `lerp_rgb` and breathing effect from atmosphere -- this is WORKING and well-implemented. |

## F4: Git (`views/git_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V25 | **Worktree list renders but data depends on background thread** | `git_view.rs:~115-180` | MEDIUM | `collect_git_data()` runs `git worktree list` in background thread. Data populates `git_view_data`. Layout exists (branch tree + worktree list + status + commit graph + branch info) but relies on background thread timing. |
| V26 | **Diff panel shows "Diff not available"** | `git_view.rs` (via `diff_panel` widget) | MEDIUM | Diff panel widget exists but always shows placeholder. No git diff integration. |
| V27 | **Branch info minimal** | `git_view.rs:~200` | LOW | Shows branch name and tracking info but no remote detail, no ahead/behind visualization. |

## F5: Logs (`views/logs_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V28 | **Static disk refresh** | `logs_view.rs` | HIGH | Reads `.roko/` files (signals.jsonl, episodes.jsonl, efficiency.jsonl, gate failures) on 500ms background refresh. No real-time streaming from orchestrator. |
| V29 | **No log level filtering** | `logs_view.rs` | MEDIUM | Renders all log entries with level-based coloring but no interactive filter by level. |
| V30 | **BTreeMap sort is correct** | `logs_view.rs` | INFO | Uses BTreeMap timestamp keying for unified time-sorted view from 5 sources. This is working correctly. |

## F6: Config (`views/config_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V31 | **Config editing partially works** | `config_view.rs` | MEDIUM | Interactive editor with field navigation, inline editing, cycling, save button. ConfigEdit input mode exists. BUT: saved changes write to `roko.toml` but do NOT trigger runtime reload. |
| V32 | **No live reload** | `config_view.rs` | MEDIUM | Changes written to disk but `App` does not re-read config after save. Requires restart. |
| V33 | **Runtime sections read-only** | `config_view.rs` | LOW | Efficiency, cascade router, gate thresholds, experiments sections appended but not editable. |

## F7: Inspect (`views/context_view.rs`)

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| V34 | **Data mostly empty** | `context_view.rs` | HIGH | 4 sections rendered (health summary, token burn by role, cost by model, cascade router + alerts) but `ContextViewData` fields (mcp_servers, token_burns, index_entries, tool_usage) are never populated from runtime. Shows "0 servers", "0 burns", etc. |
| V35 | **No MCP sub-tab** | `context_view.rs` | MEDIUM | Renders as single scrollable view. Mori has `m` key for MCP status + token sparklines. |
| V36 | **No monitors sub-tab** | `context_view.rs` | LOW | Mori F7 has 3-column server/index/tool panel + monitors sub-tab. Roko has flat 4-section layout. |

## Missing tabs

| ID | Tab | Mori key | Severity | Detail |
|----|-----|----------|----------|--------|
| V37 | **Queue overlay** | F8 | LOW | Milestone progress browser. ModalState::QueueOverview exists but as modal, not tab. Opened with empty data. |
