# Checklist Gaps — IMPLEMENTATION-CHECKLIST.md vs Existing Backlog

> **Source document**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` (2026-08-19)
> **Compared against**: `tmp/backlog/00-INDEX.md` items 01–110
> **Analysis date**: 2026-08-19

This document lists every concrete task from the implementation checklist, notes which
existing backlog item covers it (if any), and flags what is new or only partially covered.

---

## Phase 0: Claude Observability Infrastructure

### 0.1 — TUI Headless Snapshot Mode (Text + PNG)

**Checklist description**: `roko dashboard --snapshot <dir>` command to render every TUI
tab to text + PNG files using `TestBackend`. Produces `manifest.json`. Baseline for all
automated visual assessment.

**Coverage**: PARTIALLY IMPLEMENTED (new files exist but spec not fully met)

- `crates/roko-cli/src/tui/snapshot.rs` — text rendering engine exists and works
- `crates/roko-cli/src/commands/screenshot.rs` — `SnapshotConfig` / `capture_snapshots` wired
- Untracked new file `tui/snapshot.rs` shows the engine is written

**Gaps vs checklist spec**:
- PNG rendering is NOT implemented (text-only; no `image` crate, no font atlas)
- ANSI file output is NOT implemented
- Sub-view (per-SubView) rendering is NOT implemented (only top-level tabs)
- The `--snapshot` flag on `roko dashboard` specifically is NOT wired — only `roko screenshot` is
- `--snapshot-width` / `--snapshot-height` flags not wired to `dashboard` subcommand

**Suggested backlog item**: New — extend the existing `snapshot.rs` to add sub-view iteration
and wire `--snapshot` onto `dashboard` (the `screenshot` command is the separate Phase 0.2 item).

**Priority**: P2 | **Size**: S (1 day) — PNG rendering is Phase 2+ scope, skip it

---

### 0.2 — `roko screenshot` Top-Level Command

**Checklist description**: Dedicated `roko screenshot` command. Captures all surfaces,
symlinks `.roko/screenshots/latest/`, supports `--tabs`, `--pages`, `--format`, `--size`,
`--compare`, `--label`.

**Coverage**: PARTIALLY IMPLEMENTED

- `crates/roko-cli/src/commands/screenshot.rs` — command skeleton exists (untracked file)
- `capture_snapshots()` in `tui/snapshot.rs` is wired

**Gaps vs checklist spec**:
- `--compare` flag NOT implemented
- `--format` flag NOT implemented (text-only, no `png` or `all`)
- `--pages` flag NOT implemented (no per-page-slug capture)
- Auto-symlink `.roko/screenshots/latest/` NOT implemented
- Wire-up into `main.rs` top-level subcommand: CHECK if actually hooked in (need to verify)

**Maps to**: No existing backlog item. This is new.

**Suggested backlog item**: `111-screenshot-command-completion.md`

**Priority**: P1 | **Size**: S (1-2 days) — the skeleton is there; needs flags and symlink logic

---

### 0.3 — Continuous Screenshot Collection During Execution (`--screenshots` on `plan run`)

**Checklist description**: `roko plan run plans/ --screenshots` — auto-captures TUI at
startup, every N seconds, on plan-state/gate/agent/wave events, and completion. Produces
timestamped directory with `manifest.json` timeline.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers this. The `structured_log.rs` (untracked) covers structured
log output (item 0.6), but not screenshot capture during execution.

**Suggested backlog item**: `112-plan-run-continuous-screenshots.md`

**Priority**: P2 | **Size**: M (2-3 days) — hooks into event loop, requires snapshot engine (0.1/0.2)

---

### 0.4 — JSON Output Mode for CLI Commands (`--json` on status/doctor/learn/plan/agent)

**Checklist description**: Add `--json` flag to `roko status`, `roko doctor`, `roko learn all`,
`roko plan list`, `roko plan show`, `roko agent list`. Structured machine-readable output for
Claude/agents to consume programmatically.

**Coverage**: PARTIALLY COVERED by existing backlog items

- Backlog **100** (CLI Error Message Quality) touches CLI output but not JSON mode
- Backlog **77** (CLI UX Consistency) mentions output formatting consistency
- No backlog item specifically calls for `--json` flags on these commands

**Suggested backlog item**: `113-cli-json-output-mode.md`

**Priority**: P1 | **Size**: M (2-3 days) — five commands, each needs a JSON serialization path

---

### 0.5 — `roko diagnose <plan-id>` Command

**Checklist description**: Structured JSON diagnostic report for plan failures — failed task,
gate results, classified errors, git state, recovery suggestions, episode IDs, cost.

**Coverage**: IMPLEMENTED (untracked new file)

`crates/roko-cli/src/commands/diagnose.rs` exists and implements `cmd_diagnose()` with
the full report structure: `DiagnoseReport`, `FailedTaskInfo`, `GateResultInfo`,
`RunStateSummary`, `GitStateInfo`. Wire-up into `main.rs` needs verification.

**Gaps vs checklist spec**:
- `classified_errors` field with per-line error classification is NOT in the report
  (the spec shows `error_class`, `error_summary`, `suggestion` per error)
- `episode_ids` field NOT in the report
- `total_cost_usd` at plan level NOT in the report (only via `RunStateSummary`)

**Maps to**: Partially — no specific backlog item. Consider adding a completion spec.

**Suggested backlog item**: `114-diagnose-command-enrich.md` (minor enrichment only)

**Priority**: P2 | **Size**: XS (2-3 hours) — the bulk is done; add three fields

---

### 0.6 — Structured Log File Output (`--log-file` on `plan run`)

**Checklist description**: `roko plan run --log-file /tmp/roko.jsonl` writes per-event
JSONL (task start/complete/fail, agent spawn/death, gate pass/fail, wave progress) flushed
after each line for real-time tailing.

**Coverage**: IMPLEMENTED (untracked new file)

`crates/roko-cli/src/runner/structured_log.rs` implements `StructuredLogger` with
`open()`, `noop()`, and `log(event: &RunnerEvent)`. Wire-up into `event_loop.rs` and
the `--log-file` CLI flag need verification.

**Gaps vs checklist spec**:
- Need to verify `--log-file` flag is actually wired into `plan run` CLI args
- Need to verify `StructuredLogger.log()` is called at the right hook points in `event_loop.rs`
- Wave transition events may not be represented in `RunnerEvent` (wave system not yet built)

**Maps to**: No existing backlog item. Largely done; minimal gap.

**Suggested backlog item**: `115-structured-log-wire-verification.md`

**Priority**: P2 | **Size**: XS (1-2 hours) — verify wiring, add any missing call sites

---

### 0.7 — Screenshot Diffing and Assessment (`roko screenshot --compare`)

**Checklist description**: Text line-diff + PNG pixel-diff between two snapshot directories.
Produces JSON diff report + diff PNGs. Supports `--reference-mode` for mori screenshot comparison.

**Coverage**: NOT IMPLEMENTED

The `screenshot.rs` skeleton has `--compare` in the checklist spec but no implementation.
No existing backlog item covers this.

**Suggested backlog item**: `116-screenshot-diff-compare.md`

**Priority**: P3 | **Size**: M (2-3 days) — text diff is straightforward; PNG diff requires image dep

---

## Phase 1: Execution UX (Make It Work Like Mori)

### 1.1 — Queue Manifest (`.roko/queue.toml`)

**Checklist description**: TOML file defining milestone groups, plan execution order,
dependency chains (`depends_on`), per-run settings (`max_agents`, `mode`, `express`).
`roko plan run --queue .roko/queue.toml` feeds the ordered plan list into runner-v2.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers a queue manifest. Backlog item **103** (Plan Execution
Resilience) covers resilience of individual plan execution but not orchestration-level
queue ordering.

**Suggested backlog item**: `117-queue-manifest.md`

**Priority**: P2 | **Size**: L (3-5 days) — new TOML format, milestone graph, runner integration

---

### 1.2 — Wave Computation (Kahn's Algorithm for Plan-Level Parallelism)

**Checklist description**: Compute which plans can run in parallel based on `depends_on`.
Group plans into waves (Wave 0 = no deps, Wave N depends on Wave N-1). `roko plan list --waves`
shows grouping. Header shows `Wave N/M`.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers this. The task DAG computation exists at the
*task-within-plan* level in `runner/task_dag.rs`, but not at the *plan-across-plans* level.

**Suggested backlog item**: `118-plan-wave-computation.md`

**Priority**: P2 | **Size**: M (2-3 days) — Kahn's algorithm, already have task_dag.rs as reference

---

### 1.3 — Express Mode (`--express` on `plan run`)

**Checklist description**: Skip strategist/review agents and auto-fix on gate failure.
Auto-fix sequence: `cargo fix --allow-dirty` → auto-fixer agent with error digest → retry
up to 3 times → fail-forward.

**Coverage**: PARTIALLY COVERED by backlog **05** (Express Mode)

Backlog 05 covers the strategist-bypass logic (predicate + phase transition) but specifically
calls out `cargo fix --allow-dirty` as out of scope (see its "Out of Scope" section which
labels it "Gap 2"). The checklist treats both as the same feature.

**Gap**: Backlog 05 does NOT include the `--express` CLI flag, the auto-fixer agent dispatch,
or the retry-up-to-3-times logic. Only the strategist bypass predicate is specced.

**Maps to existing**: Backlog **05** (covers ~40% of the checklist's intent)

**Suggested backlog item**: `119-express-mode-autofix.md` (complement to 05)

**Priority**: P2 | **Size**: M (2-3 days) — `cargo fix` wiring + auto-fixer agent loop

---

### 1.4 — TUI Recovery Keybindings (5 operator actions)

**Checklist description**: `s` (soft retry), `z` (diagnose modal), `S` (repair with error
context), `R` (clean slate reset), `c` (reverify gates only). Context-sensitive hints in
status bar when a failed task is selected.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers TUI recovery keybindings. Backlog **73** (UX Backlog Rollup)
is a meta-item and backlog **107** (Plan Run UX Friction) mentions the TUI issue but doesn't
spec keybindings.

**Suggested backlog item**: `120-tui-recovery-keybindings.md`

**Priority**: P1 | **Size**: M (2-3 days) — `TuiAction` variants, channel-to-runner, status bar hints

---

### 1.5 — LLM Failure Reflections

**Checklist description**: On gate failure, fire a background fast-model call asking "What
failed? Why? What should the next attempt do differently?" Store in
`.roko/learn/reflections.jsonl`. Inject last 3 reflections into retry prompt.

**Coverage**: COVERED by backlog **15** (Post-Gate Reflection Loop)

Backlog 15 specifically specs the `spawn_reflection_agent` function, the prompt template,
deduplication, cost guard, and injection into retry. The scaffolding
(`post_gate_reflection.rs`, `lessons_from_post_gate_reflections`, `record_gate_failure_reflection`)
exists. The only missing piece (the actual LLM call) is exactly what backlog 15 tracks.

**Maps to**: Backlog **15** (full coverage)

---

### 1.6 — Preflight Checks at Plan-Run Startup

**Checklist description**: Run a subset of `roko doctor` checks before plan execution: valid
config, LLM credentials, disk space > 2GB, git state, valid plans dir, no stale locks, Rust
toolchain available. Exit on FAIL, continue on WARN.

**Coverage**: PARTIALLY COVERED by existing items

- Backlog **79** (Doctor/Onboarding Diagnostics) covers the doctor command itself
- Backlog **107** (Plan Run UX Friction) mentions stale worktrees silently blocking execution
- No backlog item specifically calls for a pre-run preflight gate inside `plan run`

**Suggested backlog item**: `121-plan-run-preflight-checks.md`

**Priority**: P1 | **Size**: S (1 day) — reuse doctor check logic, add startup gate in event_loop.rs

---

## Phase 2: TUI Quality (Make It Look Like Mori)

### 2.1 — Unify TUI Data Models (`DashboardData` + `TuiState` → single `TuiModel`)

**Checklist description**: Two parallel data models bridged by a conversion function cause
bugs. Create single `TuiModel` serving both standalone (disk) and connected (StateHub) modes.
Remove the bridge function.

**Coverage**: PARTIALLY COVERED by backlog **110** (Deprecate JSONL / StateHub-Only)

Backlog 110 (Phase 3 of its plan) calls for removing all JSONL readers from the TUI and
making StateHub the single source. This implicitly requires the data model unification but
doesn't spec it explicitly as a standalone step.

**Gap**: The structural unification of `DashboardData` and `TuiState` is not specced
as a discrete refactoring task. It's implied by 110 but needs its own scope.

**Suggested backlog item**: `122-tui-data-model-unification.md`

**Priority**: P2 | **Size**: L (3-5 days) — large refactor touching app.rs (4,576 LOC) and state.rs (5,290 LOC)

---

### 2.2 — Resolve Two Parallel Page Systems

**Checklist description**: `PageId`/`PageScaffold` (text mode) and `Tab`/`SubView` (ratatui)
overlap. Remove `PageId`/`PageScaffold` and keep only the ratatui system.

**Coverage**: NOT IMPLEMENTED

No existing backlog item specifically calls for removing the legacy page system. The issue
is mentioned in the architectural analysis docs but not as a concrete backlog item.

**Suggested backlog item**: `123-remove-legacy-page-system.md`

**Priority**: P2 | **Size**: S (1 day) — remove dead code, verify no rendering regressions

---

### 2.3 — Port ROSEDUST Color Palette

**Checklist description**: Replace default color scheme with Mori's warm rose-tinted grey
palette (true black bg, SAGE green, WARNING amber, EMBER red-orange, BONE, DREAM indigo,
no pure white, HSV gradient tables).

**Coverage**: PARTIALLY COVERED by backlog **71** (TUI Design System Alignment)

Backlog 71 mentions TUI design system alignment but at P3. The ROSEDUST palette port is a
concrete sub-task within that. The checklist gives much more specific color values and
implementation details.

**Maps to**: Backlog **71** (sub-task, not fully specced)

**Suggested backlog item**: Either expand 71 or create `124-rosedust-palette.md`

**Priority**: P2 | **Size**: S (1 day) — well-specified, contained to theme/colors module

---

### 2.4 — Port Information-Dense Header Bar

**Checklist description**: Rebuild header to match Mori's single-row density: pulsing dot,
app name, wave progress, queue name, 15-char gradient progress bar, ETA, elapsed, MCP count,
compact system metrics (CPU%/MEM/NET/DSK), F-key tab strip.

**Coverage**: PARTIALLY COVERED by backlog **108** (TUI Live Feedback Gaps)

Backlog 108 documents that the F1 header shows stale/zero data. But it doesn't spec the
redesign of the header layout to match Mori's density. The redesign is a new UX spec.

**Suggested backlog item**: `125-header-bar-mori-parity.md`

**Priority**: P2 | **Size**: M (2-3 days) — wave progress, ETA calc, gradient bar, system metrics

---

### 2.5 — Port Plan Tree Widget (Wave→Plan Hierarchy)

**Checklist description**: Rebuild plan tree to show collapsible waves with plans inside,
progress bars, health indicators, iteration counts, elapsed time, wave blocker chains.
Target: Mori's F1 left panel layout.

**Coverage**: NOT IMPLEMENTED as a distinct item

The plan list rendering exists in the TUI but the wave/milestone grouping hierarchy is
not implemented (because waves don't exist yet — see 1.2). This depends on 1.1 (queue
manifest) and 1.2 (wave computation).

**Suggested backlog item**: `126-plan-tree-wave-hierarchy-widget.md`

**Priority**: P2 | **Size**: M (2-3 days) — depends on 117/118; significant widget rewrite

---

### 2.6 — Port Error Digest Widget

**Checklist description**: New panel aggregating errors from gate output, pipeline errors,
preflight warnings, runtime issues. Panel border turns red when errors exist.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers an error digest aggregation panel. The TUI has individual
error displays per-tab but no unified cross-source digest widget.

**Suggested backlog item**: `127-error-digest-widget.md`

**Priority**: P2 | **Size**: S (1 day) — data already exists; widget + aggregation logic needed

---

### 2.7 — Adaptive Frame Rate

**Checklist description**: Drop from 60fps to ~20fps when agents are active and no user
input for 5 seconds. Resume 60fps on any key/mouse event.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers adaptive frame rate. Backlog **106** (Memory Allocation Hot
Paths) mentions render-loop efficiency in passing.

**Suggested backlog item**: `128-adaptive-frame-rate.md`

**Priority**: P3 | **Size**: XS (2-3 hours) — track last input time, toggle tick interval

---

### 2.8 — Exponential Smoothing on Metrics Display

**Checklist description**: Apply exponential smoothing (`alpha ~0.12 per frame`) to all
numeric displays (CPU%, token counts, progress %) to prevent jarring visual jumps.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers metric smoothing. This is a pure UX polish item.

**Suggested backlog item**: `129-metric-exponential-smoothing.md`

**Priority**: P3 | **Size**: XS (2-3 hours) — wrapper type or inline smoothing at render sites

---

### 2.9 — Context-Sensitive Keybind Hints

**Checklist description**: Status bar shows different hints depending on active tab AND
selection state (e.g., retry/diagnose/repair only when a failed task is selected on F2:plans).

**Coverage**: NOT IMPLEMENTED as discrete feature

Keybind hints exist but are not context-sensitive. No existing backlog item specs this.
This is a dependency of item 1.4 (recovery keybindings must exist first).

**Suggested backlog item**: Part of `120-tui-recovery-keybindings.md` or separate.

**Priority**: P2 | **Size**: XS (2-3 hours) — conditional rendering in status bar

---

### 2.10 — Content-Aware Tab Badges

**Checklist description**: Show counts on inactive tab labels (`e:Errors(3)`, `a:Agents(2)`)
so operators can see important counts without switching tabs.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers tab badge counts. Pure UX polish.

**Suggested backlog item**: `130-tab-content-badges.md`

**Priority**: P3 | **Size**: XS (1-2 hours) — read counts from TuiState, render in tab strip

---

## Phase 3: Observability (Single-Pane Inspect View)

### 3.1 — Single-Pane Inspect View (F7:inspect equivalent)

**Checklist description**: F7 tab showing MCP runtime status, AST index stats, and all
learning metrics (episodes, playbook, routing %, model stats, gate thresholds) in one
dense three-panel layout.

**Coverage**: NOT IMPLEMENTED as a single coherent view

The F7 tab currently exists but the data is fragmented. No existing backlog item specs
building the specific three-panel dense inspect layout from the mori reference screenshots.

The underlying data sources all exist:
- Episodes: `.roko/episodes.jsonl`
- Playbook: `.roko/learn/playbook.json`
- Routing: `.roko/learn/cascade-router.json`
- Gate thresholds: `.roko/learn/gate-thresholds.json`

**Suggested backlog item**: `131-inspect-view-f7-parity.md`

**Priority**: P2 | **Size**: M (2-3 days) — layout + data plumbing from existing sources

---

### 3.2 — Per-Model/Provider Cost Stats in TUI

**Checklist description**: F7 shows per-model and per-provider statistics: pass rate,
average duration, retry rate, tokens per run, cost per run. Data from `cascade-router.json`.

**Coverage**: PARTIALLY COVERED by backlog **13** (Historical Cost Calibration)

Backlog 13 covers calibration and cost tracking. The TUI display of per-model stats is
not specced separately but the data exists.

**Maps to**: Backlog **13** (data source), new for TUI display

**Suggested backlog item**: Part of `131-inspect-view-f7-parity.md`

---

### 3.3 — Full Prompt Text Logging

**Checklist description**: Store complete prompt text per invocation to
`.roko/learn/prompt-logs/{episode-id}.txt`. Configurable retention (keep last 100).

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers full prompt text archival. Backlog **80** (Learning Subsystem
Data Quality) discusses data quality but not raw prompt storage.

**Suggested backlog item**: `132-prompt-text-logging.md`

**Priority**: P2 | **Size**: XS (1-2 hours) — write after prompt assembly in event_loop.rs, add GC

---

## Phase 4: Self-Development Loop

### 4.1 — CLI Plan Control Commands (non-TUI)

**Checklist description**:
- `roko plan retry <plan-id> [task-id]` — retry failed plan/task from CLI
- `roko plan pause` / `roko plan resume` — pause/resume execution (currently TUI-only via `p`)
- `roko plan cancel <plan-id>` — cancel a running plan from CLI

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers these CLI control commands. The TUI has `p` for pause but
no CLI equivalents exist. These are the CLI-native analogs to the TUI recovery keybindings
(1.4), allowing Claude/agents to control execution without a terminal.

**Suggested backlog item**: `133-plan-cli-control-commands.md`

**Priority**: P1 | **Size**: M (2-3 days) — signal-based IPC to running plan run process

---

### 4.2 — Backlog-to-Plan Pipeline (`roko backlog import`)

**Checklist description**: Convert markdown backlog items into PRD ideas feeding the plan
generation pipeline. `roko backlog import tmp/backlog/100-cli-error-message-quality.md`
parses markdown, extracts title/description/acceptance criteria, creates a PRD idea,
optionally triggers draft + plan generation.

**Coverage**: NOT IMPLEMENTED

No existing backlog item covers this. The PRD pipeline (`roko prd idea → draft → plan`)
exists end-to-end, but no command to batch-import markdown specs into it.

**Suggested backlog item**: `134-backlog-import-command.md`

**Priority**: P2 | **Size**: M (2-3 days) — markdown parsing, PRD idea creation, pipeline trigger

---

### 4.3 — Automated Visual Assessment Loop

**Checklist description**: Wire `roko screenshot` into the self-development feedback loop.
Post-run screenshot capture, mori reference comparison, gap identification, plan generation,
execution with `--screenshots`, diff-based verification.

**Coverage**: NOT IMPLEMENTED (depends on 0.2 and 0.7)

This is the integration of all screenshot tooling into a workflow. It's primarily
documentation + convention rather than new code, but requires 0.2 and 0.7 to be
complete first.

**Suggested backlog item**: Part of `116-screenshot-diff-compare.md` or a workflow doc

**Priority**: P3 | **Size**: S (1 day) — mostly convention + minor `plan run` integration hooks

---

### 4.4 — Endpoint-Based Monitoring Documentation

**Checklist description**: Document the ~365 HTTP endpoint workflow for Claude to monitor
execution via `curl localhost:6677/api/...`. No new code, just documentation.

**Coverage**: NOT A CODE CHANGE

The routes exist. This is pure documentation. Skip as a code backlog item.

---

## Phase 5: Cleanup

### 5.1 — Decompose God Objects

**Checklist description**:
- Split `event_loop.rs` (~23K lines) into queue manager, agent dispatcher, gate runner, state persister, learning recorder, experiment manager, error handler
- Split `app.rs` (4,576 lines) into key handler, action dispatcher, I/O coordinator, snapshot manager
- Split `state.rs` (5,290 lines) into plan state, agent state, learning state, system metrics
- Split `dashboard.rs` (7,445 lines) into header, plan panel, agent panel, metrics panel, sub-tab manager

**Coverage**: `event_loop.rs` split is COVERED by backlog **20** (Event Loop Decomposition)

Backlog 20 is a full spec for the `event_loop.rs` extraction with six named targets
(A through F) and explicit acceptance criteria. It does NOT cover `app.rs`, `state.rs`,
or `dashboard.rs`.

**Gaps**:
- `app.rs` decomposition: NOT in any backlog item
- `state.rs` decomposition: NOT in any backlog item
- `dashboard.rs` decomposition: NOT in any backlog item

**Maps to**: Backlog **20** (event_loop.rs only)

**Suggested backlog items**:
- `135-tui-app-decomposition.md` (app.rs split)
- `136-tui-state-decomposition.md` (state.rs split)
- `137-tui-dashboard-decomposition.md` (dashboard.rs split)

**Priority**: P3 | **Size**: L (3-5 days each) — pure refactoring, no behavior changes

---

### 5.2 — Decide on Partial Cybernetic Features

**Checklist description**:
- **CorticalState** (2,717 LOC) — Wire into runner dispatch or remove
- **Inference Gateway** — Route runner-v2 through it or document as HTTP-only
- **Agent Groups coordination** — Implement coordination modes or remove enum variants
- **Cross-Cut Functors** — Wire into main dispatch or keep gate-failure-only

**Coverage**: PARTIALLY COVERED by existing items

- Cross-Cut Functors wiring is noted as "done" in CLAUDE.md
- Agent Groups: backlog **55** (AgentPool Runtime Integration) is adjacent but not identical
- CorticalState: partially covered by E23 (agent cognitive autonomy, 10/10 in GAPS.md)
- Inference Gateway: the inference gateway is documented as "Complete (E26 12/12)" in CLAUDE.md

**Gap**: No backlog item specifically targets CorticalState wire-or-remove decision or
the Agent Groups coordination mode audit.

**Suggested backlog item**: `138-cybernetic-feature-wire-or-remove-audit.md`

**Priority**: P3 | **Size**: M (2-3 days) — audit + wire or remove decision for each feature

---

### 5.3 — Remove Unused Code

**Checklist description**:
- Archive advanced HDC math (TDA, tropical algebra, sheaf Laplacian) — exists but unused
- Remove duplicate page system — after 2.2 is done

**Coverage**: PARTIALLY COVERED

- Duplicate page system removal is the same as checklist 2.2 above (new `123-remove-legacy-page-system.md`)
- Advanced HDC math archival: NOT in any backlog item. Backlog **19** (Contextual Bandit Dead Code)
  covers dead learning code but not HDC math specifically.

**Suggested backlog item**: `139-archive-unused-hdc-math.md`

**Priority**: P3 | **Size**: XS (1-2 hours) — move to archive or add `#[allow(dead_code)]` with a note

