# UX/TUI/workflow backlog crosswalk

**Audit date:** 2026-09-01
**Baseline:** pushed `main` through `a8625a9cc`, with the original P0-P7 count anchored at
`fced716b6`
**Method:** reconcile the six parity audits with current production call sites, focused tests, and
`tmp/tui-parity2/31-LIVE-RUN-EVIDENCE.md`. A renderer, field, key handler, or backlog checkbox is
not completion unless its producer and consumer are both operational.

Status meanings:

- **Verified** — the behavior is present on the production path and backed by focused tests or
  equivalent captured evidence.
- **Partial** — useful implementation exists, but an original acceptance criterion, live producer,
  acknowledgement, or validation fixture is still absent.
- **Missing** — the requested production capability is not implemented; adjacent scaffolding does
  not change this classification.

Follow-up code is reported as **source-present** where live acceptance evidence is still absent. It
is never promoted solely because it compiles or has a unit test.

## Verified

| Backlog item | Verified behavior | Boundary |
|---|---|---|
| **#130 — Content-Aware Tab Badges** | `TuiState::tab_badge` supplies counts to `widgets/header_bar.rs`; the active tab suppresses its badge, and focused state/header tests cover the behavior. | This verifies badges, not the completeness of the underlying metrics (see #239/#240). |
| **#201 — TUI Notification Toasts for Runtime Events** | `apply_dashboard_snapshot` maps new gate, plan-terminal, stall, and error events; `push_deduped_notification` suppresses two-second duplicates; `App::tick` caps the transient stack at 20. Current unstaged rendering also caps visible density, preserves the footer, and safely truncates Unicode. | This is transient-toast behavior only. Expired notifications are deleted, so #369 remains missing. |
| **#217 — TUI Log Search/Filter** | `LogSearchState`, `/`, `n`/`N`, highlight/filter rendering, invalid-regex handling, and visible-list match indexing are wired through `input.rs`, `app.rs`, and `views/logs_view.rs`, with focused tests. | Search covers the retained in-memory log list; it is not paged history (#367). |
| **#219 — TUI Plan Tree Filter/Search** | `PlanTreeFilter`, text/status predicates (including `status:failed`), filtered rendering, and navigation preserving the real plan identity are wired and tested in `state.rs`, `app.rs`, and `widgets/plan_tree.rs`. | This does not make the displayed wave topology authoritative (#125). |

## Partial

