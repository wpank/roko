+++
id = "gap-31e1b0"
kind = "gap"
title = "PK94 Showcase and deploy: LA3 spec duel: two cheap attempts on a vague and a precise spec, capped at $0.20 (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 94
size = "S"
hold = "waits on Will's deferred decision(s) 3346, 7101, 9302, 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["roko-serve/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK94"
anchors = ["crates/roko-serve/src/routes", "demo/demo-app/src/pages", "docker"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-4a5109", "gap-00e882", "gap-fbd580", "gap-ba9965", "gap-05c4c0"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn showcase_spec_duel_runs_both_variants_under_one_reservation' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_spec_duel_"

[[verify]]
command = "deploy/showcase/preflight.sh --live-checks https://roko-showcase.fly.dev"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK94, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9360 | S | p3 | LA3 spec duel: two cheap attempts on a vague and a precise spec, capped at $0.20 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9360-la3-spec-duel.md` |
| 2 | 9361 | S | p2 | Turn on live actions on the Fly showcase and pass the live acceptance checks (author-run) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9361-turn-on-live-actions-on-the-fly-showcase.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/showcase/actions.rs`, `demo/demo-app/src/pages/showcase/SpecQuality.tsx`, `docker/showcase.roko.toml`, `fly.showcase.toml`.

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

- Waits on: PK57 (gap-4a5109), PK87 (gap-00e882), PK90 (gap-fbd580), PK92 (gap-ba9965), PK93 (gap-05c4c0).
- On hold until Will takes the deferred decision(s) 3346, 7101, 9302, 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
