+++
id = "bug-54c729"
kind = "bug"
title = "roko-serve's runs route reports every task status other than passed as failed"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-serve/routes/runs"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-7e1b6b at 63b77c9f5)"
anchors = ["crates/roko-serve/src/routes/runs.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "bug-4e5a59"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn runs_route_keeps_unverified_and_skipped_statuses' crates/roko-serve/src/ && cargo test -p roko-serve --lib runs_route_keeps_unverified_and_skipped_statuses"
+++

## Problem

`terminal_status` in `crates/roko-serve/src/routes/runs.rs` (:1340) returns `"passed"` when the record says passed, and `"failed"` for everything else (:1346-1352). Tasks that settled as `unverified`, `skipped` or `already_satisfied` (gap-9eb1e1) are reported as failed over HTTP.

## Why it matters

Honest verdicts (epic spec-e9d7ec): the API turns "not checked" and "nothing to do" into "failed". bug-7e1b6b fixes the dashboard's version of this, the opposite error (counting them as passed).

## Where

`terminal_status`.

## Plan

1. Return the settled outcome as recorded (`passed`, `failed`, `unverified`, `skipped`, `already_satisfied`, …), and let clients group them.
2. Add `runs_route_keeps_unverified_and_skipped_statuses`.

## Done when

- [ ] The runs route reports each task's real outcome.
- [ ] The `[[verify]]` command passes.