| Backlog item | What is implemented | What still prevents completion |
|---|---|---|
| **#71 — TUI Theme Alignment with ROSEDUST**, **#73 — UX Backlog Rollup**, **#107 — Plan Run UX Friction** | Mori-like hierarchy, restrained default effects, responsive header work, connected state, search, filters, and some runtime controls are real. | These are umbrella items. Open control, streaming, topology, resize, performance, and evidence items below prevent closing them as a group. |
| **#108 — TUI Live Feedback and Plan Run Performance Gaps** | The runner creates `TuiBridge` before cache warming, publishes startup status, emits 15-second `AgentHeartbeat`s, exposes tools after dispatch, avoids default Cargo gates for non-Rust diffs, and publishes selected gate labels. | The standalone dashboard is still file/poll based; provider output/tokens are only as granular as provider events; gate subprocess lines are not streamed; the displayed rung is a joined selected set rather than actual rung transitions. The completed provider run predates the latest source changes, so no new paid end-to-end proof exists. |
| **#111 — Screenshot Command Completion** | `roko screenshot` captures selected/all tabs through `App::render_tabs_to_text`, writes schema-v2 manifests, supports dimensions/labels, and manages a `latest` symlink. | It produces plain `.txt` frames only; there is no `--compare`, ANSI/style preservation, or pixel output. The backlog's stronger visual-evidence acceptance is not met. |
| **#112 — Plan Run Continuous Screenshots** | Released `ScreenshotCollector` wiring adds `plan run --screenshots`, interval/output options, a bounded worker queue, startup/event/interval/shutdown captures, low-disk skipping, atomic manifest updates, unique directories, and focused unit tests. Captures materialize live `StateHub` snapshots through the production full-frame draw path. | It has not been validated in a live plan run. Files are plain text, gate capture metadata lacks the exact rung, and shutdown joins the worker. Original startup/shutdown/gate/30-second fixture checks remain open. |
| **#120 — Plan Run Preflight Checks** | Provider/tool/worktree checks and warnings run before execution. | The live session still exposed environment warnings and a cold-start visibility gap; there is no complete tested decision/repair flow for every warning. |
| **#121 — TUI Data Model Unification** | A `TuiModel` facade and snapshot application helpers exist. | `App` still owns both `DashboardData` and `TuiState`; legacy pull data remains passed through views and token-sparkline fallback. The dual model is not retired. |
| **#123 — ROSEDUST Color Palette Port** | The visual palette and semantic `Theme` constants are broadly used, and Mori is the accepted visual baseline. | There is no dedicated completed palette port/fallback proof; inline/legacy styling remains, progress semantics are not the requested full gradient contract, and text screenshots cannot verify ANSI colors. |
| **#124 — Header Bar Mori Parity**, **#239 — Header Bar Enrichment** | A compact responsive header, warnings, badges, MCP/NET/DSK/FPS slots, progress, elapsed time, and ETA fallback render. | Critical-path ETA is never produced, NET/DSK data has correctness gaps, and the 80-column/live-terminal acceptance matrix is not complete. |
| **#125 — Plan Tree Wave Hierarchy Widget** | The plan tree groups plans under execution-wave rows and supports drill/filter/navigation. | Live evidence showed a flat `Wave 1/1`; displayed hierarchy can reflect synthesized execution grouping rather than authored/runtime dependencies. |
| **#126 — Error Digest Widget** | Gate failures, runtime errors, and conductor diagnoses have visible panels/toasts. | There is no demonstrated cross-source dedupe/remediation lifecycle matching the backlog's complete digest acceptance. |
| **#127 — F7 Inspect View Parity** | Inspect subviews, MCP/config caches, learning panels, token/cost views, and prompt/model tables are reachable. | Several connected fields remain aggregate, inferred, or empty; source presence has not been backed by a representative live F7 evidence set. |
| **#128 — Event-Driven TUI Render Loop and Adaptive Tick Rate** | Connected mode redraws on `StateHub` watch changes and still services input/ticks. | Standalone mode continues periodic file polling; there is no shared adaptive policy or p95 draw/latency benchmark proving the requested behavior. |
| **#129 — Metric Exponential Smoothing** | EMA is applied to CPU and token/cost rates, with a focused convergence test. | The backlog also requires smooth progress and per-role token displays; those values remain discrete. There are two `SmoothedValue` implementations rather than the proposed single reusable path. |
| **#146 — Plan CLI Control Commands** | CLI pause/resume/cancel/retry files are polled by Runner-v2. Pause/resume really stop/restart scheduler dispatch. | Cancel invokes the run-wide cancellation token rather than a plan-scoped state transition; retry only logs “next tick”; commands lack active-run validation and acknowledgements. The cancel/retry acceptance tests therefore fail conceptually. |
| **#156 — Per-Model/Provider Cost Stats TUI Display** | F7 has a cost/model widget with pass rate, duration, token, provider, and cost aggregates. | Connected cost can be proportionally inferred from global totals; authoritative cost-by-model events remain incomplete, and the requested interactive cost sort/live reconciliation is not fully proven. |
| **#157 — Context-Sensitive Keybind Hints** | Hints vary by tab and selected task state. Released help/status text labels unimplemented recovery commands as pending instead of claiming they work; narrow footers drop whole low-priority tokens and retain `?:help` instead of clipping text. | Several actions still have no runner consumer, and active-run state acceptance has not been exercised across the terminal matrix. |
| **#182 — Dedicated Lightweight `status.json`** | Status writing exists. Released finalization code makes terminal success/failure/cancel override stale gate/task state, writes terminal status immediately, closes synthetic plan-verify attempts, and preserves real plan/run totals; focused terminal-status tests pass. | The schema still lacks the requested PID and uses `phase` rather than the documented `current_phase`; kill/stale-process behavior and a post-change live terminal fixture are unverified. |
| **#189 — Agent Status Panel** | F3/F1 render dense active-agent rows with model, task, role, context, tokens, progress, and transcript selection. | Provider snapshots do not reliably populate every requested effort/context/turn field, and the 30+ agent/scroll live fixture is absent. |
| **#216 — TUI Queue Overview Modal** | A navigable queue modal renders milestone-like wave progress. | It derives milestones from `TuiState::execution_waves`, not `queue.toml`, and F2 opens it with `o` (global `u`/F8), not the specified `q`. |
| **#228 — Dogfood Session Evidence Bundle** | `scripts/run_evidence.py`, the validator, benchmark wrapper, schema-v2 manifest, redaction, bounded logs, run-scoped slices, metrics, and debrief generation are source-complete; the strict explicit-only fixture passed. | Seven runtime fixtures remain unchecked: successful one-task run, pre-event exit, gate timeout, pre-existing log isolation, secret rejection, live-output latency, and five cold/five warm repetitions. Source-complete is not evidence-complete. |
| **#232 — TUI Connected-Mode Data Bridge** | Runner events flow through `StateHub`/`TuiBridge` into connected `TuiState`; core task/agent/token/cost/gate-completion data renders. | Typed learning history, authoritative model costs, live git/attempt metadata, critical ETA, and true gate streaming are absent or incomplete. Standalone mode still uses files. |
| **#233 — Executor-Neutral TUI Command Transport** | A bounded in-process `TuiCommand` channel reaches Runner-v2; pause/resume are operational. Current unstaged UI no longer fakes pause when no runner accepts the send. | There is no frozen executor-neutral `ExecutionCommand`/ack contract, command ID/run scope, accepted/rejected/completed lifecycle, stale/full/disconnected UX, or fake/graph adapter. Retry/repair/reverify/skip are log-only and cancel is global. |
| **#234 — Gate Output Streaming to TUI** | Real bounded gate output is delivered to the gate widget after completion; cargo/test lines are colorized and scrollable. Unstaged code publishes a gate-start label and elapsed state. | The producer sends the completed buffer, not stdout/stderr lines while the subprocess runs. Selected labels are not per-rung transition events. “Streaming” remains an overclaim. |
| **#235 — TUI Render-Path Disk I/O Elimination** | Production render functions in `tui/views`, `widgets`, and `modals` no longer call filesystem reads; MCP/config/inspect data comes from TTL/invalidation caches in `TuiState`. | This verifies the narrow disk-I/O fix, not the full backlog acceptance: no 5-second external-edit fixture, write-race fixture, or ≥30-fps heavy-I/O benchmark was run. `Theme::from_env()` still executes in `App::draw`, so “no per-frame work/I/O” is too broad. |
| **#237 — TUI Keyboard Model Fixes** | `v` maps to reverify; focus-zone types/cycles exist; Help scrolls; Diff and Procs have separate stored offsets. | Global digits are suppressed only on Agents/Logs, so `1` on F2 still switches to F1 instead of selecting plan 1. Normal Procs and several other zones still mutate `diff_scroll`. Help now admits recovery consumers are pending, but the original end-to-end bindings are not complete. |
| **#238 — Plan Detail Enrichment** | Plan detail renders elapsed, branch/worktree/commit and diff/file-stat fields when present. | Connected `TaskEntry` has no dependency/acceptance/verify fields; connected constructors commonly leave branch/worktree/commit/files/diff stats empty or target the root rather than the accepted attempt worktree. |
| **#240 — NET/DSK System Metrics Sampling** | CPU/memory/network/capacity fields are sampled and displayed. | Network delta units are wrong, disk I/O rate is not correctly populated, and the macOS/heavy-compilation acceptance fixture is absent. |
| **#241 — TUI Visual Density Improvements** | The bottom ribbon, compact header, restrained effects, responsive hiding, bounded toasts, narrow Agents/Logs layouts, complete footer tokens, and single dominant transcript materially improve Mori-like legibility. | Dashboard phase layout still reserves a fixed block while idle, several views retain oversized borders/empty regions, and the checked-in terminal matrix is static rather than a live resize fixture. |
| **#365 — Fix TUI Modal Input Precedence End to End** | Most named modals intercept keys before normal tab input; several modal-local navigation tests exist. | `ModalState::Quit` falls through the active-modal match before confirm handling, and the full key→action→state cases for Quit, destructive confirms, Inject, and BatchReview are not covered. |

