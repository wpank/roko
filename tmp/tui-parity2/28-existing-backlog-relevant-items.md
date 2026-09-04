# Existing Backlog: TUI-Relevant Items

> Scanned 2026-09-01 from six source categories across `tmp/tui-parity/`,
> `.roko/GAPS.md`, `tmp/cli-audit/`, `tmp/engine-audit/`, and parity batch
> files. Each item is cross-referenced against the P0-P7 checklist to exclude
> already-completed work.

## Source inventory

| Source | Path | Items extracted |
|---|---|---|
| TUI parity index | `tmp/tui-parity/00-INDEX.md` | P0-P7 (38 items), PX (11), MX (10), perf addenda (2), long-term (7) |
| GAPS.md | `.roko/GAPS.md` | 9 TUI-specific entries, 3 backlog items |
| v1 UX audit | `tmp/tui-parity/archive/ux-audit/SUMMARY.md` | 3 critical gaps, 7 medium-term, 7 long-term |
| v2 UX audit | `tmp/tui-parity/archive/ux-audit/v2/00-MASTER-SUMMARY.md` | 5 root causes, 130 issues (9 critical) |
| Parity checklist | `tmp/tui-parity/archive/ux-mori-parity-audit/00-AUDIT-SUMMARY.md` | 17 partial, 2 not started (of 53 total) |
| Post-merge live audit | `tmp/tui-parity/audits/post-merge-live-audit-2026-08-31.md` | 9 partial, 8 not operational (of 38 P0-P7) |
| Visual density audit | `tmp/tui-parity/audits/visual-density-effects.md` | 3 unresolved visual items |
| CLI audit | `tmp/cli-audit/SUMMARY.md` | 2 TUI-adjacent findings |
| Engine audit | `tmp/engine-audit/SUMMARY.md` | 2 TUI integration findings |

---

## Completed (P0-P7): excluded from this report

These items are marked done in the tracker and confirmed by source evidence.
They are listed here only as a deduplication reference.

| ID | Item | Status |
|---|---|---|
| P0.1 | Token sparkline reads from TuiState | Verified |
| P0.2 | Plan task denominator idempotent | Source-fixed |
| P0.3 | Cost ordering race | Verified |
| P0.4 | Connected token rate/history | Source-fixed |
| P1.2 | Log search render | Verified |
| P1.3 | Plan tree filter render | Verified |
| P1.4 | F3 role tabs switch output | Verified |
| P1.6 | F7 sub-tab 5 reachable | Verified |
| P3.2 | Cache config TOML parse | Verified |
| P3.3 | Cache Inspect file reads | Verified |
| P4.1 | Four-row bottom ribbon | Verified |
| P4.2 | Contextual empty states | Verified |
| P4.3 | NET/DSK sampling | Verified |
| P4.4 | Effects default Minimal | Verified (overflow also fixed) |
| P4.5 | PAUSED badge styling | Verified (visual only) |
| P4.6 | Warning bar | Verified |
| P4.7 | Header MCP/NET/DSK/FPS | Verified |
| P6.1 | Number key shadowing fix | Verified |
| P6.2 | `v` moved to verify | Verified |
| P7.2 | Log vs Signals differentiated | Verified |
| P7.4 | Agent attempt/iteration in title | Verified |

---

## Category 1: Bugs

Items where existing behavior is incorrect, crashes, or misleads the user.

### BUG-01: MCP config read on every render frame (P3.1 partial)

- **Source:** v2/04, v2/07, v2/13; post-merge audit; 00-INDEX P3.1
- **Status:** Partial -- F6 config and F7 inspect are cached, but the MCP
  sub-tab still calls `Config::from_file` and `McpConfig::load` inside render.
- **Impact:** Up to 60 file reads/second on the MCP panel. Performance
  degradation on HDD or during config writes.
- **Fix:** Cache MCP config in TuiState, refresh on fs-watcher signal or
  timer. ~50-100 LOC.

### BUG-02: Shared scroll state between Diff and Procs sub-tabs (P6.5)

- **Source:** v2/04; 00-INDEX P6.5
- **Status:** Not operational. Both sub-tabs share the same scroll offset,
  so scrolling in Diff moves Procs and vice versa.
- **Fix:** Separate scroll state per sub-tab. Trivial.

### BUG-03: git_diff loaded once, never refreshed (P7.1)

