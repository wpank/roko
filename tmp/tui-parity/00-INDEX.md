# TUI Parity: Consolidated Audit & Implementation Index

> **Post-merge correction (2026-08-31):** PR #74's “38/38 complete” summary is
> not supported by live verification. The merged baseline is 19 verified, 11
> partial/scaffolded, and 8 not operational. See the canonical
> [post-merge live audit](audits/post-merge-live-audit-2026-08-31.md). The
> checklist below is the original planning inventory, not completion evidence.

> **Visual follow-up:** The unusable Full-effects render was separately
> redesigned against Mori's current code: effects preserve operational text,
> layouts use content-first master/detail geometry, and empty Diagnosis chrome
> is collapsed. See [visual density and effects](audits/visual-density-effects.md).

> Consolidated 2026-08-31 from three parallel audit campaigns:
> - **v1 audit** (`tmp/tui-parity/archive/ux-audit/SUMMARY.md`): 19 agents, architecture + streaming + broken features
> - **v2 audit** (`tmp/tui-parity/archive/ux-audit/v2/00-MASTER-SUMMARY.md`): 15 agents, 130 issues with file:line evidence
> - **Parity checklist** (`tmp/tui-parity/archive/ux-mori-parity-audit/00-AUDIT-SUMMARY.md`): 53 items, 34 done / 17 partial / 2 not started

## The Verdict

Roko's TUI has 2x the code of mori (47K vs 24K LOC), more tabs (10 vs 7), and better
infrastructure (StateHub, adaptive framerate, braille sparklines, background git/sys
collection). But during a live `plan run`, it feels static and empty because of a small
number of specific wiring failures.

The gap is **concentrated, not systemic**. Five root causes explain 80% of the problem,
and P0 alone would transform the experience.

---

## The Five Root Causes

### RC-1: Two disconnected data models

The TUI maintains two parallel data paths that are never synchronized in connected mode:
`DashboardData` (file-based, initialized empty during `plan run`) and `DashboardSnapshot`
(event-driven, populated via StateHub). Token sparklines, efficiency panels, learning views,
and cost-by-model all read from the empty `DashboardData`. The header bar works because it
reads from `TuiState`.

**Impact**: Token sparkline zeros, efficiency "waiting for data...", cost-by-model empty.
**Fix**: Make all widgets read from `TuiState` first, fall back to `DashboardData`. ~200 LOC.

### RC-2: Recovery keybindings are facade-only

Every recovery action (s/z/S/R/c/F/V) writes a JSON entry to `.roko/engrams.jsonl` and shows
a toast. No component reads these signals. The runner has no command channel from TUI.
Similarly, `p` (pause) flips a visual flag the runner never reads.

**Impact**: "Confirmed: Soft-retry plan 'foo'" but nothing happens.
**Fix**: Add `mpsc::Sender<TuiCommand>` from App to the runner event loop. ~300 LOC.

### RC-3: Gate output is stripped before reaching TUI

Gate execution captures full cargo/test stdout/stderr in `state.gate_output`, but
`DashboardEvent::GateResult` carries only plan_id/task_id/gate/passed -- the raw output
is discarded. There is no live streaming of gate execution.

**Impact**: During the 30-120s that gates run, the TUI shows nothing.
**Fix**: Forward `GateCompletion.output` through events; build `GateOutputWidget`. ~500 LOC.

### RC-4: Disk I/O in render path

Three render functions read files on every frame (20-60 fps): MCP sub-tab reads
`roko.toml` + MCP config, F7 Inspect reads `mcp-stats.json`/`playbook.json`, and F6 Config
parses `roko.toml` via TOML on every frame.

**Impact**: Up to 120 file reads per second. Performance degradation on HDD.
**Fix**: Cache in `TuiState`, refresh on fs-watcher signal or timer. ~100 LOC.

### RC-5: Built-but-not-rendered features

Several features have complete state management and input handling but the render layer
never uses them: log search, plan tree filter, critical path ETA, three-panel inspect
(unreachable sub-tab), and F3 role tabs (decorative only).

