+++
id = "bug-9a5970"
kind = "bug"
title = "A plan run refused by the spec gate or failing before its plan starts records no error under its run id, so hosts show no reason"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "88943cfd4"
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-serve/src/routes/plans/run_control.rs::launch_single_plan_run", "crates/roko-cli/src/spec_gate.rs"]
links = { depends_on = [], blocks = [], related = ["bug-103bce", "bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_refused_before_dispatch_records_why_in_its_events' crates/ && cargo test -p roko-serve -p roko-cli a_run_refused_before_dispatch_records_why_in_its_events"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T10:06:08Z"
commit = "88943cfd4"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-16T08:12:32Z"
forced = false
model = "claude-opus-5-5"
evidence = "88943cfd4: a plan or plan-set run that fails before its plan starts gets, in its own index (roko-serve run_end, shared with interrupted_runs), the error and run_completed {outcome: refused, stage: start, reason: the error's first line}; the spec gate's refusal events end with the run's error naming plan, task and rule; the run summary reads both as failed (terminal for every host) with refused: {stage: start | spec_gate, reason}, and the plan status names a spec-gate refusal instead of task-level failures; serve's refused prompt runs write the same rows. No manifest is touched, so a refused run never reads as interrupted. Verify on the branch with main 7a68db4c5 merged in (ae83df729): cargo test -p roko-serve -p roko-cli a_run_refused_before_dispatch_records_why_in_its_events passes (3 tests); roko-cli lib 3706 and roko-serve lib 1037 passed, all roko-serve integration tests pass, the only failures are the 15 known roko-cli integration tests of bug-c63cca and bug-2a31bc (scratch HOME, no claude CLI); clippy -p roko-serve -p roko-cli -D warnings clean. Live, zero cost, a real roko serve: an HF2 plan and a run started while another process held the runner lock both read refused with their reason, before and after a restart (was: no reason, 404) - tmp/hermes-roko-v5/findings/roko-recovery.md."
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
