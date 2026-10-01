+++
id = "bug-54c729"
kind = "bug"
title = "roko-serve's runs route reports every task status other than passed as failed"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-serve/routes/runs"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "479bec688"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/bug-7e1b6b at 63b77c9f5)"
anchors = ["crates/roko-serve/src/routes/runs.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b", "bug-4e5a59"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn runs_route_keeps_unverified_and_skipped_statuses' crates/roko-serve/src/ && cargo test -p roko-serve --lib runs_route_keeps_unverified_and_skipped_statuses"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T12:49:53Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20d gate on fae7133cd, re-checked with the clippy fix on 9f3c184c5 (MAIN 479bec688 has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/compose/core/execution/graph/neuro/runtime/serve; lib tests roko-cli 3282, roko-agent 2282, roko-core 1962, roko-serve 990, roko-compose 561, roko-graph 476, roko-runtime 288, roko-execution 245, roko-neuro 239 all pass; extras: codex/cursor/openai parity 4+4+4 (streaming tests no longer ignored), default_engine 1, C1 1, C7 2, bin 429, graph_task_dispatch loop 10/10, including runs_route_keeps_unverified_and_skipped_statuses. Merged 601997dd1 (work/bug-4e5a59 2f2f8fb1c)."
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

## Notes

- Implemented on `work/bug-4e5a59` at `b63ee888c`; cargo verification deferred to the batch check.
- A `task_completed` that names its outcome reports the outcome's class from `classify_task_outcome` (bug-7e1b6b).
  A runtime `task_skipped` is now terminal and reports `skipped` (it was `observed`). Records without an outcome
  string are read as before. A runner `task.attempt.completed` passes only with `passed`: its `exhausted` and
  `timed_out` outcomes stay `failed`, where the dashboard classifier would make them `unverified`.
- Premise re-checked at 5c90262ec: no producer writes a task record with one of the new outcomes into the indexes
  this route reads, so the misreport was latent. Serve's StateHub logs dashboard events to `.roko/events.jsonl`
  without a `run_id`, so `events-by-run/` never holds them. The runtime index holds ingested `RuntimeEvent`s, whose
  `task_completed` carries only `passed`. The runner index holds the deleted Runner-v2's records.
- Not fixed here: on the runtime path a Graph task's outcome is lost. `GraphRuntimeEventAdapter` maps every
  `NodeCompleted` to `TaskCompleted { passed: true }`, so an unverified or already-satisfied task would read as
  passed. Only tests construct the adapter today.