**Fix**: Wire render to state. Each is ~50-150 LOC.

---

## Priority Checklist

### P0: Fix the "dead TUI during plan run" problem (~1.75d)

Fixes RC-1. Makes the TUI actually show live data during `plan run`.

- [ ] **P0.1** `#121` Make token sparkline read from `TuiState` when `DashboardData` is empty (0.5d) -- v2/06
- [ ] **P0.2** `NEW-001` Add `tasks_total` to `DashboardEvent::PlanStarted` so progress bar works (0.25d) -- v2/06
- [ ] **P0.3** `NEW-002` Fix cost_usd event ordering race (agent_completed before cost event) (0.25d) -- v2/06
- [ ] **P0.4** `NEW-003` Populate `token_rate`/`token_history` in `update_from_dashboard_snapshot()` (0.25d) -- v2/06
- [ ] **P0.5** `#121` Bridge learning/efficiency sparkline data to connected-mode `TuiState` (0.5d) -- v2/01, v2/14

### P1: Make interaction actually work (~2.5d)

Fixes RC-2 and RC-5. Makes keybindings do something real.

- [ ] **P1.1** `#119` Add TUI-to-runner command channel (Pause/Resume/Retry/Skip) (1d) -- v2/09, v2/15, parity/batch-4
- [ ] **P1.2** `#217` Wire log search render in `logs_view.rs` (0.5d) -- v2/07, parity/batch-4
- [ ] **P1.3** `#219` Wire plan tree filter render (0.5d) -- v2/01, parity/batch-4
- [ ] **P1.4** `NEW-004` Make F3 role tabs actually switch agent output (0.25d) -- v2/03
- [ ] **P1.5** `#196` Display `critical_path_eta_minutes` in header or plan view (0.25d) -- v2/01, parity/batch-2
- [ ] **P1.6** `NEW-005` Register F7 sub-tab 5 (three-panel inspect) so it's reachable (trivial) -- v2/07

### P2: Show gate execution (~1.75d)

Fixes RC-3. Makes gate phases visible.

- [ ] **P2.1** `NEW-006` Forward `GateCompletion.output` through `DashboardEvent` (0.5d) -- v2/11
- [ ] **P2.2** `NEW-007` Build `GateOutputWidget` with streaming cargo/test output (1d) -- v2/11, v1/SUMMARY
- [ ] **P2.3** `#108` Show live "gate rung X running" indicator during execution (0.25d) -- v2/11, parity/batch-1

### P3: Fix performance issues (~1.25d)

Fixes RC-4. Eliminates per-frame I/O.

- [ ] **P3.1** `NEW-008` Cache MCP config in `TuiState`, refresh on fs-watcher signal (0.5d) -- v2/04, v2/07
- [ ] **P3.2** `NEW-009` Cache config TOML parse, don't re-read every frame (0.5d) -- v2/13
- [ ] **P3.3** `NEW-010` Move F7 inspect file reads to background refresh cycle (0.25d) -- v2/07

### P4: Visual density and polish (~1.75d)

- [ ] **P4.1** `NEW-011` Shrink bottom ribbon from 6 rows to 4 (0.25d) -- v2/08
- [ ] **P4.2** `NEW-012` Replace 8 generic empty states with contextual messages (0.5d) -- v2/12
- [ ] **P4.3** `NEW-013` Add NET/DSK sampling to `collect_sys_metrics_bg` (0.25d) -- v2/14
- [ ] **P4.4** `NEW-014` Change effects default from `Off` to `Minimal` (trivial) -- v2/10, parity/batch-8
- [ ] **P4.5** `NEW-015` Style PAUSED indicator as inverted badge (fg/bg swap) (trivial) -- v2/10
- [ ] **P4.6** `NEW-016` Add persistent warning bar below header (0.5d) -- v2/05
- [ ] **P4.7** `NEW-017` Add MCP count, NET, DSK, FPS to header bar (0.25d) -- v2/05

