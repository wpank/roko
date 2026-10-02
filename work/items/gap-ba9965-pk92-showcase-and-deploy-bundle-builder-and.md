+++
id = "gap-ba9965"
kind = "gap"
title = "PK92 Showcase and deploy: Bundle builder and verifier: the R2 views (routing, specs, homeostat, loops,… (+3 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
rank = 92
size = "L"
hold = "waits on Will's deferred decision(s) 3346, 7101, 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["demo-app/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK92"
anchors = ["demo/demo-app/src/main.tsx"]
lane = "frontend"
parent = "spec-0b3a32"
links = { depends_on = ["gap-2ca903", "gap-85d176", "gap-7ec3ef", "gap-63fd4c", "gap-4a5109", "gap-ed1a08", "gap-099513", "gap-a63e3c", "gap-9ecd37", "gap-59ebfd", "gap-fcb44c", "gap-799698"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'p1-routing' benchmarks/viabilitybench/showcase/build_bundle.py && grep -q 'timeline' benchmarks/viabilitybench/showcase/verify_bundle.py && python3 -m pytest -q benchmarks/viabilitybench/showcase/test_bundle.py"

[[verify]]
command = "test -f demo/demo-app/src/showcase/replay.ts && test -f demo/demo-app/e2e/showcase/replay.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/replay.spec.ts"

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/routing-calibration.spec.ts && ! grep -q 'NotMeasured' demo/demo-app/src/pages/showcase/Routing.tsx && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/routing-calibration.spec.ts"

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/specs.spec.ts && ! grep -q 'NotMeasured' demo/demo-app/src/pages/showcase/SpecQuality.tsx && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/specs.spec.ts"
+++

## Problem

This package delivers 4 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK92, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9342 | M | p3 | Bundle builder and verifier: the R2 views (routing, specs, homeostat, loops, calibration, bench) and the replay timeline | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9342-r2-bundle-views-and-timeline-in-the-builder.md` |
| 2 | 9343 | M | p3 | Replay timeline: `ReplayClock`, scrubber and `?t=` deep links, plus routes for the R2 views | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9343-replay-clock-scrubber-and-r2-route-placeholders.md` |
| 3 | 9344 | M | p3 | P1 routing and M3 calibration views (replay) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9344-p1-routing-and-m3-calibration-views.md` |
| 4 | 9345 | M | p3 | P1 spec-quality view: vague against precise specs, by model tier (replay) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9345-p1-spec-quality-view.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/showcase/build_bundle.py`, `benchmarks/viabilitybench/showcase/fixtures/`, `benchmarks/viabilitybench/showcase/test_bundle.py`, `benchmarks/viabilitybench/showcase/verify_bundle.py`, `demo/demo-app/e2e/showcase/replay.spec.ts`, `demo/demo-app/e2e/showcase/routing-calibration.spec.ts`, `demo/demo-app/e2e/showcase/specs.spec.ts`, `demo/demo-app/src/components/Charts/EscalationLadder.tsx`, `demo/demo-app/src/components/Charts/InteractionPlot.tsx`, `demo/demo-app/src/components/Charts/ReliabilityDiagram.tsx`, `demo/demo-app/src/main.tsx`, `demo/demo-app/src/pages/showcase/BenchExplorer.tsx`, `demo/demo-app/src/pages/showcase/Calibration.tsx`, `demo/demo-app/src/pages/showcase/Homeostat.tsx`, `demo/demo-app/src/pages/showcase/LoopLedger.tsx`, `demo/demo-app/src/pages/showcase/Replays.tsx`, `demo/demo-app/src/pages/showcase/Routing.tsx`, `demo/demo-app/src/pages/showcase/SpecQuality.tsx`, `demo/demo-app/src/showcase/replay.ts`, `demo/demo-app/src/showcase/schemas/`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK30 (gap-2ca903), PK44 (gap-85d176), PK49 (gap-7ec3ef), PK50 (gap-63fd4c), PK57 (gap-4a5109), PK67 (gap-ed1a08), PK71 (gap-099513), PK81 (gap-a63e3c), PK82 (gap-9ecd37), PK84 (gap-59ebfd), PK85 (gap-fcb44c), PK91 (gap-799698).
- On hold until Will takes the deferred decision(s) 3346, 7101, 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
