+++
id = "gap-21e5f5"
kind = "gap"
title = "PK88 Showcase and deploy: Apply the owner decision to the image registry, worker image default, commit identity…"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
rank = 88
size = "S"
subsystem = ["release"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK88"
anchors = [".github/workflows/docker-publish.yml", "crates/roko-cli/src/worker/cloud.rs", "crates/roko-core/src/config/serve.rs", "docker/RAILWAY.md", "docker/README.md"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-4119fb"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worker_image_default_matches_the_published_prefix' crates/roko-core/src/ && cargo test -p roko-core worker_image_default_matches_the_published_prefix"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK88, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9341 | S | p3 | Apply the owner decision to the image registry, worker image default, commit identity and crate authors | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9341-apply-the-owner-decision-to-images-and-identities.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/docker-publish.yml`, `crates/roko-cli/src/worker/cloud.rs`, `crates/roko-core/src/config/serve.rs`, `docker/RAILWAY.md`, `docker/README.md`.

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

- Waits on: PK83 (gap-4119fb).
- Suggested model: sonnet.
