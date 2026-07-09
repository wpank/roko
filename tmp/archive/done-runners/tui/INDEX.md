# TUI Gap Index

Definitive exhaustive list of everything not done in the TUI. Every gap traced to file:line. Every claim verified against actual code. Runtime behavior verified by full execution trace.

**Audit date**: 2026-04-14
**Reference**: Mori at `/Users/will/dev/uniswap/bardo/apps/mori/src/`
**Target**: Roko at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`
**Method**: Static code analysis + full runtime execution trace (CLI entry → frame render → keypress dispatch)

---

## Documents

| File | What | Items |
|------|------|-------|
| [01-data-flow-gaps.md](01-data-flow-gaps.md) | Every TuiState field traced from source to consumer | 44 |
| [02-widget-gaps.md](02-widget-gaps.md) | Every widget file read line-by-line, callers verified | 56 |
| [03-input-gaps.md](03-input-gaps.md) | Every TuiAction variant, key handler, and dispatch arm | 41 |
| [04-modal-gaps.md](04-modal-gaps.md) | Every modal file, open/close paths, key intercepts | 19 |
| [05-view-gaps.md](05-view-gaps.md) | Per-tab feature matrix, sub-tab completeness | 37 |
| [06-dead-code.md](06-dead-code.md) | Every pub fn checked for callers, dead module analysis | 29 |
| [07-rendering-gaps.md](07-rendering-gaps.md) | PostFX, layout, atmosphere, theme systems | 24 |
| [08-runtime-gaps.md](08-runtime-gaps.md) | **What actually happens when you run `roko dashboard`** — data loading, tab renders, keypress behavior | 41 |
| [09-stubs-placeholders.md](09-stubs-placeholders.md) | **Every hardcoded value, silent failure, placeholder, stringly-typed state** | 64 |

---

## Total gap count: 355

| Severity | Count | Examples |
|----------|-------|---------|
| **CRITICAL** | 7 | Plans directory mismatch (RT1), episode path conflict (RT2), mouse events dropped (K1), ScrollAccel dead (K2), token sparkline empty (W27), sys_metrics zero (W20), plan_scroll desync (W24) |
| **BLOCKING** | 12 | token_history never populated, gate_results never bridged, parallel_agents empty, pending_approval dead |
| **HIGH** | ~80 | 21 tabs render empty, 9 keys do nothing/wrong, orphaned modals, empty data modals, no input rendering for inject/filter |
| **MEDIUM** | ~110 | Heuristic phases, hardcoded 200K context, binary progress, macOS-only metrics, 90+ stringly-typed comparisons |
| **LOW** | ~146 | Dead widgets (13 files), dead code, duplicates, placeholder scaffold, polish |

---

## The 3 root causes that make the TUI a blank shell

### 1. Plans directory mismatch (RT1)
`plans_dir()` returns `.roko/plans/` but actual plans live at `./plans/`. **Fixes**: plan tree, task counts, header progress, phase pipeline, wave indicators, all plan-derived data.

### 2. Episode file path conflict (RT2)
Orchestrator writes `.roko/episodes.jsonl`, TUI reads `.roko/memory/episodes.jsonl`. **Fixes**: agent output, token counts, model info, phase elapsed times, task status.

### 3. DashboardData returns empty on fresh workspace (RT3)
All `.roko/` data files are empty or absent. **Partially expected** on a workspace where no agents have run, but combined with RT1 and RT2, the TUI has zero data even after agents run.

Fix RT1 and RT2 and the TUI immediately becomes useful for projects that have run agents.

---

## The 10 highest-impact fixes (in dependency order)

1. **Fix `plans_dir()` path** — `.roko/plans/` → `./plans/` (or wherever plans actually are). One-line fix, unblocks most of the dashboard.

2. **Fix episode file path** — align orchestrator write path and TUI read path to same file. One-line fix, unblocks agent data display.

3. **Add `Mouse` variant to `Event` enum** + handle in `EventHandler::next()`. ~5 lines, unblocks all mouse support.

4. **Render inject/filter input text** — add an input line widget in `draw()` when `input_mode == Inject` or `Filter`. ~30 lines, makes inject and filter usable.

5. **Fix PageUp/PageDown** — call `scroll_focused(+/-page_height)` instead of `+/-1`. ~2 lines.

6. **Populate modals with real data** — pass `tui_state.execution_waves` to WaveOverview, tasks to TaskPicker. ~3 lines each.

7. **Bridge gate_results** — in `update_from_snapshot()`, convert `DashboardData.gate_results` → `TuiState.gate_results`. ~10 lines.

8. **Instantiate ScrollAccel** — add field to App, call `push()` in scroll handlers. ~15 lines.

9. **Wire `plan_scroll` to `plan_scroll_offset`** — either rename or sync them. 1 line.

10. **Add sysinfo for Linux** — use `sysinfo` crate or `/proc` parsing. ~50 lines.

---

## What actually WORKS (verified by runtime trace)

| Feature | Status |
|---------|--------|
| Terminal setup/restore (raw mode, alternate screen, panic hook) | Working |
| Heartbeat/breathing animations | Working |
| F1-F7 tab switching | Working |
| Git view (F4) — branches, commits, worktrees, status | Working (after ~1-3s) |
| Logs view (F5) — unified log from JSONL files | Working when data exists |
| Config view (F6) — edit/save roko.toml | Working (best tab) |
| Help overlay (`?`) | Working |
| Quit (`q`) — modal-aware two-press | Working |
| Ctrl-R refresh | Working |
| Ctrl-S config save | Working |
| Adaptive frame rate (60fps → 20fps idle) | Working |
| Terminal resize | Working (via ratatui) |
| Notification toasts | Working |
| Status footer git info | Working |
| Rosedust color theme | Working |

---

## Corrections from previous audit

| Claim | Actual |
|-------|--------|
| `views/agents_view.rs` is dead code | WRONG — wired via `render_tab_content()` |
| `views/context_view.rs` is dead code | WRONG — wired |
| `views/logs_view.rs` is dead code | WRONG — wired |
| `dim_overlay()` never called | WRONG — called at `app.rs:435` |
| No responsive margin | WRONG — `responsive_outer_margin()` is wired |
| No adaptive frame rate | WRONG — implemented at `app.rs:354-367` |
| No immediate draw after keypress | WRONG — implemented at `app.rs:337-343` |
| Notifications never populated | WRONG — inject/confirm/save push notifications |
| `phase_compact.pct` always 0.0 | WRONG — 50.0 for Active, 100.0 for Done |
| `SysMetrics` never populated | PARTIALLY WRONG — macOS `top` parsing works, but only after ~2s delay |
