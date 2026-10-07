+++
id = "gap-21e5f5"
kind = "gap"
title = "PK88 Showcase and deploy: Apply the owner decision to the image registry, worker image default, commit identity…"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
rank = 88
size = "S"
subsystem = ["release"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "cb8cbfeed"
source = "tmp/backlog/2026-10-02-complete-and-wire PK88"
anchors = [".github/workflows/docker-publish.yml", "crates/roko-cli/src/worker/cloud.rs", "crates/roko-core/src/config/serve.rs", "docker/RAILWAY.md", "docker/README.md"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-4119fb"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worker_image_default_matches_the_published_prefix' crates/roko-core/src/ && cargo test -p roko-core worker_image_default_matches_the_published_prefix"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T04:48:56Z"
commit = "cb8cbfeed"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T01:55:06Z"
forced = false
evidence = "Gate 13c (merged cb8cbfeed): roko-core and roko-cli lib tests pass; verify worker_image_default_matches_the_published_prefix passes. 9341 per decision 9305: images under ghcr.io/wpank in docker-publish.yml, the worker_image default and docker docs; cloud workers' git identity overridable by ROKO_WORKER_GIT_AUTHOR_NAME/_EMAIL with a neutral default."
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

## Progress

- 9341: implemented at 06480e5db. docker-publish.yml's IMAGE_PREFIX, serve.rs's default worker_image
  (+ doc comment), and the three published image names in docker/README.md and docker/RAILWAY.md
  now read `ghcr.io/wpank` instead of `ghcr.io/nunchi-trade` (decision 9305). New test
  `worker_image_default_matches_the_published_prefix` (serve.rs) reads docker-publish.yml with
  `include_str!` so the two can't drift apart silently again. cloud.rs's hardcoded
  `roko`/`roko@nunchi.dev` commit identity is now `resolve_worker_git_identity()`, overridable via
  `ROKO_WORKER_GIT_AUTHOR_NAME`/`_EMAIL` (neutral default email: GitHub's own
  `roko@users.noreply.github.com` no-reply convention), catalogued in env_registry.rs's
  server_deploy() category. No RokoConfig field: cloud.rs's only caller
  (`run_code_implementer_cloud`) has no RokoConfig in scope, and a config field would need deeper,
  uncompilable-for-me threading through the call chain; a pure env-var default/override matches
  env_registry.rs's own stated purpose (a catalog of direct env::var reads) at much lower risk, so
  loader.rs's schema tree is unaffected. `authors` in Cargo.toml and the relay domain left alone
  (bug-911361's, per the brief). Verify's static grep passes; `cargo test` deferred to the gate.