- **Source:** v2/04; 00-INDEX P7.1
- **Status:** Not operational. The Diff sub-tab loads git diff at startup
  and never updates, even as agents modify files.
- **Fix:** Refresh in the background watcher cycle. 0.25d.

### BUG-04: Procs sub-tab uses wrong scroll state (P7.3)

- **Source:** v2/04; 00-INDEX P7.3
- **Status:** Not operational. Same root cause as BUG-02.
- **Fix:** Dedicate scroll state per sub-tab. Trivial.

### BUG-05: Help overlay shows stale keybindings (P6.4)

- **Source:** v2/09; 00-INDEX P6.4
- **Status:** Partial. Help overlay still shows old effects-cycle keys
  and recovery action claims that do not match actual behavior.
- **Fix:** Update the help text to reflect current bindings. 0.25d.

### BUG-06: Mouse hit-test zones mismatch rendered areas (MX.6)

- **Source:** v1/05; 00-INDEX MX.6
- **Status:** Broken. Click targets do not align with the rendered widget
  positions, causing incorrect tab/panel selection on mouse click.
- **Fix:** Recalculate hit-test rects from actual render layout. 0.5d.

### BUG-07: Per-frame Theme::from_env() syscalls

- **Source:** v1/16 (performance section); 00-INDEX cross-reference 3.
- **Status:** Not tracked in P0-P7. `Theme::from_env()` is called on every
  frame, issuing syscalls to read `ROKO_HIGH_CONTRAST` and `NO_COLOR`.
- **Fix:** Cache the result once at startup or on SIGHUP. Trivial.

### BUG-08: Vec::remove(0) on history buffers

- **Source:** v1/16 (performance section); 00-INDEX cross-reference 3.
- **Status:** Not tracked in P0-P7. History buffers use `Vec::remove(0)`
  which is O(n) instead of VecDeque O(1).
- **Fix:** Replace with VecDeque. Trivial.

### BUG-09: Unbounded memory growth in IncrementalTailer and TuiState vectors

- **Source:** v1/16 (performance section); 00-INDEX cross-reference 3.
- **Status:** Not tracked. `IncrementalTailer::items` and several TuiState
  vectors grow without bound. Fine for typical runs, problem for 24/7
  `roko dashboard` monitoring.
- **Fix:** Ring buffer or periodic trim. S.

---

## Category 2: Missing Features

Items where functionality does not exist or the producer is absent.

### FEAT-01: TUI-to-runner command channel incomplete (P1.1)

- **Source:** v2/09, v2/15; parity batch-4; 00-INDEX P1.1; GAPS.md RC-2
- **Status:** Partial. The enum and channel exist. `p` (pause) flips a
  local UI flag the runner never reads. Soft-retry, repair, reverify,
  and skip handlers mostly only log. Cancel is process-wide abort.
- **What works:** Channel plumbing, TUI-side enum dispatch.
- **What does not:** Runner-side consumption of Pause, Resume, Retry,
  Skip. The recovery keybindings (s/z/S/R/c) write to engrams.jsonl
  but the runner never polls those signals.
- **Impact:** The entire TUI recovery/control workflow is non-functional.
- **Fix:** Wire runner `select!` branch to poll the command channel and
  act on each variant. ~300 LOC, 1d.

### FEAT-02: Critical-path ETA not produced (P1.5)

- **Source:** v2/01; parity batch-4 #196; post-merge audit; 00-INDEX P1.5
- **Status:** Not operational. The `critical_path_eta_minutes` field and
  header display branch exist, but no production code ever assigns a value.
- **Fix:** Call `remaining_eta_minutes()` from the snapshot update path
  and write to `tui_state.critical_path_eta_minutes`. 0.25d.

### FEAT-03: Gate output not forwarded to TUI (P2.1)

- **Source:** v2/11; post-merge audit; 00-INDEX P2.1; GAPS.md RC-3
- **Status:** Not operational. `TuiBridge::gate_output_line()` exists but
  has no production caller. `gate_result()` always sends
  `output_text: None`. Raw cargo/test stdout/stderr is captured in
  `state.gate_output` but discarded before reaching DashboardEvent.
- **Fix:** Forward `GateCompletion.output` through DashboardEvent. 0.5d.

### FEAT-04: GateOutputWidget not fed by real gate (P2.2)

