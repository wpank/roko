+++
id = "bug-5c25e1"
kind = "bug"
title = "roko run in a fresh workspace fails budget admission against a $1 turn cap although roko.toml sets max_turn_usd = 0"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["cli", "config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "5e50e959a"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:15, wk-readme's quick-start check for bug-09690f; details in bug-09690f's Notes)"
anchors = ["crates/roko-cli/src/commands/util.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-09690f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fresh_workspace_run_passes_budget_admission' crates/roko-cli/src/ && cargo test -p roko-cli --lib fresh_workspace_run_passes_budget_admission"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "0 means no cap everywhere: invariant 1 fires only when a finite turn cap exceeds a finite plan cap, and roko run's admission (run::check_budget_admission) takes caps from core [budget] (4c1ff3c5c, merged 19d4d4ac3). Batch 4 gate (work/rust-batch-4; crates tree identical to MAIN after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only b0ae64620; clippy -p roko-cli -p roko-core -p roko-agent --no-deps -D warnings clean; lib tests roko-cli 3077, roko-core 1921, roko-agent 2240, 0 failed."
+++

## Problem

In a fresh `roko init` workspace, `roko run --max-retries 0 "..."` fails budget admission with `predicted turn cost $1.5000 exceeds max_turn_usd $1.0000`, although `roko.toml` sets `max_turn_usd = 0.0`.

## Why it matters

A new user's first commands fail, and the README's quick start depends on them. Epic spec-ae5f94.

## Where

The admission check in `crates/roko-cli/src/commands/util.rs` (about line 457), and wherever the $1.00 default replaces the configured 0.

## Current state

Reproduced on 2026-09-29 with `target/debug/roko` built at `33e107da1`, in scratch directories with an empty `HOME` and no API keys (wk-readme).

## Plan

1. Find where 0 becomes $1.00. Treat 0 as no cap, consistently with the loader.
2. Add a test that admits a run in a freshly initialised workspace.

## Done when

- [ ] A fresh workspace's `roko run` passes admission.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `98a77c510` (same binary): a fresh workspace's `roko run` fails with `predicted turn cost $1.5000 exceeds max_turn_usd $1.0000`. `Config::from_roko_config` replaced the core `[budget]` with the CLI type's defaults ($10 plan, $1 task and turn).
- The CLI budget now takes its caps from the core section (`BudgetConfig::from_core`). The admission check moved unchanged into `roko_cli::run::check_budget_admission`, so the library test can run it.
- Implemented on `work/bug-e1327f` at `4c1ff3c5c`; cargo verification deferred to the batch check.