## Missing

| Backlog item | Missing production capability / evidence |
|---|---|
| **#110 — Deprecate All JSONL File I/O — StateHub as Single Source of Truth** | JSONL writers/readers remain widespread, and standalone dashboard/replay paths intentionally depend on persisted files. |
| **#122 — Remove Legacy Page System** | `PageId`, `PageScaffold`, `PageRegistry`, and legacy page modules remain exported and used by `dashboard.rs`/`app.rs`. |
| **#148 — TUI God Objects Decomposition** | `app.rs`, `state.rs`, and `dashboard.rs` remain large multi-responsibility modules; no requested controller/model/view decomposition exists. |
| **#151 — TUI PNG Snapshot Rendering** | No `tui-png` feature, PNG rasterizer, reusable font atlas, `--format png`, or pixel/color verification exists. |
| **#152 — Screenshot Diff/Compare Engine** | No screenshot diff subcommand, baseline store, similarity score, tolerance, or diff image exists. The unrelated top-level `--compare` flag is not screenshot comparison. |
| **#153 — Automated Visual Assessment Loop** | No automatic reference comparison, threshold gate, regression-generated fix plan, reference lifecycle, or learning record exists. |
| **#196 — Critical Path ETA Computation and Display** | `task_dag::remaining_eta_minutes` and the display field exist, but production never assigns `TuiState::critical_path_eta_minutes`; `roko status` has no authoritative critical ETA. |
| **#199 — TUI Resizable Panes** | No `PaneConfig`, resize bindings, per-tab ratios, persistence file, or shared split helper exists. |
| **#236 — TUI Empty State Messages Improvement** | Contextual messages were added to some core panels, but the original generic states remain in dashboard, gate, agent-output, router, token, learning, and plan views. The item cannot be claimed complete TUI-wide. |
| **#323 — Status Snapshot Source Unification and Offline Behavior** | Multiple status/snapshot/file sources and fallback builders remain. The unstaged terminal `status.json` projection fix closes one stale-terminal symptom but does not unify sources, PID checks, offline behavior, or error reporting. |
| **#327 — Plan-Scoped Run Lifecycle and Humane Resume UX** | The frozen plan-scoped lifecycle, run inventory/archive/GC, scoped migration, shared Runner-v2/Graph decision matrix, and safe `--new` semantics do not exist. The live run's stale-resume behavior is direct negative evidence. |
| **#366 — Remove Per-Frame TUI Data-Pipeline Waste** | `App::draw` still calls `Theme::from_env()` every frame; legacy snapshot/model copying and render-time collection work remain; no allocation benchmark or ≤16 ms p95 gate exists. |
| **#367 — Paged Agent Output History and Search** | Agent transcript history is memory-bounded only; there is no canonical paging, historical search, dedupe across live/settled storage, or redacted page-in path. |
| **#368 — Route TUI Mouse Scroll and Click by Rendered Panel Coordinates** | Mouse routing still relies on generic hit/focus behavior and shared scroll fallbacks; no rendered panel registry or modal-safe coordinate routing satisfies the acceptance matrix. |
| **#369 — Retain and Inspect TUI Notification History** | Expired toasts are removed from `App::notifications`; there is no bounded retained history, filter/navigation, canonical jump, redaction boundary, or eviction counter. |

## Screenshot evidence boundary (#111/#112 versus #151–#153)

Current static and continuous captures are valuable **full-frame text evidence**: they invoke the
same `App::draw` layout/content path as the terminal and serialize the resulting cell symbols to
`.txt`. They do **not** serialize ANSI attributes, RGB colors, terminal font rasterization, or pixels.
Therefore:

- “full-frame” proves layout/content reachability at a chosen cell size;
- it does not prove ROSEDUST color fidelity, terminal compositor behavior, or PNG appearance;
- no visual regression score or reference comparison exists until #151 and #152 are implemented;
- no automated assess→plan→fix→learn loop exists until #153 is implemented.

## Immediate closure order

1. Finish #233's acknowledged, run-scoped command contract; make retry/repair/reverify/skip/cancel
   operational before advertising them.
2. Add a true gate-line producer and exact rung transitions for #234.
3. Run #112's live startup/interval/gate/shutdown/low-disk fixture matrix and attach the manifests.
4. Run #228's seven remaining evidence fixtures, including a post-terminal-projection provider run.
5. Fix #237/#365/#368 input routing before adding more controls.
6. Implement #151/#152 only after text evidence is stable; then make #153 consume those artifacts.
