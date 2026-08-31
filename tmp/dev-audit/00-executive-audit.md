# Executive audit

## Direct answer: where did the 90 minutes go?

The earlier reflex-store task was not a small change hidden behind excessive tests. Its source
spec was hundreds of lines and crossed durable storage, matching, promotion/demotion, runner
dispatch, safety authorization, replay attribution, CLI display, concurrent access, and
commit/push/merge workflow. The resulting work was thousands of changed lines.

The time was divided among:

- Understanding and implementing a broad end-to-end contract.
- Repeated Rust compile/check/clippy cycles across a very large roko-cli dependency graph.
- Multiple agents sharing one filesystem and contending for Cargo locks.
- Integration review discovering real release blockers: unexecuted actions being promoted,
  unsafe shell execution, non-isolated replay evidence, lossy condition identity, replay cleanup,
  exact-attempt attribution, and failed-reflex fallback behavior.
- Adding focused regressions for those issues and rerunning compilation after each structural fix.
- Git worktree, commit, push, merge, and final verification.

Cached test execution itself was not the dominant cost. In that session, the large roko-cli
library harness executed in roughly seconds once compiled; compilation/linking and integration
iteration were the expensive part. Removing tests would have left most of the elapsed time and
would have removed evidence for the exact bugs being fixed.

The honest optimization is:

- Make normal tasks much smaller and run them in FAST mode.
- Prevent duplicate compilation and duplicate orchestration.
- Run only impact-selected checks interactively.
- Keep broad suites off the critical path.
- Classify high-risk work separately instead of promising it in five minutes.

## Direct answer: what happened in the pictured Roko run?

The exact final runner interval was 18m09s, not 26m. The TUI remained open after the run failed,
kept its timer moving, and displayed stale active state. Several earlier restarts also added to
the operator's wall-clock experience.

| Phase | Measured time | Finding |
|---|---:|---|
| Setup plus maintenance | about 8.7s | GC/maintenance ran before useful work |
| Unconditional workspace warm | 35.6s | Did not prevent sandboxed cold builds |
| T1 agent | 368.6s | Tiny edit, then cold compile in /tmp |
| T1 canonical compile | 99.2s | Checked lib although main.rs changed |
| T1 structural check | 14ms | High-value and essentially free |
| T1 authored cargo check | 8.1s | The meaningful binary compile, but duplicated |
| T2 agent | about 600s | Useful edit existed; cold compile continued until timeout |
| Final runner interval | 1,089s | 1 of 7 tasks passed |

No test suite or clippy invocation appears in this path. The gate output explicitly shows compile
plus structural verification only.

## The highest-leverage fixes

Status in expanded integration snapshot `52d5f4df4` (implementation complete; final batched
verification and merge still pending):

- [x] Admission before preparation. Task capacity and exact attempt ownership are reserved before
  worktree loading, routing, prompt assembly, hooks, or actual-launch counters.
- [x] One verification owner for FAST. The agent patches and the runner executes exactly one
  authored verify; exact required Cargo semantics are deduplicated conservatively.
- [x] Make the FAST cache usable. Agent Cargo is prohibited, warm artifacts are preserved, and a
  narrowly scoped shared-target grant exists only as an explicit trusted opt-in.
- [x] Stop microtask amplification in plan generation. FAST policy now enforces bounded cohesive
  tasks, rejects same-file fragmentation/duplicate verification, and supplies exact artifacts,
  ranges, and symbol anchors.
- [x] Hard FAST budgets. The agent cap and internal plan deadline leave settlement headroom inside
  the outer evidence deadline; timeout evidence is exported only after durable settlement.
- [x] Complete the development feedback harness. Schema-v2 bundles, strict validation, metrics,
  score/debrief output, resource admission, run-scoped APIs, safe GET discovery, and opt-in
  CLI/text/PNG evidence hooks are implemented.
- [x] Replace poll-driven FAST admission with wake-driven scheduling, interpose hard deadlines
  through provider startup, salvage safe timeout diffs through the ordinary gate path, and make
  terminal projections converge.
- [ ] Finish the broader machine loop. Disk admission is implemented; the background high-water
  cache/GC lane and a benchmark-selected long-term cache strategy are not claimed by this snapshot.

See [11-implementation-status.md](11-implementation-status.md) for exact commits, the distinction
between implemented and verified, and the deliberately open architecture residuals.

## What “five minutes” should mean

Five minutes is realistic for a one-line edit, localized type/function change, parser/output
change, or small endpoint wiring when context is exact and a cache is warm.

It is not a safe universal completion SLO for:

- Durable state formats or migrations.
- Authorization/sandbox changes.
- Scheduler/concurrency changes.
- Cross-provider agent-loop integration.
- Broad public API changes across many crates.

For those, the five-minute SLO is: produce the first verified slice or a precise, evidence-backed
decomposition and blocker. Release-grade completion remains a separate lane.

## Tests: disable, retain, or redesign?

Redesign.

- Remove broad workspace test/clippy runs from the default local loop.
- Run no tests by default for T0 mechanical/doc/config changes.
- Run one exact existing or newly added regression for localized T1 logic.
- Prefer real CLI/API/browser/TUI evidence for T2 behavior.
- Retain focused invariant tests for T3 persistence/security/concurrency work.
- Run broad impacted/full suites asynchronously before merge or nightly.
- Quarantine live/network/flaky suites behind explicit features so they are not compiled merely
  to skip at runtime.

This gives most of the speed benefit without turning “fast” into “unknown.”