---

## Summary: New vs Covered Items

### Items ALREADY COVERED by existing backlog (no new spec needed)

| Checklist Item | Backlog # | Notes |
|---|---|---|
| 1.5 LLM Failure Reflections | **15** | Full spec exists, just needs the LLM call wired |
| 1.3 Express Mode (strategist bypass part) | **05** | Covers bypass; `cargo fix` + retry loop is new |
| 5.1 Split event_loop.rs | **20** | Comprehensive spec with 6 named targets |
| 3.2 Per-model cost stats (data) | **13** | Data source; TUI display part needs 131 |
| Phase 0 IPC for standalone dashboard | **110** | Phase 2 of that spec covers cross-process IPC |

### Items PARTIALLY IMPLEMENTED (new files exist, wire/flags missing)

| Checklist Item | Status | Action needed |
|---|---|---|
| 0.1 TUI Snapshot Mode (text) | snapshot.rs exists | Verify `--snapshot` flag on `dashboard`; add sub-view iteration |
| 0.2 `roko screenshot` command | screenshot.rs exists | Wire `--compare`, `--format`, `--pages`, symlink |
| 0.5 `roko diagnose` command | diagnose.rs exists | Add `classified_errors`, `episode_ids` fields |
| 0.6 Structured log file | structured_log.rs exists | Verify `--log-file` flag wired; verify hook call sites |

