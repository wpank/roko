+++
id = "bug-9a5970"
kind = "bug"
title = "A plan run refused by the spec gate or failing before its plan starts records no error under its run id, so hosts show no reason"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-serve/src/routes/plans/run_control.rs::launch_single_plan_run", "crates/roko-cli/src/spec_gate.rs"]
links = { depends_on = [], blocks = [], related = ["bug-103bce", "bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_refused_before_dispatch_records_why_in_its_events' crates/ && cargo test -p roko-serve -p roko-cli a_run_refused_before_dispatch_records_why_in_its_events"
+++

## Problem

Two ways a plan run started from Hermes ends without saying why, seen 2026-10-15 (roko-main 7b6a5af44 and Will's
serve binary, plugin 0.4.0):

1. **Spec-gate refusal.** `demo-hello` was refused before dispatch (`HF3: verify passes on the unchanged base`). The
   run's events hold one `event_log_entry` (`event_type: spec.gate.refused`) and `run_completed`; the summary has no
   error, so the Hermes card reads "failed · no gate ran · — of $0.25 · 11s" and "roko recorded no gate that did not
   pass, no blocked task and no error; the milestones are all there is."
2. **Failure before the plan starts.** A run that failed in `run_plans` because the workspace runner lock was taken
   (run `20b8b38c…`) published only an unscoped `ServerEvent::Error` (`.roko/events.jsonl`: `plan execution failed
   for demo-hello: A roko plan runner is already active …`). The run's own events hold only `plan_completed
   success=false` (keyed `run-20b8b38c`), `GET /api/runs/<id>` answers 404 and `/summary` shows no error.

## Why it matters

A person who clicked Run sees "failed" with nothing to act on. bug-103bce made task errors reach hosts; these two
paths were left out.

## Where

- `crates/roko-serve/src/routes/plans/run_control.rs::launch_single_plan_run`: the `Err(err)` arm publishes an
  unscoped error.
- `crates/roko-cli/src/spec_gate.rs`: the refusal is logged and recorded as an event-log entry only.

## Plan

1. Give both an `error` event in the run's own index (with the run id), as bug-103bce does for task errors, so the
   summary and hosts carry the reason.
2. Test both: a run refused by the spec gate, and a run whose start fails, each have an error event with the
   reason under their run id.

## Done when

`a_run_refused_before_dispatch_records_why_in_its_events` passes (the `[[verify]]` command).