- **Source:** v2/11; post-merge audit; 00-INDEX P2.2; GAPS.md RC-3
- **Status:** Partial. The colorized widget and snapshot ring exist, but
  no gate producer feeds them, so it cannot stream a real run. During the
  30-120s that gates run, the TUI shows nothing.
- **Fix:** Build streaming line bridge from gate_dispatch to the widget
  ring. 1d.

### FEAT-05: No live gate-rung indicator (P2.3)

- **Source:** v2/11; post-merge audit; 00-INDEX P2.3
- **Status:** Not operational. A start event is logged, but
  `TuiState.current_gate_rung` is never assigned or cleared. No "rung X
  running" visual appears during gate execution.
- **Fix:** Assign/clear `current_gate_rung` on gate rung start/complete. 0.25d.

### FEAT-06: Connected learning/efficiency bridge incomplete (P0.5)

- **Source:** v2/06, v2/14; post-merge audit; 00-INDEX P0.5
- **Status:** Partial. Some JSON and aggregate fields are copied and
  cost-by-model has a fallback, but typed live efficiency/learning event
  data remains empty or incomplete in connected mode.
- **Fix:** Bridge typed learning events through DashboardEvent variants
  into TuiState. 0.5d.

### FEAT-07: Task dependencies not in plan detail modal (P5.1)

- **Source:** v2/02; post-merge audit; 00-INDEX P5.1
- **Status:** Not operational. `TaskEntry` has only id/name/status/agent_id.
  No `depends_on` field reaches the modal.
- **Fix:** Add `depends_on` to `TaskEntry` bridge type and populate from
  the plan DAG. 0.5d.

### FEAT-08: Acceptance/verify fields absent from plan detail (P5.2)

- **Source:** v2/02; post-merge audit; 00-INDEX P5.2
- **Status:** Not operational. `acceptance` and `verify` fields are absent
  from `TaskEntry` and the plan detail modal.
- **Fix:** Add fields to `TaskEntry`, populate from tasks.toml. 0.25d.

### FEAT-09: Files-modified/diff stats in plan detail (P5.3)

- **Source:** v2/02; post-merge audit; 00-INDEX P5.3
- **Status:** Partial. Modal rows and `PlanEntry` fields exist, but all
  production constructors set them to `None`.
- **Fix:** Populate from worktree diff stats. 0.5d.

### FEAT-10: Branch/worktree/commit in plan detail (P5.4)

- **Source:** v2/02; post-merge audit; 00-INDEX P5.4
- **Status:** Partial. Fields and modal rows exist, but production
  constructors leave them empty.
- **Fix:** Populate from runner worktree state. 0.5d.

### FEAT-11: Per-plan elapsed timer (P5.5)

- **Source:** v2/15; post-merge audit; 00-INDEX P5.5
- **Status:** Partial. Terminal run duration now freezes correctly, but
  per-plan start times still need proof.
- **Fix:** Track per-plan start/end times in TuiState. 0.25d.

### FEAT-12: Tab focus on remaining 7 tabs (P6.3)

- **Source:** v2/09; v1/05; 00-INDEX P6.3
- **Status:** Partial. Only F1, F2, F3 have working focus zone navigation.
  The remaining 7 tabs define FocusZone variants but do not consume focus
  in their input handlers.
- **Fix:** Wire focus-zone input dispatch for F4-F10. 1d.

### FEAT-13: Tab content badges (PX.1)

- **Source:** Parity batch-0 #130; 00-INDEX PX.1
- **Status:** Not started. `Tab::label()` returns `&'static str`. There is
  no dynamic count injection or `format!("Agents({})", n)` pattern.
- **Fix:** Add `fn label_with_counts(&self, state: &TuiState) -> String`
  or equivalent. XS-S.

### FEAT-14: Per-plan agent-handle map (PX.2)

- **Source:** Parity batch-7 #139; 00-INDEX PX.2
- **Status:** Not started. Only `agent_active: bool` and a total counter
  exist. No `HashMap<TaskId, AgentHandle>` for targeted per-task
  cancellation or accurate double-dispatch detection.
- **Fix:** Add handle map, insert on spawn, remove on completion. M.

### FEAT-15: Batch controller (PX.3)

- **Source:** Parity batch-4 #179; 00-INDEX PX.3
- **Status:** Partial. CLI flag, config field, and event types exist. The
  event loop variable `_completed_since_batch_pause` is prefixed with `_`
  and never incremented or compared.
