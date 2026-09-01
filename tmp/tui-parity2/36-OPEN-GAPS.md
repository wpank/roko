# Open UX/TUI gaps after the follow-up pass

**Date:** 2026-09-01
**Purpose:** shortest actionable queue after reconciling source, persisted run data, Mori/Bardo,
backlog acceptance criteria, and controlled full-frame captures.

## P0 — operator truth and control

1. **Acknowledged command contract (#146/#233).** Define run-scoped command IDs and
   accepted/rejected/completed acknowledgements. Implement plan-scoped cancel and real
   retry/repair/reverify/skip state transitions before restoring those controls to normal hints.
2. **True gate streaming (#234).** Publish exact rung-start/rung-line/rung-finish events while the
   subprocess is alive. Completion-buffer replay must not be labeled streaming.
3. **Terminal source convergence (#182/#323).** Add PID/staleness semantics, unify terminal/offline
   status sources, and verify success/failure/cancel with a post-change live fixture.
4. **Modal/input correctness (#365/#368).** Finish modal precedence and route mouse input by the
   rendered panel registry; remove shared-scroll fallbacks.

## P1 — information needed during a run

1. **Connected plan detail (#238).** Carry dependencies, acceptance/verify text, authoritative
   start time, changed files/diff stats, branch, worktree, and commit from the accepted attempt.
2. **Critical-path ETA (#196).** Feed the existing DAG estimate into connected state and CLI status;
   label fallback estimates distinctly.
3. **Provider transcript completeness (#108/#232/#367).** Preserve semantic tool segments and add
   bounded paged history/search without duplicating live and settled output.
4. **Metric correctness (#239/#240).** Fix network units, produce disk I/O rate, and distinguish
   configured MCPs from live connectivity.
5. **Topology truth (#125).** Render authored/runtime dependencies; do not present a synthesized flat
   wave as authoritative plan topology.
6. **Unified semantic transcript (tool-audit P0/P1).** Replace provider-specific/string-tail
   output projections with the canonical event and transcript model in
   `../tool-audit/01-event-schema.md`; preserve tool, reasoning, todo, and subagent semantics.

## P2 — rendering and evidence

1. **Render-path budget (#128/#366).** Remove remaining per-frame environment/model rebuild work and
   establish a p95 draw/allocation benchmark at 80x24, 120x40, and 200x60.
2. **Complete responsive audit (#199/#241).** Apply explicit width/height disclosure priorities to
   every view before adding user-resizable panes.
3. **Style-preserving evidence (#151).** Add ANSI or cell-style serialization first, then an optional
   deterministic PNG rasterizer with a bundled font.
4. **Comparison and assessment (#152/#153).** Add baselines, tolerance masks, semantic/cell diffs,
   and CI gating only after deterministic styled captures exist.
5. **Notification history (#369).** Retain a bounded, searchable, redacted history separate from
   ephemeral toasts.
6. **Transcript widget and provider-neutral output (tool-audit Phase 3).** Render semantic
   blocks with icons, colors, separators, fold/search, follow-tail, and responsive layouts.

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
