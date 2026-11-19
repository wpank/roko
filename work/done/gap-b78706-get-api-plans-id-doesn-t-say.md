+++
id = "gap-b78706"
kind = "gap"
title = "GET /api/plans/{id} doesn't say whether the plan has a run a resume would continue, so a chat host learns it only from plan_run's refusal, after its person approved paid work"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "6d1c896bd"
source = "round-v7-roko-ladder#cbdb6b-precheck"
discovered_from = "agent:plugin-wow (P-14: ask before Hermes' approval prompt)"
anchors = ["crates/roko-serve/src/routes/plans/reads.rs::get_plan"]
links = { depends_on = [], blocks = [], related = ["bug-cbdb6b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_plan_says_whether_it_has_a_run_a_resume_would_continue' crates/ && cargo test -p roko-serve -- a_plan_says_whether_it_has_a_run_a_resume_would_continue"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T10:10:46Z"
commit = "6d1c896bd"
by = "roko-ladder"
forced = false
evidence = "6d1c896bd: GET /api/plans/{id} carries resumable_run, the run the plan checkpoint records with its status, passed tasks and tasks left while a resume would continue it (CheckpointRun, resumable), else null, from the same CliRuntime::checkpoint_run lookup plan_run's refusal uses (bug-cbdb6b); reading it changes nothing and an unreadable checkpoint leaves null (reads.rs get_plan). The run's plan directory and the one it reads come from one helper (plans.rs plan_dir_of, used by start_plan_run_with too). Test a_plan_says_whether_it_has_a_run_a_resume_would_continue: red at 5de243ec8 (resumable_run absent: left Null, right the failed run), green now; it also shows null for a finished run and for no checkpoint, and that nothing runs. Checks: roko-serve lib 1069 passed plus its 15 integration targets; clippy -D warnings (roko-serve) and nightly rustfmt clean."
+++

## Problem

A chat host only learns that a plan has a run a resume would continue from `plan_run`'s refusal (bug-cbdb6b), and
that comes after the host has asked its person to approve paid work. With Hermes' approvals on, it goes:

1. the person approves "run plan X, cap $2";
2. roko refuses with `resumable_run`;
3. the person chooses "continue";
4. the person approves a second time.

`GET /api/plans/{id}` says nothing about the plan's checkpoint, so the host can't ask "continue or start over?"
before the first approval.

## Why it matters

Round v7's plugin (plugin-wow, P-14) wants to ask before any approval, using the same words as roko's refusal.
Without this, it can only guess from its own run log, and that misses runs started elsewhere (a terminal, the HTTP
API). Related: bug-cbdb6b.

## Where

- `crates/roko-serve/src/routes/plans/reads.rs::get_plan`, the handler behind `GET /api/plans/{id}`.
- `CliRuntime::checkpoint_run` (roko-serve `runtime.rs`; implemented in roko-cli `serve_runtime.rs`). It is the
  lookup `plan_run`'s refusal uses.

## Current state

bug-cbdb6b (53533fe93) added `checkpoint_run` and the refusal. It gives the refusal's data as `{run_id, status,
passed_tasks, tasks_left, resumable}`.

## Plan

1. `get_plan` resolves the plan's directory the same way `start_plan_run_with` does: share one helper, don't copy
   it.
2. It adds `resumable_run`: the `checkpoint_run` result when it is resumable, else null. The lookup reads the
   checkpoint files and changes nothing. If it fails, `resumable_run` is null and the route still answers.

## Done when

`GET /api/plans/{id}` carries `resumable_run` with the run, its status, passed tasks and tasks left while the plan's
last run stopped part way, and null otherwise (a finished run, or no checkpoint). Checked by the test in
`[[verify]]`.

## Notes

- Same shape as `plan_run`'s `resumes` and `set_aside`. Its `resumable` is always true here.
- Discovered by plugin-wow; the lead approved it on 2026-10-20.