- **Fix:** Remove underscore, increment on plan completion, compare to
  `config.batch_size`, pause when threshold reached. S.

### FEAT-16: Replan escalation ladder (PX.4)

- **Source:** Parity batch-7 #134; 00-INDEX PX.4
- **Status:** Partial. Four `StructuralReplanStrategy` variants are
  defined but never selected based on retry count. Only two coarse
  strategies (`Decompose`, `RetryWithEscalation`) are used.
- **Fix:** Wire retry-count-based strategy selection. S.

### FEAT-17: Conductor supervisor actions (PX.5)

- **Source:** Parity batch-7 #178; 00-INDEX PX.5
- **Status:** Partial. Nudge and ForceAdvance log only. No agent injection
  or DAG mutation occurs. Opt-in only; not active without conductor config.
- **Fix:** Wire Nudge to inject context, ForceAdvance to mutate DAG. M.

### FEAT-18: Cascade router --force-backend warning (PX.6)

- **Source:** Parity batch-3 #107; 00-INDEX PX.6; GAPS.md UX34
- **Status:** Not done. No warning when `--force-backend` bypasses learned
  routing. The cascade router silently skips learning.
- **Fix:** Emit warning when force_backend is set. XS.

### FEAT-19: Plan generate crash retry/escalation (PX.7)

- **Source:** Parity batch-3 #57; 00-INDEX PX.7
- **Status:** Partial. `roko prd plan` has crash retry and model escalation
  (3 retries + escalation chain). `roko plan generate` has none -- a single
  `run_agent_logged` call with no retry loop.
- **Fix:** Port the retry/escalation logic from prd.rs to plan.rs. S.

### FEAT-20: CLI error recovery hints (PX.8)

- **Source:** Parity batch-6 #100; 00-INDEX PX.8
- **Status:** Partial. Recovery hints exist on major terminal failure paths
  but 285 `eprintln!` calls remain without hints, 157 in `commands/` alone.
- **Fix:** Audit and add hints to remaining error paths. M.

### FEAT-21: Setup wizard TUI conversion (PX.9)

- **Source:** Parity batch-9 #223; 00-INDEX PX.9
- **Status:** Partial. The wizard works as a stdin-interactive text flow.
  The spec called for a ratatui TUI, not text prompts.
- **Fix:** Rewrite setup wizard as a ratatui modal/flow. M.

### FEAT-22: Progressive formality -- `roko undo` (PX.10)

- **Source:** Parity batch-9; 00-INDEX PX.10
- **Status:** Not done. 4/5 verbs implemented (do/think/show/tune). No
  `Undo` variant, handler, or spec implementation.
- **Fix:** Design and implement `roko undo` semantics. S.

### FEAT-23: Sub-tab navigation consistency (PX.11)

- **Source:** v1/11 (cross-tab issues); 00-INDEX cross-reference 3
- **Status:** Not tracked until the UX audit cross-reference. Sub-tab
  navigation uses letter keys on F1 but number keys elsewhere. No
  cross-tab drill-down navigation. Inconsistent interaction model.
- **Fix:** Standardize sub-tab navigation across all tabs. S.

### FEAT-24: AgentOutputWidget (MX.8)

- **Source:** v1/SUMMARY.md "Critical UX Gap 1"; 00-INDEX cross-reference 3
- **Status:** Not tracked as a standalone item. The tracker addresses RC-1's
  data-bridge side (P0 fixes) but not the rendering cause. Current agent
  output is ~120 LOC of raw text lines vs mori's 1,679 LOC
  `agent_output.rs` with semantic parsing, render cache, per-agent tabs,
  pin/unpin auto-scroll, and contextual empty states.
- **Impact:** 60%+ of operator attention during live runs. The v1 audit
  assigns 40% of the quality gap to this single cause.
- **Fix:** Build dedicated `AgentOutputWidget` (~1,200 LOC). 2-3d.

### FEAT-25: Graph-to-TUI event integration (MX.1)

- **Source:** v1/SUMMARY.md; engine-audit/19, 20; 00-INDEX MX.1; GAPS.md
- **Status:** Zero integration. The graph engine emits events to a separate
  `TelemetryEventSink` with no StateHub/DashboardEvent bridge. TUI never
  sees graph execution. ~55 StateHub calls in runner-v2 vs 0 in graph engine.
