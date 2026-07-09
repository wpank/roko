# TUI Runtime Gaps

What actually happens when you run `roko dashboard`. Traced end-to-end from CLI entry to each frame, each tab, each keypress. Every silent failure, empty render, and broken interaction.

**Audit date**: 2026-04-14
**Method**: Full code trace of `main.rs:cmd_dashboard` → `App::new_with_page` → `main_loop` → `draw` → each view

---

## CRITICAL — Data loading broken at startup

| ID | Gap | Where | Impact |
|----|-----|-------|--------|
| RT1 | **Plans directory mismatch** — `plans_dir()` returns `.roko/plans/` but actual plans live at `./plans/` (top-level: P06, P07, W01) | `dashboard.rs:load_plan_summaries()` line 1558 | All three real plans are invisible to the TUI. Plan tree empty. Task counts 0/0. Phase pipeline pending. Wave indicators hidden. The entire dashboard is a blank shell. |
| RT2 | **Episode file path conflict** — orchestrator writes to `.roko/episodes.jsonl`, TUI reads `.roko/memory/episodes.jsonl` | `dashboard.rs` uses `MEMORY_DIR = ".roko/memory"` + `EPISODES_FILE`, but `orchestrate.rs:11534` writes to `.roko/episodes.jsonl` | When orchestrator runs and creates episodes, the TUI will never find them. Agent output, token counts, model info all derived from episodes — all will be permanently empty. |
| RT3 | **DashboardData returns fully empty** — zero plans, zero agents, zero signals, zero episodes, zero efficiency events, zero experiments, zero gate results | Every `.roko/` file is either empty or absent on fresh workspace | Every data-driven widget renders at empty/zero. TUI is a blank shell with only git info and animations. |

## HIGH — Tabs render empty/broken content

### What the user actually sees on each tab

| ID | Tab | What renders | Problem |
|----|-----|-------------|---------|
| RT4 | **F1 Dashboard — Plan tree** | "Plans (0/0)" with empty box, column headers only | No plans found due to RT1 path mismatch |
| RT5 | **F1 Dashboard — Phase compact** | "Phase" box with blank first line and "waiting..." text | Phase pipeline never populated from disk data |
| RT6 | **F1 Dashboard — Task progress** | "Tasks (0/0)" with spinner + "waiting for tasks..." | Task checklist never populated from disk |
| RT7 | **F1 Dashboard — Agent sub-tab** | "no parallel agents" + "no agent output yet" | No agents in DashboardData |
| RT8 | **F1 Dashboard — Output sub-tab** | "no agent output yet" | No episodes, no task outputs |
| RT9 | **F1 Dashboard — Diff sub-tab** | "no diff" centered | No diff content source |
| RT10 | **F1 Dashboard — Errors sub-tab** | "No gates evaluated" + "No errors" | gate_results never bridged |
| RT11 | **F1 Dashboard — Git sub-tab** | "loading git data..." then REAL data after ~1-3s | WORKS after background thread delivers |
| RT12 | **F1 Dashboard — MCP sub-tab** | "input tokens: 0 / output tokens: 0 / total cost: $0.0000" | No efficiency data |
| RT13 | **F1 Dashboard — Procs sub-tab** | "no tracked processes" | No agent data |
| RT14 | **F1 Bottom ribbon — Wave progress (40% width)** | COMPLETELY BLANK — renders nothing | `execution_waves` empty → widget returns immediately → 40% of ribbon is visual hole |
| RT15 | **F1 Bottom ribbon — Token sparkline** | "waiting for data..." with ghost border | token_history never populated |
| RT16 | **F1 Bottom ribbon — Sys metrics** | CPU 0.0%, MEM 0K, NET 0B, DSK 0B, but FPS shows real value | Sys metrics arrive after ~2s delay from `top` command; first frames show zeros |
| RT17 | **F2 Plans** | "Plans (0/0)" empty list + detail panel for nonexistent plan | Same RT1 cause |
| RT18 | **F3 Agents** | "no agents spawned" + "waiting for agent output..." | Polished empty state messages — GOOD empty state UX |
| RT19 | **F4 Git** | "loading..." then REAL branches, commits, worktrees, status | WORKS after ~1-3s. Best-populated tab. |
| RT20 | **F5 Logs** | "no log entries -- run agents to generate signals and episodes" OR real entries if JSONL files exist | WORKS when data exists, correct empty state |
| RT21 | **F6 Config** | Real config from roko.toml, editable fields, save works | WORKS — best functioning tab |
| RT22 | **F7 Inspect** | All zeros: "input tokens: 0, output tokens: 0, total cost: $0.0000, pass rate: 0.0%, C-Factor: (not computed)" | No efficiency/cascade/gate data |
| RT23 | **Header bar** | Heartbeat animates, "roko" label, "0/0" tasks, "$0.00", "0" tokens, CPU 0%, MEM 0K | Mostly zeros; F-key strip shows F1-F6 only (F7 missing!) |
| RT24 | **Status footer** | Git branch + commit + age (WORKS), "0/0" progress, keybind hints | Git section works after initial populate_git_info() |

## HIGH — Key presses that do nothing or behave wrong

