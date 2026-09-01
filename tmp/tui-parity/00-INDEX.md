# TUI Parity: Consolidated Audit & Implementation Index

> **Current implementation handoff (2026-09-01):** TUI work that intersects the CLI audit is scheduled in [`../cli-audit/IMPLEMENTATION-CHECKLIST.md`](../cli-audit/IMPLEMENTATION-CHECKLIST.md). It serializes #128/#365–#369 with #232–#237/#248/#266 and the current dirty TUI paths; do not dispatch directly from historical completion counts below.

> **Post-merge correction (2026-08-31):** PR #74's “38/38 complete” summary is
> not supported by live verification. The merged baseline is 19 verified, 11
> partial/scaffolded, and 8 not operational. Follow-up source fixes move P0.2 and
> P0.4 to implemented, yielding a **source status** of 21 implemented, 9 partial,
> and 8 not operational. The dev-audit integration additionally implements idle
> redraw/watcher bounds and terminal event/snapshot/task/PID/elapsed convergence,
> which are outside the original 38-item count. Panic-hook cleanup and post-FX
> overflow fixes are prepared for the final integration. Final interactive/live
> verification has not run, so the 21-source count must not be reported as a new
> live-verified count. See the canonical
> [post-merge live audit](audits/post-merge-live-audit-2026-08-31.md).

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

Checkboxes mean the requested source path exists in the current integration/handoff. They do not
replace the live status matrix or the coordinator's pending final interactive run. Unchecked items
are explicitly marked partial or not operational where the live audit found scaffolding.

### P0: Fix the "dead TUI during plan run" problem (~1.75d)

Fixes RC-1. Makes the TUI actually show live data during `plan run`.

- [x] **P0.1** `#121` Make token sparkline read from `TuiState` when `DashboardData` is empty (0.5d) -- v2/06
- [x] **P0.2** `NEW-001` Add/preserve `tasks_total` from `DashboardEvent::PlanStarted` so progress is idempotent (0.25d) -- follow-up source fix; final live check pending
- [x] **P0.3** `NEW-002` Fix cost_usd event ordering race (agent_completed before cost event) (0.25d) -- v2/06
- [x] **P0.4** `NEW-003` Populate cumulative connected token history/rate without zero-delta EMA corruption (0.25d) -- follow-up source fix; final live check pending
- [ ] **P0.5** `#121` Bridge learning/efficiency sparkline data to connected-mode `TuiState` (0.5d) -- **partial**; typed live learning events remain incomplete

### P1: Make interaction actually work (~2.5d)

Fixes RC-2 and RC-5. Makes keybindings do something real.

- [ ] **P1.1** `#119` Add TUI-to-runner command channel (Pause/Resume/Retry/Skip) (1d) -- **partial**; channel exists, several runner effects do not
- [x] **P1.2** `#217` Wire log search render in `logs_view.rs` (0.5d) -- v2/07, parity/batch-4
- [x] **P1.3** `#219` Wire plan tree filter render (0.5d) -- v2/01, parity/batch-4
- [x] **P1.4** `NEW-004` Make F3 role tabs actually switch agent output (0.25d) -- v2/03
- [ ] **P1.5** `#196` Display `critical_path_eta_minutes` in header or plan view (0.25d) -- **not operational**; no producer
- [x] **P1.6** `NEW-005` Register F7 sub-tab 5 (three-panel inspect) so it's reachable (trivial) -- v2/07

### P2: Show gate execution (~1.75d)

Fixes RC-3. Makes gate phases visible.

- [ ] **P2.1** `NEW-006` Forward `GateCompletion.output` through `DashboardEvent` (0.5d) -- **not operational**; bridge has no production caller
- [ ] **P2.2** `NEW-007` Build `GateOutputWidget` with streaming cargo/test output (1d) -- **partial**; widget is not fed by a real gate
- [ ] **P2.3** `#108` Show live "gate rung X running" indicator during execution (0.25d) -- **not operational**; state is not assigned/cleared

### P3: Fix performance issues (~1.25d)

Fixes RC-4. Eliminates per-frame I/O.

