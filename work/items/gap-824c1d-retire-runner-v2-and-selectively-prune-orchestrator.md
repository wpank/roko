+++
id = "gap-824c1d"
kind = "gap"
title = "Retire Runner-v2 and Selectively Prune Orchestrator Code"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/261-runner-retirement-orchestrator-prune.md#261 — Retire Runner-v2 and Selectively Prune Orchestrator Code"
discovered_from = "audit:tmp/backlog/archive/261-runner-retirement-orchestrator-prune.md#261 — Retire Runner-v2 and Selectively Prune Orchestrator Code"
anchors = ["crates/roko-cli/src/runner/mod.rs::run", "crates/roko-cli/src/main.rs:1941", "crates/roko-cli/src/commands/plan.rs:597"]
links = { depends_on = [], blocks = [], related = ["bug-96aff4", "find-5ffca2", "bug-17af83"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Runner-v2 was retired by another route: runner/event_loop.rs was deleted in 6b5da8616 and `--engine legacy|runner-v2` is accepted only to exit with an error (main.rs:1941, commands/plan.rs:597). Residual: the bail-only deprecated runner::run stub still has three production callers (commands/do_cmd.rs:997, prd.rs:1161, worker/cloud.rs:568), tracked by bug-96aff4 / find-5ffca2 / bug-17af83. The blocked batch 1-6 prune plan was never executed as written."
+++
[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/261-runner-retirement-orchestrator-prune.md#261 — Retire Runner-v2 and Selectively Prune Orchestrator Code`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-RV2-1 (Subsystem: Runner-v2 Retirement)`
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Phase 6: Retirement (6.1-6.3 retire orchestrate.rs / event_loop.rs / ACP bare spawn)`

Some cited files are gone: `graph_execution/dispatch/`, `tmp/engine-audit/15-dead-code-cli.md`.

How to verify: Check: Verify #277 evidence, then execute fixed batches 1-6 without generating a second caller/design inventory.; Move the named retained host files exactly once and update imports without behavior changes.; Delete the named… NOTE: CONSOLIDATED P3-RV2-1 (Runner-v2 retirement) is deferred; CLAUDE.md says Runner-v2 kept as --engine legacy for one release cycle. [evidence: own status: Blocked; CONSOLIDATED P3-RV2-1: deferred; 00-STATUS-SUMMARY 3…

Verified 2026-09-28: closed as superseded; see [closed].evidence.