### P5: Enrich plan detail (~2.0d)

Data already exists; just needs rendering.

- [ ] **P5.1** `NEW-018` Add `depends_on` to `TaskEntry` bridge type (0.5d) -- v2/02
- [ ] **P5.2** `NEW-019` Add `acceptance`/`verify` to `TaskEntry` (0.25d) -- v2/02
- [ ] **P5.3** `NEW-020` Add files-modified/diff stats to plan detail modal (0.5d) -- v2/02
- [ ] **P5.4** `NEW-021` Add branch/worktree/commit to plan detail (0.5d) -- v2/02
- [ ] **P5.5** `NEW-022` Add per-plan elapsed time (timer updates during run) (0.25d) -- v2/15

### P6: Keyboard model fixes (~1.5d)

- [ ] **P6.1** `NEW-023` Fix global number key shadowing per-tab handlers (0.25d) -- v2/09
- [ ] **P6.2** `NEW-024` Move `v` from effects cycle to verify (match mori) (trivial) -- v2/09
- [ ] **P6.3** `NEW-025` Add tab focus on remaining 7 tabs (1d) -- v2/09, v1/05
- [ ] **P6.4** `NEW-026` Update help overlay with correct bindings (0.25d) -- v2/09
- [ ] **P6.5** `NEW-027` Fix shared scroll state between Diff and Procs sub-tabs (trivial) -- v2/04

### P7: Sub-tab specific fixes (~0.75d)

- [ ] **P7.1** `NEW-028` Refresh `git_diff` in background watcher (currently loaded once) (0.25d) -- v2/04, v2/13
- [ ] **P7.2** `NEW-029` Differentiate Log vs Signals sub-tabs (currently identical) (0.25d) -- v2/07
- [ ] **P7.3** `NEW-030` Fix Procs sub-tab using wrong scroll state (trivial) -- v2/04
- [ ] **P7.4** `NEW-031` Add agent attempt/iteration count to output title (trivial) -- v2/03

### Parity-only items (from checklist, not in v2 priorities)

- [ ] **PX.1** `#130` Tab content badges -- add dynamic count API to Tab labels (XS-S) -- parity/batch-0
- [ ] **PX.2** `#139` Per-plan agent-handle map -- `HashMap<TaskId, AgentHandle>` in runner (M) -- parity/batch-7
- [ ] **PX.3** `#179` Batch controller -- remove stub, wire batch_size pause logic (S) -- parity/batch-4
- [ ] **PX.4** `#134` Replan escalation -- wire 4 `StructuralReplanStrategy` variants to retry count (S) -- parity/batch-7
- [ ] **PX.5** `#178` Conductor supervisor -- wire Nudge/ForceAdvance beyond logging (M) -- parity/batch-7
- [ ] **PX.6** `#107` UX34: cascade router `--force-backend` override learning warning (XS) -- parity/batch-3
- [ ] **PX.7** `#57` Plan generate crash retry/escalation (prd path only; plan generate missing) (S) -- parity/batch-3
- [ ] **PX.8** `#100` CLI error recovery hints (285 remaining `eprintln!` without hints) (M) -- parity/batch-6
- [ ] **PX.9** `#223` Setup wizard -- convert from stdin-interactive to ratatui TUI (M) -- parity/batch-9
- [ ] **PX.10** `NEW-032` Progressive formality -- `roko undo` verb (S) -- parity/batch-9

### Medium-term items (from v1 audit)

- [ ] **MX.1** `NEW-033` GraphEventSink + GraphTuiAdapter (graph engine TUI integration) (2d) -- v1/19
- [ ] **MX.2** `NEW-034` Wire ApprovalChannel to TUI (approval flow end-to-end) (1d) -- v1/09
- [ ] **MX.3** `NEW-035` Enable Anthropic API streaming (`stream: true`) (1d) -- v1/07
- [ ] **MX.4** `NEW-036` ASCII DAG widget (graph topology visualization) (2d) -- v1/17
- [ ] **MX.5** `NEW-037` Fix 250ms async path keyboard latency (0.5d) -- v2/09, v1/16
- [ ] **MX.6** `NEW-038` Fix mouse hit-test zones (0.5d) -- v1/05
- [ ] **MX.7** `NEW-039` WCAG contrast fix for ROSEDUST palette (0.5d) -- v1/05

