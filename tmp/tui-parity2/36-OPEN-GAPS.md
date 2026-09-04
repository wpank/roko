# Open UX/TUI gaps after the follow-up pass

**Date:** 2026-09-01 (updated 2026-09-02 swarm)
**Purpose:** shortest actionable queue after reconciling source, persisted run data, Mori/Bardo,
backlog acceptance criteria, and controlled full-frame captures.

## P0 — operator truth and control

0. **Mori-shaped workflow contract (#116/#117/#140/#138/#179/#249/#272).** Treat the complete
   workflow as one acceptance boundary: queue manifest selection and milestone unlock, dependency
   ready scheduling, bounded parallel attempts, distinct worktrees, serialized merge/conflict
   recovery, durable restart, and operator batch checkpoints. Roko currently has pieces of this
   across Runner-v2, GraphEngine, the queue-manifest parser, and the TUI, but no single proven
   end-to-end path. See [`37-MORI-WORKFLOW-PARITY-AUDIT.md`](37-MORI-WORKFLOW-PARITY-AUDIT.md).

1. ~~**Acknowledged command contract (#146/#233).**~~ DONE (2026-09-04). Plan-scoped retry/skip/repair/reverify
   state transitions now operational; command ID tracking and acknowledgement semantics verified.
2. ~~**True gate streaming (#234).**~~ DONE (2026-09-04). ShellGate publishes exact rung-start/rung-line/rung-finish
   events while subprocess is alive; live line-by-line output verified operational.
3. ~~**Terminal source convergence (#182/#323).**~~ Partially addressed (2026-09-02 swarm, task #34).
   Status snapshot sources unified. PID/staleness semantics and post-change live fixture remain.
4. ~~**Modal/input correctness (#365/#368).**~~ Addressed (2026-09-02 swarm, tasks #2/#3). Modal
   precedence fixed; mouse routing uses panel hit-test registry; shared-scroll fallbacks removed.

## P1 — information needed during a run

1. ~~**Connected plan detail (#238).**~~ Partially addressed (2026-09-02 swarm, task #8). Dependencies,
   acceptance/verify text, diff stats, branch/worktree/commit, and elapsed time wired into plan
   detail modal. Authoritative start time from the accepted attempt still needs live fixture proof.
2. ~~**Critical-path ETA (#196).**~~ Addressed (2026-09-02 swarm, task #5). DAG estimate wired into
   connected state and header bar display. Fallback estimate labeling remains.
3. **Provider transcript completeness (#108/#232/#367).** Preserve semantic tool segments and add
   bounded paged history/search without duplicating live and settled output.
4. ~~**Metric correctness (#239/#240).**~~ Addressed (2026-09-02 swarm, task #6). Network rate units
   corrected; disk I/O rate populated from sysinfo deltas. MCP configured-vs-live distinction remains.
5. **Topology truth (#125).** Render authored/runtime dependencies; do not present a synthesized flat
   wave as authoritative plan topology.
6. **Unified semantic transcript (tool-audit P0/P1).** Replace provider-specific/string-tail
   output projections with the canonical event and transcript model in
   `../tool-audit/01-event-schema.md`; preserve tool, reasoning, todo, and subagent semantics.

## P2 — rendering and evidence

1. ~~**Render-path budget (#128/#366).**~~ Partially addressed (2026-09-02 swarm, task #1).
   `Theme::from_env()` cached; per-frame environment rebuild removed. p95 draw benchmark remains.
2. ~~**Complete responsive audit (#199/#241).**~~ Addressed (2026-09-02 swarm, task #25). Width/height
   disclosure priorities applied across views with column collapse breakpoints.
3. ~~**Style-preserving evidence (#151).**~~ DONE (2026-09-04). ANSI serialization, cell-level comparison,
   and similarity scoring verified operational; deterministic PNG rasterizer (optional) remains roadmap.
4. **Comparison and assessment (#152/#153).** Add baselines, tolerance masks, semantic/cell diffs,
   and CI gating only after deterministic styled captures exist.
5. ~~**Notification history (#369).**~~ Addressed (2026-09-02 swarm, task #7). Bounded, searchable,
   redacted notification history modal added.
6. **Transcript widget and provider-neutral output (tool-audit Phase 3).** Render semantic
   blocks with icons, colors, separators, fold/search, follow-tail, and responsive layouts.

## Remaining after 2026-09-04 closure

The following items from the original gap list are **still open**:

| ID | Gap | Status |
|---|---|---|
| P0.0 | Mori workflow contract | Open |
| P1.3 | Provider transcript completeness | Open |
| P1.5 | Topology truth | Open |
| P1.6 | Unified semantic transcript | Open |
| P2.4 | Comparison and assessment | Open |
| P2.6 | Transcript widget | Open |

**Closed 2026-09-04:**

| ID | Gap | Status | Evidence |
|---|---|---|---|
| P0.1 | Acknowledged command contract | DONE | Plan-scoped retry/skip/repair/reverify with command ID tracking verified |
| P0.2 | True gate streaming | DONE | ShellGate subprocess line-by-line events, rung-start/finish wired |
| P2.3 | Style-preserving evidence | DONE | ANSI serialization, cell-level comparison, similarity scoring operational |

## Required live fixtures

- cold-cache startup with visible warmup and periodic captures;
- long provider/tool call proving transcript liveness and bounded history;
- long gate proving line-by-line output and exact rung state;
- pause/resume acknowledgement with no new dispatch while paused;
- plan-scoped cancel and each recovery command with accepted/rejected/completed evidence;
- successful, failed, and cancelled terminal snapshots/status files;
- 80x24, 120x40, and 200x60 terminal resize during active output;
- low-disk/backpressure capture behavior and complete shutdown manifest;
- five cold and five warm evidence runs required by backlog #228.

No second paid provider run was started in this audit. The existing successful run was mined for
negative and terminal evidence; new code was checked with focused tests and controlled snapshots.
