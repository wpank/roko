+++
id = "gap-3516d6"
kind = "gap"
title = "PK86 Showcase and deploy: `deploy/showcase/preflight.sh`: checks P1–P14, with a `--local` mode that boots a… (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
rank = 86
size = "M"
subsystem = ["deploy/fly"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK86"
anchors = ["demo/demo-app", "demo/demo-app/e2e", "deploy"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-4119fb", "gap-fcb44c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x deploy/showcase/preflight.sh && bash -n deploy/showcase/preflight.sh && deploy/showcase/preflight.sh --local"

[[verify]]
command = "test -f demo/demo-app/playwright.fly-smoke.config.ts && test -f demo/demo-app/e2e/showcase/fly-smoke.spec.ts && cd demo/demo-app && npx playwright test -c playwright.fly-smoke.config.ts"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK86, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9337 | M | p2 | `deploy/showcase/preflight.sh`: checks P1–P14, with a `--local` mode that boots a showcase-mode serve | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9337-showcase-preflight-script-with-local-mode.md` |
| 2 | 9338 | S | p3 | Playwright `fly-smoke`: log in, open the Overview drawers, log out, and see `WakeUpBanner` on a cold start | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9338-fly-smoke-playwright-config.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `demo/demo-app/e2e/showcase/fly-smoke.spec.ts`, `demo/demo-app/playwright.fly-smoke.config.ts`, `deploy/showcase/README.md`, `deploy/showcase/preflight.sh`.

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

- Waits on: PK83 (gap-4119fb), PK85 (gap-fcb44c).
- Suggested model: sonnet.