- [ ] **P3.1** `NEW-008` Cache MCP config in `TuiState`, refresh on fs-watcher signal (0.5d) -- **partial**; MCP panel still reads in render
- [x] **P3.2** `NEW-009` Cache config TOML parse, don't re-read every frame (0.5d) -- v2/13
- [x] **P3.3** `NEW-010` Move F7 inspect file reads to background refresh cycle (0.25d) -- v2/07

### P4: Visual density and polish (~1.75d)

- [x] **P4.1** `NEW-011` Shrink bottom ribbon from 6 rows to 4 (0.25d) -- v2/08
- [x] **P4.2** `NEW-012` Replace 8 generic empty states with contextual messages (0.5d) -- v2/12
- [x] **P4.3** `NEW-013` Add NET/DSK sampling to `collect_sys_metrics_bg` (0.25d) -- v2/14
- [x] **P4.4** `NEW-014` Change effects default from `Off` to `Minimal` (trivial) -- overflow regression source fix prepared; final live check pending
- [x] **P4.5** `NEW-015` Style PAUSED indicator as inverted badge (fg/bg swap) (trivial) -- visual only; pause semantics remain P1.1
- [x] **P4.6** `NEW-016` Add persistent warning bar below header (0.5d) -- v2/05
- [x] **P4.7** `NEW-017` Add MCP count, NET, DSK, FPS to header bar (0.25d) -- v2/05

### P5: Enrich plan detail (~2.0d)

Data already exists; just needs rendering.

- [ ] **P5.1** `NEW-018` Add `depends_on` to `TaskEntry` bridge type (0.5d) -- **not operational**
- [ ] **P5.2** `NEW-019` Add `acceptance`/`verify` to `TaskEntry` (0.25d) -- **not operational**
- [ ] **P5.3** `NEW-020` Add files-modified/diff stats to plan detail modal (0.5d) -- **partial**; renderer has no producer
- [ ] **P5.4** `NEW-021` Add branch/worktree/commit to plan detail (0.5d) -- **partial**; renderer has no producer
- [ ] **P5.5** `NEW-022` Add per-plan elapsed time (timer updates during run) (0.25d) -- **partial**; terminal run duration now freezes correctly, but per-plan starts still need proof

### P6: Keyboard model fixes (~1.5d)

- [x] **P6.1** `NEW-023` Fix global number key shadowing per-tab handlers (0.25d) -- v2/09
- [x] **P6.2** `NEW-024` Move `v` from effects cycle to verify (match mori) (trivial) -- v2/09
- [ ] **P6.3** `NEW-025` Add tab focus on remaining 7 tabs (1d) -- **partial**; several zones do not consume focus
- [ ] **P6.4** `NEW-026` Update help overlay with correct bindings (0.25d) -- **partial**; stale effects/recovery claims remain
- [ ] **P6.5** `NEW-027` Fix shared scroll state between Diff and Procs sub-tabs (trivial) -- **not operational** input path

### P7: Sub-tab specific fixes (~0.75d)

- [ ] **P7.1** `NEW-028` Refresh `git_diff` in background watcher (currently loaded once) (0.25d) -- **not operational**
- [x] **P7.2** `NEW-029` Differentiate Log vs Signals sub-tabs (currently identical) (0.25d) -- v2/07
- [ ] **P7.3** `NEW-030` Fix Procs sub-tab using wrong scroll state (trivial) -- **not operational** input path
- [x] **P7.4** `NEW-031` Add agent attempt/iteration count to output title (trivial) -- v2/03

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

### Dev-audit integration follow-ups (outside the 38-item denominator)

- [x] Connected TUI uses configured low-frequency refresh, redraws only on visible state changes,
  and avoids the broad `.roko` watcher when StateHub is authoritative (`88c724744`).
- [x] Recursive fallback watching excludes worktrees, targets, caches, and archives (`88c724744`).
- [x] Run completion freezes elapsed/ETA and converges terminal plan/agent/PID state while
  preserving cleanup-degraded ownership (`52d5f4df4`).
- [x] Replayed/duplicate plan, task, and agent lifecycle events update counters idempotently
  (`52d5f4df4`).
