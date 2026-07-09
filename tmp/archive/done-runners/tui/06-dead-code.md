# TUI Dead Code (Exhaustive Audit)

Updated 2026-04-14 by exhaustive code audit. Every `pub fn` in widgets/ checked for callers. Every file checked for imports. MAJOR corrections to previous version.

## CORRECTIONS to previous doc

The previous version (06-dead-code.md) had several INCORRECT entries:

| Previous ID | Claim | Actual status |
|-------------|-------|---------------|
| X4 | `views/agents_view.rs` "not called by any active tab handler" | **WRONG** -- Called from `views/mod.rs:render_tab_content()` for `Tab::Agents`. Fully wired. |
| X5 | `views/context_view.rs` "not called" | **WRONG** -- Called from `views/mod.rs:render_tab_content()` for `Tab::Inspect`. Fully wired. |
| X6 | `views/logs_view.rs` "only on legacy path" | **WRONG** -- Called from `views/mod.rs:render_tab_content()` for `Tab::Logs`. Fully wired. |
| X12 | `postfx.rs` "never applied to rendered frames" | **PARTIALLY WRONG** -- `dim_overlay()` IS called at `app.rs:435` when any modal or help is active. The rest (bloom, vignette, etc.) are dead because `EffectsConfig` defaults all to false. |

## Confirmed dead code: Modal files

| ID | File | What | File:Line | Why dead |
|----|------|------|-----------|----------|
| D1 | `modals/help.rs` | `render_help_modal()` | `help.rs:1-60` | Uses `crate::tui::mori_theme::MoriTheme` -- module path does NOT exist in `tui/mod.rs`. Import will fail. Never called from `app.rs` (which uses `render_help_overlay` + `help_lines()` at `app.rs:439`). |
| D2 | `modals/plan_detail.rs` | `render_plan_detail_modal()` | `plan_detail.rs:1-80` | No `ModalState::PlanDetail` variant. Not called from `app.rs` draw() or `render_modals()`. `show_plan_detail` boolean toggles but triggers no render. |
| D3 | `modals/task_detail.rs` | `render_task_detail_modal()` | `task_detail.rs:1-60` | No `ModalState::TaskDetail` variant. Not called from `app.rs` draw() or `render_modals()`. `show_task_detail` boolean intercepts keys (`input.rs:296`) but nothing renders. **Keys consumed invisibly.** |

## Confirmed dead code: Widget files (zero callers from views or app)

| ID | File | What | File:Line | Why dead |
|----|------|------|-----------|----------|
| D4 | `widgets/agent_grid.rs` | `render_agent_grid()` | `agent_grid.rs` | Takes raw `AgentState` (legacy struct), not `TuiState`. No callers in any view or app.rs. |
| D5 | `widgets/token_bar.rs` | `render_token_bar()` | `token_bar.rs` | Takes raw `AgentState`, not `TuiState`. No callers. |
| D6 | `widgets/phase_bar.rs` | `render_phase_bar()` | `phase_bar.rs` | Takes raw phase string, not `TuiState`. No callers. |
| D7 | `widgets/scrollbar.rs` | `render_scrollbar()` | `scrollbar.rs` | Legacy wrapper. No callers from active views (task_progress uses ratatui Scrollbar directly). |
| D8 | `widgets/wave_bar.rs` | `render_wave_bar()` | `wave_bar.rs:35` | Takes `WaveProgress` struct. Zero callers -- `wave_bar::` never appears as import or call. |
| D9 | `widgets/plan_list.rs` | `render_plan_list()` | `plan_list.rs:28` | Zero callers -- `plan_list::` never appears as import or call. plans_view.rs does its own inline list rendering. |
| D10 | `widgets/tab_bar.rs` | `render_tab_bar()` | `tab_bar.rs:22` | Zero callers -- `tab_bar::` never appears as import or call. Views render their own inline tab bars. |
| D11 | `widgets/status_badge.rs` | `render_status_badge()` | `status_badge.rs:76` | Zero callers -- `status_badge::` never appears as import or call. |
| D12 | `widgets/context_gauge.rs` | `render_context_gauge()` | `context_gauge.rs:14` | Zero callers -- `context_gauge::` never appears as import or call. agents_view.rs renders its own inline context gauge. |
| D13 | `widgets/phase_timeline.rs` | `render_phase_timeline()` | `phase_timeline.rs:23` | Zero callers -- `phase_timeline::` never appears as import or call. Dashboard uses phase_compact instead. |
| D14 | `widgets/agent_output.rs` | `render_agent_output()` | `agent_output.rs:33` | Zero callers -- `agent_output::` never appears as import or call. Views render output inline. |
| D15 | `widgets/command_output.rs` | `render_command_output()` | `command_output.rs:20` | Zero callers -- `command_output::` never appears as import or call. Dashboard renders command output inline. |
| D16 | `widgets/agent_pool.rs` | `render_agent_pool()` (widget version) | `agent_pool.rs:107` | Zero callers from views -- `widgets::agent_pool` never imported. Note: DIFFERENT from `modals/agent_pool_modal.rs` which IS called via ModalState dispatch. |