- **Fix:** Build GraphEventSink (~250 LOC) and GraphTuiAdapter (~500 LOC);
  pass runner's live hub instead of isolated `new_in_process()`. 2d.

### FEAT-26: Approval TUI wiring (MX.2 / MX.10)

- **Source:** v1/09; 00-INDEX MX.2; GAPS.md; parity audit
- **Status:** Architecturally complete, substantively disconnected. Four
  specific disconnections: (1) `RunConfig.approval` is dead code, (2)
  ApprovalChannel is never created, (3) BatchReview modal has no input
  handler, (4) graph engine rejects approval in TTY terminals.
- **Fix:** Wire all four disconnection points. 1-2d.

### FEAT-27: Anthropic API streaming (MX.3)

- **Source:** v1/07; 00-INDEX MX.3; GAPS.md
- **Status:** The Anthropic adapter never sends `stream: true`. All
  Anthropic API calls are batch-only. This is the biggest provider
  streaming gap.
- **Fix:** Enable `stream: true` on the Anthropic Messages API path. 1d.

### FEAT-28: ASCII DAG widget (MX.4)

- **Source:** v1/17; 00-INDEX MX.4
- **Status:** Not built. No graph topology visualization exists in the TUI.
- **Fix:** Build ASCII DAG renderer (~650 LOC). 2d.

### FEAT-29: Provider streaming parity (MX.9)

- **Source:** v1/07 (streaming architecture); 00-INDEX cross-reference 3
- **Status:** Not tracked. Provider-by-provider streaming quality varies:
  OpenAI-compat (real-time SSE), Claude CLI (burst-y), Anthropic API
  (batch-only), Gemini/Cerebras/Ollama (batch-only).
- **Fix:** Audit and document per-provider streaming plan. M.

### FEAT-30: WCAG contrast fix for ROSEDUST palette (MX.7)

- **Source:** v1/05; 00-INDEX MX.7
- **Status:** ROSEDUST palette has insufficient contrast ratios for WCAG
  compliance. `ROKO_HIGH_CONTRAST` env var exists but the default palette
  fails accessibility checks.
- **Fix:** Adjust default ROSEDUST colors for minimum 4.5:1 contrast. 0.5d.

### FEAT-31: 250ms async path keyboard latency (MX.5)

- **Source:** v1/16; v2/09; 00-INDEX MX.5
- **Status:** The sync path has immediate redraw (excellent). The async
  `run()` path hardcodes 250ms poll (~4fps), causing up to 250ms keyboard
  latency during `plan run`.
- **Fix:** Reduce async poll interval or use event-driven wakeup. 0.5d.

---

## Category 3: Visual Improvements

### VIS-01: Legacy page scaffold removal (#122)

- **Source:** Parity batch-0 #122
- **Status:** Partial. `PageId`/`PageScaffold` types and `current_page`
  field remain as a parallel legacy rendering path alongside the Mori-style
  F1-F10 tab system. Intentionally kept for "text-mode compatibility."
- **Fix:** Remove legacy page path if text-mode is no longer needed. S.

### VIS-02: Triple-bordered left panel (VD-03)

- **Source:** visual-density-effects.md VD-03
- **Status:** Unresolved. Three stacked bordered panels in the Dashboard
  left column consume 6 border rows out of ~18 available (33% chrome).
  Mori uses single-line horizontal rules between sections.
- **Fix:** One outer block with Borders::ALL, inner sections with
  Borders::TOP only. +4 rows recovered. S.

### VIS-03: No 256-color terminal fallback

- **Source:** v1/16 (terminal compat)
- **Status:** Not tracked. TUI uses 24-bit RGB only (ROSEDUST). No
  256-color or 16-color fallback for older terminals.
- **Fix:** Detect terminal capability and fall back gracefully. M.

### VIS-04: Daimon affect not visualized in context panels

- **Source:** GAPS.md backlog #10
- **Status:** Done as standalone view (`affect_view.rs`), but the Daimon
  affect state (PAD gauges, somatic markers) is not integrated into the
  dashboard or agent detail panels where operators would see it during
  live runs.
- **Fix:** Add compact PAD gauge to agent detail or dashboard context. S.

### VIS-05: Oversized dashboard_view.rs (2,400+ LOC)

- **Source:** v1/11 (cross-tab issues)
- **Status:** Not tracked. Single view file is too large for comfortable
  maintenance. Mori's equivalent was spread across smaller focused files.
