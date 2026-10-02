+++
id = "gap-00e882"
kind = "gap"
title = "PK87 Showcase and deploy: Deploy the replay-only showcase to Fly and pass the F1 acceptance checks (author-run) (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
rank = 87
size = "S"
hold = "waits on Will's deferred decision(s) 9302, 9304 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["deploy/fly"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK87"
anchors = ["crates/roko-serve/src/routes/middleware.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-9ecd37", "gap-fcb44c", "gap-3516d6"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x deploy/showcase/preflight.sh && deploy/showcase/preflight.sh https://roko-showcase.fly.dev"

[[verify]]
command = "! grep -q 'downgrading to read scope' crates/roko-serve/src/routes/middleware.rs && grep -rqw 'fn privy_jwt_role_mismatch_is_rejected' crates/roko-serve/src/ && cargo test -p roko-serve --lib privy_jwt_role_mismatch_is_rejected"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK87, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9339 | S | p2 | Deploy the replay-only showcase to Fly and pass the F1 acceptance checks (author-run) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9339-deploy-the-replay-showcase-to-fly-f1.md` |
| 2 | 9340 | S | p2 | A Privy JWT with a role outside the allow-list gets read scope instead of being rejected (D18) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9340-privy-role-mismatch-is-rejected-not-downgraded.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/middleware.rs`.

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

- Waits on: PK82 (gap-9ecd37), PK85 (gap-fcb44c), PK86 (gap-3516d6).
- On hold until Will takes the deferred decision(s) 9302, 9304 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