## Confirmed dead code: Infrastructure

| ID | File | What | File:Line | Why dead |
|----|------|------|-----------|----------|
| D17 | `scroll.rs` | `ScrollAccel` struct | `scroll.rs:10-54` | Exported from `mod.rs:37` but never used outside its own file. Zero callers in app.rs, input.rs, or any view. |
| D18 | `atmosphere.rs` | `Atmosphere::apply()` | `atmosphere.rs` | Full-frame bloom function. Defined but zero callers -- `.apply()` never appears outside atmosphere.rs. (Note: `breathing_brightness()`, `heartbeat()`, `spinner()`, `spinner_ethereal()` ARE used by widgets.) |
| D19 | `layout.rs` | `split_horizontal()` | `layout.rs:45` | Zero callers outside tests. Views use ratatui Layout directly. |
| D20 | `layout.rs` | `split_vertical()` | `layout.rs:58` | Zero callers outside tests. Views use ratatui Layout directly. |
| D21 | `postfx_pipeline.rs` | `apply_pipeline()` | `postfx_pipeline.rs` | Called from `app.rs:460` BUT only when `fx_config.screen_postfx` is true. `EffectsConfig::default()` sets `screen_postfx: false`. No code path ever sets it to true. Pipeline code is reachable but never reached. |
| D22 | `postfx.rs` | `bloom()`, `vignette()`, `modal_glow()`, `ambient_orbs()`, `dream_atmosphere()`, `amber_color_grade()`, `drop_shadow()` | `postfx.rs` | Only called from `postfx_pipeline.rs` which is dead per D21. (`dim_overlay()` IS alive -- called at `app.rs:435`.) |
| D23 | `effects_config.rs` | `EffectsConfig` | `effects_config.rs` | All fields default to false/0.0. No code path modifies them. Config view does not include effects settings. |
| D24 | `widgets/mod.rs` | `render_dashboard` (2656-line legacy function) | `widgets/mod.rs` | Only reachable via `draw_legacy()` path which is never triggered. All tab rendering goes through `views/mod.rs:render_tab_content()`. |

## Confirmed dead code: Duplicate definitions

| ID | File | What | File:Line | Why problematic |
|----|------|------|-----------|-----------------|
| D25 | `hit_test.rs` + `input.rs` | `FocusZone` enum defined TWICE | `hit_test.rs:7`, `input.rs:37` | Two separate `FocusZone` enums with slightly different variants. `input.rs` version used for key routing. `hit_test.rs` version used for mouse routing. App.rs at lines 1100-1108 manually converts between them. Should be unified. |
| D26 | `state.rs` | `AgentState` + `AgentRow` | `state.rs:32-53`, `state.rs:60-82` | Two separate agent representation structs. `AgentState` used by `agents_by_id` HashMap (legacy). `AgentRow` used by `agents` Vec (new). Both exist in TuiState. |
| D27 | `state.rs` | Duplicate plan index fields | `state.rs:324,358,360` | `current_plan_idx`, `selected_plan`, `selected_plan_idx` -- three separate usize fields tracking plan selection. |
| D28 | `state.rs` | Duplicate cost fields | `state.rs:472,486` | `cumulative_cost_usd` and `cost_dollars` -- both f64, both tracking cumulative cost. |
| D29 | `state.rs` | Duplicate scroll fields | `state.rs:386-388` | `agent_scroll: Option<usize>` (with None=auto-tail semantics) and `output_scroll: usize` -- both for agent output scroll. |

## Dead code by module count

| Module | Total pub fns | Called | Dead | Dead % |
|--------|---------------|--------|------|--------|
| modals/ | 14 render fns | 11 | 3 (help, plan_detail, task_detail) | 21% |
| widgets/ | 25+ render fns | ~12 | ~13 (agent_grid, token_bar, phase_bar, scrollbar, wave_bar, plan_list, tab_bar, status_badge, context_gauge, phase_timeline, agent_output, command_output, agent_pool) | 52% |
| infrastructure | 8 fns/structs | 3 | 5 (ScrollAccel, split_horizontal, split_vertical, Atmosphere::apply, render_dashboard) | 63% |
| postfx | 8 effect fns | 1 (dim_overlay) | 7 | 88% |