---

## What Roko Does BETTER Than Mori

Not everything is a gap. These are genuine improvements:

1. **Braille sparklines** -- 2x resolution vs mori's flat bars
2. **Background git collection** -- zero I/O in render path for git data
3. **Cost/budget tracking** -- per-plan, per-task, per-model cost display (mori had none)
4. **Gradient progress bars** -- smooth color transitions vs mori's single-color
5. **Semantic agent output parsing** -- 8 segment types vs mori's 5 (on F3)
6. **Adaptive frame rate** -- 60fps active, 20fps idle (vs mori's fixed 60fps)
7. **Config editor** -- full interactive editor with inline editing and Ctrl-S save
8. **Notification toasts** -- categorized, timed, dismissible (mori had basic)
9. **Learning sub-tab** -- C-factor sparklines, concluded experiments (mori had none)
10. **Provider health** -- circuit breaker state, per-model routing (mori had basic)
11. **Watcher-driven git data** -- 500ms debounced notify watcher (mori polled on timer)
12. **Bounded channels** -- 256-entry bounded with backpressure (mori used unbounded)
13. **Queue manifest + DAG** -- cross-plan DAG, wave computation, `roko plan list --waves`
14. **Plan preflight checks** -- 7 checks before execution (mori had none)

---

## File Index

| File | Contents |
|------|----------|
| `00-INDEX.md` | This file: master index, root causes, priority checklist, effort estimates |
| `01-IMPLEMENTATION-PLAN.md` | Phased implementation plan with file:line targets and verification steps |
| `audits/post-merge-live-audit-2026-08-31.md` | Live reproduction, crash root cause, and operational status of all 38 P0-P7 claims |
| `audits/` | Directory for individual audit report copies (from v1, v2, parity) |
| `plans/` | Directory for generated implementation plan TOMLs |
| `reference/` | Directory for mori reference material |

### Source audit locations

| Source | Path |
|--------|------|
| v2 audit (15 reports) | `tmp/tui-parity/archive/ux-audit/v2/01-*.md` through `15-*.md` |
| v1 audit (17 reports) | `tmp/tui-parity/archive/ux-audit/01-*.md` through `17-*.md` |
| Parity checklist (10 batches) | `tmp/tui-parity/archive/ux-mori-parity-audit/batch-0.md` through `batch-9.md` |
| Graph integration | `tmp/engine-audit/19-graph-tui-integration.md`, `20-graph-event-emission.md` |

---

## Effort Estimate

| Priority | Items | Days | Cumulative UX Quality |
|----------|-------|------|-----------------------|
| P0 (dead TUI fix) | 5 | 1.75 | 75% -> 82% |
| P1 (interaction) | 6 | 2.5 | 82% -> 87% |
| P2 (gate output) | 3 | 1.75 | 87% -> 90% |
| P3 (performance) | 3 | 1.25 | 90% -> 92% |
| P4 (visual) | 7 | 1.75 | 92% -> 93% |
| P5 (plan detail) | 5 | 2.0 | 93% -> 94% |
| P6 (keyboard) | 5 | 1.5 | 94% -> 95% |
| P7 (sub-tabs) | 4 | 0.75 | 95% -> 96% |
| PX (parity) | 10 | ~5.0 | |
| MX (medium-term) | 7 | ~7.5 | |
| **P0-P7 Total** | **38** | **~13d** | |
| **All items** | **55** | **~26d** | |

P0 alone would transform the experience. P0+P1+P2 (~6 days) would bring it to ~90% of
mori quality. All P0-P7 priorities (~13 days) would reach ~96%.
