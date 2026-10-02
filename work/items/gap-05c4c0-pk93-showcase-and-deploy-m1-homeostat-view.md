+++
id = "gap-05c4c0"
kind = "gap"
title = "PK93 Showcase and deploy: M1 homeostat view: essential variables, recovery episodes and second-order actions… (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 93
size = "L"
hold = "waits on Will's deferred decision(s) 3346, 7101, 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["demo-app/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK93"
anchors = ["demo/demo-app/src/main.tsx"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-5ddf9b", "gap-85d176", "gap-147c4d", "gap-f7bab8", "gap-099513", "gap-4119fb", "gap-59ebfd", "gap-fcb44c", "gap-93c748", "gap-fbd580", "gap-799698", "gap-ba9965"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/homeostat.spec.ts && ! grep -q 'NotMeasured' demo/demo-app/src/pages/showcase/Homeostat.tsx && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/homeostat.spec.ts"

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/loops.spec.ts && ! grep -q 'NotMeasured' demo/demo-app/src/pages/showcase/LoopLedger.tsx && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/loops.spec.ts"

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/bench-explorer.spec.ts && ! grep -q 'NotMeasured' demo/demo-app/src/pages/showcase/BenchExplorer.tsx && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/bench-explorer.spec.ts"

[[verify]]
command = "grep -rqw 'fn showcase_live_projection_matches_offline_analysis' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_live_projection_matches_offline_analysis"

[[verify]]
command = "grep -rqw 'fn showcase_stream_resumes_from_last_event_id' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_stream_"

[[verify]]
command = "grep -rqw 'fn showcase_actions_reserve_before_start' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_actions_"

[[verify]]
command = "test -f demo/demo-app/src/pages/showcase/RunConsole.tsx && test -f demo/demo-app/e2e/showcase/run-console.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/run-console.spec.ts --grep-invert @live"

[[verify]]
command = "grep -rqw 'fn showcase_la2_disturbance_only_on_showcase_runs' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_la2_"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK93, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9346 | M | p3 | M1 homeostat view: essential variables, recovery episodes and second-order actions (replay) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9346-m1-homeostat-view.md` |
| 2 | 9347 | S | p3 | M2 loop ledger view: exposure, influence and benefit per loop (replay) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9347-m2-loop-ledger-view.md` |
| 3 | 9348 | S | p3 | Benchmark explorer: arms with 95% bands by campaign, metric and x-axis (replay) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9348-benchmark-explorer-view.md` |
| 4 | 9355 | M | p3 | `source=live`: showcase read models projected from S01 records, equal to the offline analysis | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9355-showcase-live-projections-over-s01-records.md` |
| 5 | 9356 | M | p3 | Showcase SSE stream: `showcase-event/1` frames with `Last-Event-ID` resume and idle close | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9356-showcase-sse-stream-with-last-event-id-resume.md` |
| 6 | 9357 | M | p2 | LA1 actions: a capped showcase run and a CPU-only audit draw, behind the ledger, CSRF and idempotency keys | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9357-la1-capped-run-and-cpu-only-audit-draw.md` |
| 7 | 9358 | S | p3 | Run console page: suite, tasks, arm and cap, a live event feed, and a spend bar | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9358-run-console-page.md` |
| 8 | 9359 | M | p3 | LA2 actions: an M1 mini-disturbance and an M2 loop break, each capped at $1 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9359-la2-m1-disturbance-and-m2-loop-break.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/showcase/actions.rs`, `crates/roko-serve/src/routes/showcase/live.rs`, `crates/roko-serve/src/routes/showcase/mod.rs`, `crates/roko-serve/src/routes/showcase/stream.rs`, `crates/roko-serve/src/routes/showcase/views.rs`, `demo/demo-app/e2e/showcase/bench-explorer.spec.ts`, `demo/demo-app/e2e/showcase/homeostat.spec.ts`, `demo/demo-app/e2e/showcase/la2.spec.ts`, `demo/demo-app/e2e/showcase/loops.spec.ts`, `demo/demo-app/e2e/showcase/run-console.spec.ts`, `demo/demo-app/src/components/Charts/RecoveryChart.tsx`, `demo/demo-app/src/components/Charts/Sparkline.tsx`, `demo/demo-app/src/main.tsx`, `demo/demo-app/src/pages/showcase/BenchExplorer.tsx`, `demo/demo-app/src/pages/showcase/Homeostat.tsx`, `demo/demo-app/src/pages/showcase/LoopLedger.tsx`, `demo/demo-app/src/pages/showcase/RunConsole.tsx`.

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

- Waits on: PK27 (gap-5ddf9b), PK44 (gap-85d176), PK59 (gap-147c4d), PK62 (gap-f7bab8), PK71 (gap-099513), PK83 (gap-4119fb), PK84 (gap-59ebfd), PK85 (gap-fcb44c), PK89 (gap-93c748), PK90 (gap-fbd580), PK91 (gap-799698), PK92 (gap-ba9965).
- On hold until Will takes the deferred decision(s) 3346, 7101, 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