- **Fix:** Extract sub-widgets or split by panel. M.

---

## Category 4: Performance

### PERF-01: MCP config per-frame I/O (same as BUG-01)

See BUG-01 above.

### PERF-02: Per-frame Theme::from_env() syscalls (same as BUG-07)

See BUG-07 above.

### PERF-03: Vec::remove(0) on history buffers (same as BUG-08)

See BUG-08 above.

### PERF-04: Unbounded memory growth (same as BUG-09)

See BUG-09 above.

### PERF-05: 250ms async poll latency (same as FEAT-31)

See FEAT-31 above.

---

## Category 5: Long-term / Vision (graph-first TUI)

These are from the v1 UX audit's "Longer-Term" section. They represent
product vision rather than immediate gaps. Tracked for reference only.

| ID | Item | Effort | Source |
|---|---|---|---|
| LT-01 | Three-column graph layout (DAG + detail + meta) | 1w | v1/17 |
| LT-02 | Streaming output docked inside graph nodes | 3d | v1/17 |
| LT-03 | Conditional edge visualization (fire/fade) | 2d | v1/17 |
| LT-04 | Hot Graph tick sparkline | 1d | v1/17 |
| LT-05 | Budget heatmap overlay | 1d | v1/17 |
| LT-06 | Activity replay VCR controls | 2d | v1/17 |
| LT-07 | Named surface projections (Workbench/Inbox/Canvas/Minimap/Autonomy) | 1w | v1/17; E37 |

---

## Summary statistics

| Category | Count | From P0-P7 unchecked | From PX/MX | New from audits |
|---|---:|---:|---:|---:|
| Bugs | 9 | 4 | 1 | 4 |
| Missing features | 31 | 12 | 12 | 7 |
| Visual improvements | 5 | 0 | 0 | 5 |
| Performance | 5 | 1 | 1 | 3 |
| Long-term vision | 7 | 0 | 0 | 7 |
| **Total unique items** | **50** | **17** | **14** | **19** |

Note: Performance items overlap with bugs (cross-referenced above), so the
unique total is 50, not 57.

### Priority ranking by impact

**Highest impact (transforms the experience):**
1. FEAT-01 -- TUI-to-runner command channel (recovery actions work)
2. FEAT-24 -- AgentOutputWidget (40% of quality gap)
3. FEAT-03/04/05 -- Gate output pipeline (visible during 30-120s gate runs)
4. FEAT-25 -- Graph-to-TUI events (graph engine visible)

**High impact (significant UX improvement):**
5. FEAT-27 -- Anthropic API streaming (real-time for primary provider)
6. FEAT-06 -- Connected learning/efficiency bridge
7. FEAT-02 -- Critical-path ETA
8. FEAT-26 -- Approval TUI wiring
9. FEAT-12 -- Tab focus on all tabs
10. BUG-01 -- MCP per-frame I/O

**Medium impact (polish and completeness):**
11-31: Plan detail enrichment (FEAT-07 through FEAT-11), batch controller,
replan escalation, conductor supervisor, tab badges, sub-tab consistency,
keyboard latency, WCAG contrast, error hints, provider streaming parity.

**Low impact (maintenance and vision):**
32-50: Legacy page removal, VecDeque fix, Theme caching, memory bounds,
256-color fallback, view file splitting, long-term graph vision items.

---

## Cross-reference with GAPS.md entries

The following GAPS.md entries are TUI-specific and map to items above:

| GAPS.md entry | Maps to |
|---|---|
| RC-1: Two disconnected data models | FEAT-06, FEAT-24 (partially addressed by P0.1-P0.4 done) |
| RC-2: Recovery keybindings facade-only | FEAT-01 |
| RC-3: Gate output stripped | FEAT-03, FEAT-04, FEAT-05 |
| RC-4: Disk I/O in render path | BUG-01 (P3.2, P3.3 done) |
| RC-5: Built-but-not-rendered | Partially resolved (P1.2-P1.4 done); FEAT-02 remains |
| Backlog #10: Daimon TUI view | VIS-04 (standalone done, integration pending) |
| Backlog #41: TUI push-mode panel data | Done (parity batch-5 #41) |
| Backlog #71: TUI design system alignment | VIS-02, VIS-03 |
| UX34: force_backend override learning | FEAT-18 |
| E37 named surface rendering | LT-07 |