| ID | Key | Expected | Actual | Problem |
|----|-----|----------|--------|---------|
| RT25 | **Enter on plan** | Plan detail modal opens | `show_plan_detail` boolean toggles but NOTHING RENDERS — no `ModalState::PlanDetail` variant, no render function called | Completely broken — user sees nothing happen |
| RT26 | **`i` to inject** | Input line appears, user types, sees text | State machine works, typing captured, submit writes signal — but NO VISUAL INPUT LINE RENDERED | User types completely blind. No input box, no cursor, no mode indicator anywhere. |
| RT27 | **`/` to filter** | Filter input line appears | Same as inject — typing captured but NO VISUAL FEEDBACK. Additionally, `filter_text` (where typing goes) is never copied to `filter` (what plan_tree reads) | Double broken: invisible AND filter doesn't apply |
| RT28 | **`p` to pause** | Pipeline pauses, indicator shows | `pipeline_run_state` toggles between strings but NO WIDGET READS IT and NO SIGNAL SENT TO ORCHESTRATOR | Nothing visible happens, nothing functional happens |
| RT29 | **`w` for wave overview** | Wave status modal with data | Modal renders but ALWAYS EMPTY — `waves: Vec::new()` hardcoded instead of reading from `tui_state.execution_waves` | Empty modal. Also: Esc does NOT close it (no key intercept). |
| RT30 | **`y`/`a` to approve** | Approves pending agent command | `pending_approval` is NEVER set to `Some(...)`, so there's never anything to approve. The handler just sets `None` to `None`. | Complete dead code path |
| RT31 | **PageUp/PageDown** | Scroll by page | Calls `scroll_focused(+/-1)` — scrolls by 1 line, identical to Up/Down | Functionally broken — PgUp = Up |
| RT32 | **Tab to cycle focus** | Visible focus indicator moves between panels | Focus zone changes internally, but MOST panels have NO visible focus indicator — only plan_tree shows a subtle title text change | User can't tell which panel is focused |
| RT33 | **Ctrl-C in modal** | Force quit | Swallowed by modal intercept handlers for task_picker, confirm, inject, filter modes — returns `TuiAction::None` | Must Esc first, then Ctrl-C |

## MEDIUM — Functional but degraded

| ID | Gap | Where | Impact |
|----|-----|-------|--------|
| RT34 | First frame renders before any background data arrives | `app.rs:325-327` — draw called before sys/data/git threads deliver | User sees ~2s of all-zeros before CPU/MEM populate. Git info arrives ~1-3s later. |
| RT35 | `git_age` never refreshes from background thread | `app.rs:305` sets `age = String::new()` in GitBgData | Git age is set once at startup by `populate_git_info()` and never updated. Goes stale during long sessions. |
| RT36 | Data refresh thread polls empty files at 500ms | `app.rs:283` re-reads all `.roko/` files every 500ms | Burns unnecessary I/O. No change detection optimization for missing files. |
| RT37 | Plans all marked `active: !completed` | `state.rs:744` sets `active: !completed` | Incomplete plans show as "active" with play icon even though nothing is running. Misleading status. |
| RT38 | sys_metrics macOS-only — Linux shows all zeros | `app.rs:1633` `#[cfg(target_os = "macos")]` guards `collect_sys_metrics_bg()` | On Linux, CPU/MEM/NET/DSK are permanently 0 with no indication collection is unsupported |
| RT39 | Network/disk metrics contain totals, not rates | `app.rs:1682-1683` fields named `_bytes_sec` store cumulative totals | NET/DSK display shows boot-time totals, not per-second rates. Labels lie. |
| RT40 | F7 Inspect NOT listed in header F-key strip | `header_bar.rs` only renders F1-F6 labels | User doesn't know F7 exists unless they press it or read help |
| RT41 | Config save doesn't trigger runtime reload | `config_view.rs` writes to roko.toml but App doesn't re-read config | Changes require TUI restart to take effect |

## What actually WORKS correctly

| Feature | Evidence |
|---------|----------|
| Terminal setup/restore (raw mode, alternate screen, mouse capture, panic hook) | `app.rs:1557-1586` |
| Heartbeat/breathing animations | `atmosphere.rs` ticked every frame, consumed by widgets |
| F1-F7 tab switching | Clean, immediate, correct view dispatch |
| Git view (F4) — branches, commits, worktrees, status | Background thread collects real git data |
| Logs view (F5) — unified log from JSONL files | Correct when data files exist |
| Config view (F6) — edit/save roko.toml | Full editing, cycling, saving with notifications |
| Help overlay (`?`) | Comprehensive keybind reference with dim overlay |
| Quit (`q`) — two-press when modals open, instant otherwise | Proper modal dismiss then quit |
| Ctrl-R refresh | Synchronous disk reload |
| Ctrl-S config save | Writes roko.toml with notification feedback |
| Adaptive frame rate | 60fps active, ~20fps idle after 3s |
| Terminal resize | Handled implicitly by ratatui |
| Inject state machine | Signal written to signals.jsonl (but invisible typing) |
| Notification toasts | Populated by inject/confirm/save, rendered correctly |

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL (data loading broken) | 3 |
| HIGH (tabs render empty/broken) | 21 |
| HIGH (keys do nothing/wrong) | 9 |
| MEDIUM (degraded but functional) | 8 |
| **Total runtime gaps** | **41** |