- [x] Panic-hook cleanup and post-processing seed overflow fixes are source-complete in the active
  handoff; integration and final debug-profile live verification remain pending.
- [ ] Run the final interactive active-agent/effects/gate/terminal smoke from the rebased tree and
  attach its logs/screenshots before upgrading any source-only claim to live verified.

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

## Cross-Reference: UX Audit (2026-09-01)

> Cross-referencing the v1 UX audit (`tmp/tui-parity/archive/ux-audit/SUMMARY.md`,
> dated 2026-08-31, 19 parallel agents) against this consolidated tracker. The v1 audit
> assessed roko at 75% of mori quality and identified 3 critical gaps, graph engine
> integration, approval TUI disconnection, streaming architecture issues, and prioritized
> fixes (P0-P6 quick wins, M1-M7 medium-term).

### 1. Root cause mapping

Each of the five root causes identified by the v2 consolidation (and carried into this
index) has a direct counterpart in the v1 UX audit:

| Root Cause | v1 UX Audit Coverage | Alignment |
|---|---|---|
| **RC-1**: Two disconnected data models | "The Three Critical UX Gaps" section 1 (agent output streaming) describes the same DashboardData/TuiState split. The "75% of mori quality" assessment and "dead TUI during plan run" diagnosis both originate from the same observable failure: connected-mode widgets reading from empty DashboardData. The v1 audit attributes 40% of the quality gap to this single cause. | **Confirmed and quantified.** The v1 audit's framing as "agent output streaming" is a symptom-level description of what the v2 consolidation formalized as RC-1. The v1 P1 fix (build AgentOutputWidget, 1.5d) addresses the render-side half; the v2 P0 fixes (P0.1-P0.5) address the data-bridge half. Both are needed. |
| **RC-2**: Recovery keybindings facade-only | v1 "Critical UX Gap 3: Recovery Keybindings Do Nothing." Identical diagnosis: keys write to engrams.jsonl, runner never polls, pause flips a visual flag only. v1 estimates 0.5d (P0 quick win), v2 estimates 1d (P1.1). | **Confirmed.** v2 is more realistic on effort because it includes the full enum/channel/select!/handler scope that v1 calls "~200 LOC." |
| **RC-3**: Gate output stripped before TUI | v1 "Critical UX Gap 2: Gate Output Widget (Missing)." Same finding: DashboardEvent::GateResult carries only pass/fail, raw output discarded, no live streaming during 30-120s gate execution. v1 references mori's `command_output.rs` (218 LOC). | **Confirmed.** v2 P2.1-P2.3 expands the fix scope to include gate-rung indicators and streaming line events, which v1's "port mori widget" estimate (0.5d) does not cover. |
| **RC-4**: Disk I/O in render path | v1 "TUI Performance" section lists the same three offenders: MCP sub-tab, F7 Inspect, F6 Config. The v1 audit additionally documents `Vec::remove(0)` on history buffers and per-frame `Theme::from_env()` syscalls, neither of which appears in the v2 P3 items. | **Confirmed, v1 has additional detail.** The VecDeque and Theme::from_env fixes are not tracked in P3; they should be considered P3 extensions. |
| **RC-5**: Built-but-not-rendered features | v1 "Broken/Partially-Wired Features" table lists 8 items; v2 RC-5 lists 5. The v1 audit includes WebSocket agent streams (partial) and atmospheric effects (default off) which v2 tracks separately under P4.4 and MX.3. | **Confirmed, broader scope in v1.** |

### 2. Tui-parity items confirmed or superseded by the UX audit

**Confirmed by identical findings (no change to tracker):**

| Tracker Item | UX Audit Section |
|---|---|
| P0.1 token sparkline | v1 P0 quick win; v2 report 06/14 |
| P0.3 cost ordering race | v1 data flow analysis (report 06) |
| P1.2 log search render | v1 P3 quick win; "Log search: Built, not rendered" |
| P1.3 plan filter render | v1 P4 quick win; "Plan tree filter: Built, not rendered" |
| P1.4 role tabs switch output | v1 "Role tabs: Decorative only" |
| P1.5 critical-path ETA | v1 P6 quick win; "Critical path ETA: Computed, not shown" |
| P4.4 effects default | v1 P5 quick win; "Ships as EffectsPreset::Off" |
| P6.1 number key shadowing | v1 report 09 keyboard model |
| MX.5 250ms async path latency | v1 performance section: "up to 250ms keyboard latency" |
| MX.6 mouse hit-test zones | v1 "Mouse hit-test: Broken" |
| MX.7 WCAG contrast | v1 "WCAG contrast: Fails" (ROSEDUST palette) |

