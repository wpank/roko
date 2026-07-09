# TUI Rendering & Visual Effects Gaps (Exhaustive Audit)

Updated 2026-04-14 by exhaustive code audit. MAJOR corrections to previous version.

## CORRECTIONS to previous doc

The previous version had several INCORRECT claims:

| Previous ID | Claim | Actual status |
|-------------|-------|---------------|
| R1 | "Always full area" (no responsive margin) | **WRONG** -- `responsive_outer_margin()` IS called in `app.rs:draw()`. 1-cell margin when terminal >= 120w x 50h. Working. |
| R6 | "dim_overlay exists but never called" | **WRONG** -- `dim_overlay()` IS called at `app.rs:435` when any modal or help is active. Working. |
| R18 | "Waits for next 16ms tick" | **WRONG** -- Immediate draw after keypress IS implemented at `app.rs:337-343`. `last_input` timestamp updated on key event, triggering immediate redraw. |
| R19 | "Always 60fps" | **WRONG** -- Adaptive frame rate IS implemented at `app.rs:354-367`. When user idle >3s and no active agents, only draws every 3rd frame (~20fps). |
| R22 | "No toast system" | **WRONG** -- Notifications ARE populated (SubmitInject at `app.rs:670-700`, ConfirmYes, ConfigSave all push Notification entries). `render_notifications()` IS called at `modals/mod.rs:177`. |

## Section 1: Draw pipeline sequence (actual)

The draw pipeline in `app.rs:draw()` (lines 373-468) executes in this order:

```
1. responsive_outer_margin(full_area)     -- 1-cell margin when >= 120x50
2. Layout: header(3) + alert(0-1) + content(fill) + footer(2)
3. render_header_bar()                    -- top bar with tabs, agent count, cost
4. render_wave_row()                      -- alert/wave row (dynamic 0-1 line)
5. render_tab_content()                   -- dispatches to active tab's view
6. render_status_footer()                 -- bottom status line
7. dim_overlay(0.45)                      -- IF active_modal OR show_help
8. render_help_overlay()                  -- IF show_help (app.rs built-in, NOT help.rs)
9. render_modals()                        -- IF active_modal (ModalState dispatch)
10. render_overlay()                      -- IF legacy overlay active
11. apply_pipeline()                      -- IF fx_config.screen_postfx (NEVER true)
```

## Section 2: Layout gaps

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R1 | **Alert banner row** | `app.rs:~400` | MEDIUM | Dynamic 0-1 line between header and content. Space is reserved in layout but wave_row renders only basic wave progress, no conductor alert system. |
| R2 | **Plans right panel** | `plans_view.rs` | HIGH | Plans tab is single-column flat list. Mori has two-column: wave list + plan detail. |
| R3 | **Pipeline header row** | `plans_view.rs` | MEDIUM | No selectable overview row above wave list in Plans tab. |

## Section 3: PostFX gaps

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R4 | **PostFX pipeline unreachable** | `effects_config.rs`, `app.rs:458` | HIGH | `EffectsConfig::default()` sets `screen_postfx: false`. No code path sets it to true. No config key in roko.toml. No keyboard toggle. The entire postfx_pipeline.rs is unreachable. |
| R5 | **Panel drop shadows** | `postfx.rs:drop_shadow()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R6 | **Modal glow** | `postfx.rs:modal_glow()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R7 | **Bloom effect** | `postfx.rs:bloom()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R8 | **Vignette** | `postfx.rs:vignette()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R9 | **Ambient orbs** | `postfx.rs:ambient_orbs()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R10 | **Dream atmosphere** | `postfx.rs:dream_atmosphere()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R11 | **Amber color grade** | `postfx.rs:amber_color_grade()` | LOW | Function exists but only called from pipeline (dead per R4). |
| R12 | **Self-glow pipeline** | `postfx_pipeline.rs:self_glow()` | LOW | Function exists in pipeline but unreachable (dead per R4). |
| R13 | **Effects config not user-configurable** | `effects_config.rs` | MEDIUM | No roko.toml key, no config_view entry, no keyboard toggle. All effects permanently off. To fix: add `[tui.effects]` section to config, wire into App initialization. |