### NEW items not covered by any existing backlog

| # | Checklist Item | Suggested Backlog | Priority | Size |
|---|---|---|---|---|
| 111 | Screenshot command completion (flags/symlink) | `111-screenshot-command-completion.md` | P1 | S |
| 112 | Plan run continuous screenshots (`--screenshots`) | `112-plan-run-continuous-screenshots.md` | P2 | M |
| 113 | JSON output mode for CLI commands | `113-cli-json-output-mode.md` | P1 | M |
| 114 | Diagnose command enrichment (3 missing fields) | `114-diagnose-command-enrich.md` | P2 | XS |
| 115 | Structured log wire verification | `115-structured-log-wire-verification.md` | P2 | XS |
| 116 | Screenshot diff/compare (`--compare` flag) | `116-screenshot-diff-compare.md` | P3 | M |
| 117 | Queue manifest (`.roko/queue.toml`) | `117-queue-manifest.md` | P2 | L |
| 118 | Plan-level wave computation | `118-plan-wave-computation.md` | P2 | M |
| 119 | Express mode auto-fix (cargo fix + retry loop) | `119-express-mode-autofix.md` | P2 | M |
| 120 | TUI recovery keybindings (s/z/S/R/c) | `120-tui-recovery-keybindings.md` | P1 | M |
| 121 | Plan run preflight checks | `121-plan-run-preflight-checks.md` | P1 | S |
| 122 | TUI data model unification | `122-tui-data-model-unification.md` | P2 | L |
| 123 | Remove legacy page system | `123-remove-legacy-page-system.md` | P2 | S |
| 124 | ROSEDUST color palette port | `124-rosedust-palette.md` | P2 | S |
| 125 | Header bar Mori parity | `125-header-bar-mori-parity.md` | P2 | M |
| 126 | Plan tree wave hierarchy widget | `126-plan-tree-wave-hierarchy-widget.md` | P2 | M |
| 127 | Error digest widget | `127-error-digest-widget.md` | P2 | S |
| 128 | Adaptive frame rate | `128-adaptive-frame-rate.md` | P3 | XS |
| 129 | Metric exponential smoothing | `129-metric-exponential-smoothing.md` | P3 | XS |
| 130 | Content-aware tab badges | `130-tab-content-badges.md` | P3 | XS |
| 131 | Inspect view F7 parity (3-panel layout) | `131-inspect-view-f7-parity.md` | P2 | M |
| 132 | Full prompt text logging | `132-prompt-text-logging.md` | P2 | XS |
| 133 | Plan CLI control commands (retry/pause/cancel) | `133-plan-cli-control-commands.md` | P1 | M |
| 134 | Backlog import command | `134-backlog-import-command.md` | P2 | M |
| 135 | TUI app.rs decomposition | `135-tui-app-decomposition.md` | P3 | L |
| 136 | TUI state.rs decomposition | `136-tui-state-decomposition.md` | P3 | L |
| 137 | TUI dashboard.rs decomposition | `137-tui-dashboard-decomposition.md` | P3 | L |
| 138 | Cybernetic feature wire-or-remove audit | `138-cybernetic-feature-wire-or-remove-audit.md` | P3 | M |
| 139 | Archive unused HDC math | `139-archive-unused-hdc-math.md` | P3 | XS |