**Confirmed but with added context from the UX audit:**

| Tracker Item | UX Audit Addition |
|---|---|
| P0.5 learning/efficiency bridge | v1 quantifies this as part of the "40% of operator attention" agent output gap; the tracker treats it as a data-bridge item. The v1 framing suggests P0.5 has higher user impact than its P0 placement implies. |
| P1.1 TUI-to-runner command channel | v1 notes that mori has real end-to-end JSON-RPC approval flow, not just pause/retry. The tracker's command channel scope may need expansion to cover tool approval. |
| P2.2 gate output widget | v1 provides the mori reference: `command_output.rs` 218 LOC with animated spinner, pass/fail text detection, and sonar-ring empty state. The tracker's "partial" status reflects widget code that has no production caller. |
| MX.3 Anthropic API streaming | v1 streaming analysis confirms "the adapter never sends `stream: true`" and identifies this as the biggest provider gap. The tracker carries it at medium-term priority. |

**Superseded or refined by later work:**

| Tracker Item | Status |
|---|---|
| P0.2 plan task denominator | Source-fixed by the post-merge audit; v1 did not detect this because it preceded the PR #74 merge. |
| P0.4 connected token rate/history | Source-fixed by the post-merge audit; same timeline. |
| P4.1-P4.3, P4.5-P4.7 visual polish | Verified at merge by the post-merge audit; v1 did not assess these because they were implemented after the v1 audit ran. |

### 3. New items from the UX audit not in this tracker

The v1 UX audit identifies several gaps that have no corresponding P0-P7, PX, or MX item
in this tracker:

| New Finding | UX Audit Source | Severity | Proposed Tracker Placement |
|---|---|---|---|
| **Graph engine has zero TUI integration** | v1 "Graph Engine TUI Integration" section; engine-audit reports 19-20 | High | Already partially covered by MX.1 (GraphEventSink) and MX.4 (ASCII DAG), but the v1 audit's scope is broader: it calls for GraphTuiAdapter (~500 LOC), passing runner's live hub instead of isolated `new_in_process()`, and a three-column graph layout. Consider promoting to a dedicated Graph TUI section (GX.1-GX.4) rather than two MX items. |
| **Approval TUI architecturally disconnected** | v1 "Approval TUI" section | High | Covered by MX.2 but the v1 audit reveals four specific disconnections: (1) `RunConfig.approval` is dead code, (2) ApprovalChannel is never created, (3) BatchReview modal has no input handler, (4) graph engine rejects approval in TTY terminals. MX.2's "1d" estimate is optimistic for all four. |
| **AgentOutputWidget needed (1,200 LOC vs 120 LOC render)** | v1 "Critical UX Gap 1" | Critical | Not tracked as a standalone item. The tracker addresses RC-1's data-bridge side (P0.1-P0.5) but never proposes building the dedicated widget with semantic parsing, render cache, per-agent tabs, pin/unpin auto-scroll, and contextual empty states that mori's `agent_output.rs` (1,679 LOC) provides. **This is the largest untracked item.** Suggested: add as MX.8 or elevate to P-level given its 40% quality impact. |
| **Per-frame Theme::from_env() syscalls** | v1 "Known Performance Issues" | Low | No tracker item. Minor; could be appended to P3 as P3.4. |
| **Vec::remove(0) on history buffers** | v1 "Known Performance Issues" | Low | No tracker item. Should use VecDeque. Could be P3.5. |
| **Tab maturity ranking and cross-tab issues** | v1 "Tab Maturity Ranking" + "Cross-Tab Issues" | Medium | The tracker has per-item fixes but no explicit cross-tab consistency item. Issues: sub-tab navigation inconsistent (letter keys on F1, number keys elsewhere), no cross-tab drill-down, oversized view files (dashboard_view.rs 2,400+ LOC). Consider a PX.11 for sub-tab navigation consistency. |
| **Streaming provider gap table** | v1 "Streaming Architecture Assessment" | Medium | MX.3 covers Anthropic API streaming, but the v1 audit documents provider-by-provider streaming quality: OpenAI-compat (real-time SSE), Claude CLI (burst-y), Anthropic API (batch-only), Gemini/Cerebras/Ollama (batch-only). No tracker item addresses the broader provider streaming parity. |
| **Longer-term graph-first TUI vision (L1-L7)** | v1 "Longer-Term" section | Low (vision) | Entirely untracked. Seven items spanning 2-4 weeks: three-column graph layout, streaming output docked in nodes, conditional edge visualization, Hot Graph tick sparkline, budget heatmap, activity replay VCR, named surface projections. These are product vision, not immediate gaps. |
| **Memory growth in 24/7 monitoring** | v1 "TUI Performance" | Low | `IncrementalTailer::items` and several TuiState vectors grow unbounded. No tracker item. |
| **No 256-color terminal fallback** | v1 "Terminal compat" | Low | 24-bit RGB only (ROSEDUST). Not tracked. |

