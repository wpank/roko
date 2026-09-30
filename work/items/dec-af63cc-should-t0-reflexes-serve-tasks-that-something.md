+++
id = "dec-af63cc"
kind = "decision"
title = "Should T0 reflexes serve tasks that something verifies, running verification on their cached output?"
status = "open"
triage = "unverified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/reflex_store"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates and wk-settle's report)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-learn/src/reflex_store.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-b4c565", "gap-4468bd", "bug-94151f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_reflex_on_a_checked_task_runs_verification_on_its_cached_output' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_reflex_on_a_checked_task_runs_verification_on_its_cached_output"
+++

## Problem

Since bug-b4c565, T0 reflexes serve only tasks that nothing verifies: no authored verify steps and no workspace rungs. gap-4468bd credits or demotes a reflex rule from the settled attempt record, after verification. But reflexes never serve a verified task in live runs, so that credit hook never fires, and reflex rules can't earn or lose credit.

## Decision

The options:

- **(a) Keep reflexes narrow** (unverified tasks only) until there's evidence that they help. Revisit with the M1–M4 measurements.
- **(b) Let reflexes serve checked tasks.** Run verification on the reflex's cached output, first attempt only: on a failure, the next attempt dispatches normally. gap-4468bd's hook then credits or demotes the rule.

**Recommended default: (a).** Keep reflexes narrow until there's evidence, and revisit with M1–M4.

## Done when

- [ ] Will decides. Under (b), the `[[verify]]` command passes. Under (a), close this item as decided, with the decision as evidence.
