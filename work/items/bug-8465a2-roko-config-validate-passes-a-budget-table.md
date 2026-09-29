+++
id = "bug-8465a2"
kind = "bug"
title = "roko config validate passes a budget table that the core config loader rejects"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["cli", "config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "98a77c510"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:15, wk-readme's quick-start check for bug-09690f; details in bug-09690f's Notes)"
anchors = ["crates/roko-core/src/config/validation.rs", "crates/roko-cli/src/commands/config_cmd.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-09690f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_validate_matches_the_core_loader_on_budgets' crates/ && cargo test -p roko-cli --lib config_validate_matches_the_core_loader_on_budgets"
+++

## Problem

The core loader rejects `[budget] max_plan_usd = 10, max_task_usd = 1` without `max_turn_usd`, reporting `max_turn_usd (0) must not exceed max_plan_usd (10)`. `roko config validate` passes the same file. The old README's budget example hit this.

## Why it matters

A new user's first commands fail, and the README's quick start depends on them. Epic spec-ae5f94.

## Where

The budget rule in `crates/roko-core/src/config/validation.rs` (about line 68), and the `config validate` path in `crates/roko-cli/src/commands/config_cmd.rs`.

## Current state

Reproduced on 2026-09-29 with `target/debug/roko` built at `33e107da1`, in scratch directories with an empty `HOME` and no API keys (wk-readme).

## Plan

1. Make `config validate` run the loader's invariants.
2. Fix the budget rule's handling of an unset `max_turn_usd`: 0 means no cap, and the message reads as a contradiction.
3. Add a test with the README example.

## Done when

- [ ] `config validate` and the loader agree on budget tables.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `98a77c510` (same binary): the loader rejects `[budget] max_plan_usd = 10, max_task_usd = 1` with invariant 1, while `roko config validate` passes it. Validate also passed `max_turn_usd = 2` above `max_plan_usd = 1`, which the loader rejects.
- Invariant 1 now fires only for a finite turn cap above a finite plan cap. `roko config validate` has a new phase that resolves the file through the core loader, as commands do, and lists the loader's invariant warnings.
- Implemented on `work/bug-e1327f` at `f99ae073e`; cargo verification deferred to the batch check.