### 4. Updated overall status assessment

**Before this cross-reference**, the tracker reported:
- 21 source-implemented, 9 partial, 8 not operational (P0-P7, 38 items)
- 10 PX items, 7 MX items (55 total)
- 75% baseline quality, ~96% achievable with all P0-P7

**After integrating the UX audit findings:**

The 75% baseline and root cause analysis are fully confirmed by independent assessment.
The P0-P7 priority ordering is validated. However, the tracker has a significant blind spot:
the v1 audit's highest-impact finding -- that roko needs a dedicated AgentOutputWidget
comparable to mori's 1,679 LOC agent_output.rs -- is not represented as a tracked item.
The tracker addresses the data-flow cause (P0 fixes RC-1's bridge) but not the rendering
cause (agent output is 120 LOC of raw text lines vs mori's semantic parsing, render cache,
per-agent tabs, and contextual empty states).

**Revised item count:**

| Category | Before | After (with UX audit additions) |
|---|---|---|
| P0-P7 | 38 | 38 (unchanged) |
| PX (parity) | 10 | 11 (+PX.11 sub-tab nav consistency) |
| MX (medium-term) | 7 | 10 (+MX.8 AgentOutputWidget, +MX.9 provider streaming parity, +MX.10 approval TUI four-point fix) |
| Performance addenda | 0 | 2 (P3.4 Theme::from_env, P3.5 VecDeque history) |
| Long-term (graph vision) | 0 | 7 (L1-L7 from v1 audit, tracked for reference only) |
| **Total tracked** | **55** | **68** |

The quality trajectory is unchanged: P0-P2 (~6 days) reaches ~90%, all P0-P7 (~13 days)
reaches ~96%. The MX additions (AgentOutputWidget, approval TUI, provider streaming) are
the primary items that separate 96% from full mori feature parity.

**Key action items from this cross-reference:**

1. Decide whether MX.8 (AgentOutputWidget) should be elevated to P-level given the v1
   audit's "40% of operator attention" assessment.
2. Expand MX.2 (approval TUI) scope and effort estimate to cover all four disconnections
   identified by the v1 audit.
3. Add P3.4 and P3.5 to the performance section for the minor per-frame inefficiencies.
4. Consider whether the graph-first TUI vision (L1-L7) merits a dedicated section or
   remains reference-only.

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
| v1 audit (17 reports + summary) | `tmp/tui-parity/archive/ux-audit/01-*.md` through `17-*.md`; summary at `tmp/tui-parity/archive/ux-audit/SUMMARY.md` |
| Parity checklist (10 batches) | `tmp/tui-parity/archive/ux-mori-parity-audit/batch-0.md` through `batch-9.md` |
| Graph integration | `tmp/engine-audit/19-graph-tui-integration.md`, `20-graph-event-emission.md` |
| UX audit cross-reference | See `## Cross-Reference: UX Audit (2026-09-01)` section above |

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