## Section 4: Atmosphere and animation

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R14 | **Atmosphere.apply() dead** | `atmosphere.rs` | LOW | Full-frame bloom function defined but never called. Zero callers outside atmosphere.rs. |
| R15 | **Atmosphere breathing/heartbeat/spinner ALIVE** | Various widgets | INFO | `breathing_brightness()` used by: agent_pool (`agent_pool.rs:109`), sys_metrics (`sys_metrics.rs:133`), token_sparkline (`token_sparkline.rs:100`). `heartbeat()` used by: plan_tree (`plan_tree.rs:235`), header_bar (`header_bar.rs:90`). `spinner_ethereal()` used by: phase_compact (`phase_compact.rs:84,186`). These ARE working. |
| R16 | **Dual atmosphere instances** | `app.rs`, `state.rs:370` | MEDIUM | Both `app.atmosphere` (in App struct) and `tui_state.atmosphere` (in TuiState) exist. Both `.tick()`ed in event loop. Widgets read from `tui_state.atmosphere`. PostFX pipeline reads from `app.atmosphere`. Potential phase drift between them. |

## Section 5: VFX gaps

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R17 | **No background visualization** | N/A | LOW | No plasma/noise cellular background driven by task progress. Mori has this. |
| R18 | **No nerv_viz** | N/A | LOW | No real-time state visualization in agent output backgrounds. |
| R19 | **No vfx.rs module** | N/A | LOW | No plasma, noise, field generators. Would need new file. |

## Section 6: Render loop gaps

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R20 | **Message throttle missing** | `app.rs` event loop | MEDIUM | No `MAX_MESSAGES_PER_TICK` limit. If orchestrator produces rapid output, TUI may process unlimited messages per frame, starving render. Mori caps at 20. |
| R21 | **SmoothedValues missing** | `state.rs` | MEDIUM | No EMA smoothing over cost/CPU/mem/tokens. Values in `SysMetrics` jump directly. `sys_metrics` widget reads raw `cpu_history`/`mem_history` Vecs but these are populated by direct push, not EMA. |
| R22 | **Notification expiry exists but may leak** | `app.rs`, `state.rs` | LOW | `Notification` struct has `timestamp_ms` field. But expiry timing depends on where `expire_notifications()` is called in the event loop -- needs verification that it runs every tick. |

## Section 7: Theme system split

| ID | Gap | File:Line | Severity | Detail |
|----|-----|-----------|----------|--------|
| R23 | **Two theme systems** | `dashboard.rs:80` (Theme), `widgets/rosedust.rs` (MoriTheme) | MEDIUM | `Theme` struct in `dashboard.rs:80` used by modals and app.rs. `MoriTheme` struct in `widgets/rosedust.rs` used by all active widgets (plan_tree, phase_compact, task_progress, wave_progress, sys_metrics, header_bar, status_bar, etc.). Same color values but different API surfaces. Modals take `&Theme`, widgets take `&TuiState` and access MoriTheme internally. |
| R24 | **help.rs uses third path** | `modals/help.rs:9` | LOW | `use crate::tui::mori_theme::MoriTheme` -- this module path does not exist in `tui/mod.rs`. Would fail to compile. File is dead code. |

## Section 8: Working features (previously claimed broken)

For accuracy, these items from the previous doc are CONFIRMED WORKING:

| Feature | File:Line | Evidence |
|---------|-----------|----------|
| Responsive outer margin | `app.rs:draw()`, `layout.rs:responsive_outer_margin()` | 1-cell margin when >= 120x50 |
| Dim overlay on modal/help | `app.rs:432-435` | `dim_overlay(content_area, buf, 0.45)` called when modal or help active |
| Immediate draw after keypress | `app.rs:337-343` | `last_input` timestamp triggers immediate redraw |
| Adaptive frame rate | `app.rs:354-367` | Skip frames when idle > 3s and no active agents |
| Notifications populated + rendered | `app.rs:670-700`, `modals/mod.rs:177` | SubmitInject/ConfirmYes push notifications, render_notifications renders them |
| Hit test mouse routing | `app.rs:1090-1110`, `hit_test.rs` | HitZones::compute + zone_at used for mouse click focus |
| Atmosphere breathing/heartbeat | Multiple widgets | `breathing_brightness()`, `heartbeat()`, `spinner_ethereal()` all have active callers |
| Braille sparklines | `widgets/braille.rs` -> `sys_metrics.rs`, `token_sparkline.rs` | braille_spans_f32/f64/u64 called from active widgets |
