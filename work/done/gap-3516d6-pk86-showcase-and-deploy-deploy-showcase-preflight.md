+++
id = "gap-3516d6"
kind = "gap"
title = "PK86 Showcase and deploy: `deploy/showcase/preflight.sh`: checks P1–P14, with a `--local` mode that boots a… (+1 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
rank = 86
size = "M"
subsystem = ["deploy/fly"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "cb8cbfeed"
source = "tmp/backlog/2026-10-02-complete-and-wire PK86"
anchors = ["demo/demo-app", "demo/demo-app/e2e", "deploy"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-4119fb", "gap-fcb44c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x deploy/showcase/preflight.sh && bash -n deploy/showcase/preflight.sh && deploy/showcase/preflight.sh --local"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T04:50:17Z"
commit = "cb8cbfeed"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T02:59:19Z"
forced = true
evidence = "Forced: preflight --local passed 31/31 at gate 13c against the gate's binary; work.py's static runner runs it here against the main checkout's stale target/debug/roko (Oct 2), which has no showcase subcommand. Gate fix 2476bca28 (admin key in the environment; X-Roko-CSRF on login-unlock); P8 stream and P9 freeze skip on 404 until those routes exist. 9338's fly-smoke spec is in; its A1-A4 tile check waits for gap-dbe8e7 (the builder's overview isn't the claims board), so that verify moved there."
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

## Progress

- 9337: implemented at fad7c5c90. New deploy/showcase/preflight.sh (P1-P14) and
  deploy/showcase/README.md. `bash -n` passes on both the Homebrew bash (5.3) and macOS's own
  `/bin/bash` (3.2.57). Grounded every route/status in the Rust source (auth_session.rs,
  showcase/auth.rs, routes/mod.rs's showcase_router_mounts_no_public_extras test), not just S11's
  prose: e.g. login is 204 not the spec's "200" (matches auth-gate.spec.ts), and `/api/auth/session`
  not "/api/session". P8's stream and P9's admin-freeze sub-checks SKIP on a 404 rather than FAIL:
  grepped routes/showcase/mod.rs and confirmed neither route is wired yet. Validated against a
  throwaway Python stub of the same API surface (not committed): 31/31 PASS on the happy path, and
  wrong secrets correctly FAIL the right checks with exit = failure count. Could not run `--local`
  against a real roko binary: none of the prebuilt binaries available in this environment have the
  `showcase` subcommand yet (all predate it); the static parts of the verify
  (`test -x` && `bash -n`) pass, the live part is deferred to the coordinator's gate once a binary
  with `roko showcase` exists.
- 9338: implemented at 4ec4aef67. New demo/demo-app/playwright.fly-smoke.config.ts and
  e2e/showcase/fly-smoke.spec.ts (A1-A5), modeled closely on 9332's auth-gate.spec.ts (login flow,
  cookie name) and golden-views.spec.ts (the provenance-drawer SC1 pattern), plus 9333's
  build_bundle.py call so the local run's Overview has tiles. Could not run
  `npx playwright test` (no npm builds); deferred to the coordinator's gate.

- Gate 13c (2026-10-04, coordinator): preflight --local passes 31/31 after gate fix 2476bca28 (the admin key goes in
  the environment, and login-unlock gets X-Roko-CSRF). 9338's fly-smoke spec fails only on A1-A4's tile check: the
  bundle builder's overview isn't the claims board, so its verify moved to gap-dbe8e7.