---

## Dependency Order (for the new items)

Items that must be done before others:

```
0.1 (snapshot engine)
  └→ 0.2 (screenshot command) [111]
       └→ 0.7 (screenshot diff) [116]
            └→ 4.3 (visual assessment loop) [part of 116]

1.1 (queue manifest) [117]
  └→ 1.2 (wave computation) [118]
       └→ 2.5 (plan tree wave widget) [126]

2.1 (data model unification) [122]
  └→ 2.2 (remove legacy page system) [123]

1.4 (recovery keybindings) [120]
  └→ 2.9 (context-sensitive keybind hints) [part of 120]

0.5 (diagnose command) [114]
  └→ 1.4 (diagnose modal in TUI keybindings) [120]
```

Items that are self-contained (no dependencies on new items):

- [111] Screenshot command completion
- [113] CLI JSON output mode
- [115] Structured log wire verification
- [119] Express mode auto-fix (builds on existing backlog 05)
- [121] Plan run preflight checks
- [124] ROSEDUST palette
- [127] Error digest widget
- [128] Adaptive frame rate
- [129] Metric smoothing
- [130] Tab badges
- [132] Prompt text logging
- [133] Plan CLI control commands
- [134] Backlog import command
- [138] Cybernetic feature audit
- [139] Archive unused HDC math
