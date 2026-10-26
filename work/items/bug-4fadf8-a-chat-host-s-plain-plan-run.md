+++
id = "bug-4fadf8"
kind = "bug"
title = "A chat host's plain plan_run or the portal's Run button silently sets aside a run whose tasks all passed but whose delivery a crash cut, and redoes every task: bug-cbdb6b's refusal needs tasks left, and the Run button never asks"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of", "crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run_with", "crates/roko-serve/src/routes/plans/run_control.rs::execute_plan"]
links = { depends_on = [], blocks = [], related = ["bug-cbdb6b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_plain_plan_run_over_an_undelivered_passed_run_is_refused' crates/roko-serve/src && cargo test -p roko-serve a_plain_plan_run_over_an_undelivered_passed_run_is_refused"
+++

## Problem

Say a crash cut a run during delivery: every task had passed, and the merge or its plan check was in progress. A
chat host then calls `plan_run` with neither `resume` nor `fresh`, or the portal's Run button sends
`POST /api/plans/<p>/execute` with no body. Either one starts the plan over: the checkpoint is set aside, and **every
task runs again**.

The answer names the run it set aside: `set_aside: {run_id, passed_tasks: ["T1", "T2"], tasks_left: [],
resumable: false}`. bug-cbdb6b's refusal ("Plan '<p>' has a run that can be resumed … Pass resume=true …") is meant
to stop exactly this, but it doesn't fire.

The $0 kill matrix showed this in round v7 (`tmp/hermes-roko-v7/roko-recovery/`, kill points e and e', on both the
CLI and serve paths; findings in `tmp/hermes-roko-v7/findings/roko-recovery.md`):
- A plain MCP `plan_run` or the Run button redid T1 and T2, for $0.246 on a run that had already spent $0.164.
- `plan_run {resume: true}` instead "restores T1, T2 and runs nothing" and delivers.

At e', the dead run's batch branch already holds the delivered merge.

## Why it matters

- Goal core. Long phases spend most of their money before delivery. A crash during the plan check, which builds and
  tests, is likely. The default action then throws away every passed task.
- Chat hosts omit `resume`. That was bug-cbdb6b's origin: the pilot's chat did it on 2026-10-19.
- The portal's Run button is the default action on the plan page.

## Where

- `crates/roko-cli/src/serve_runtime.rs::checkpoint_run_of` sets
  `resumable: resumes && stopped && !passed_tasks.is_empty() && !tasks_left.is_empty()`. A run that passed every task
  but didn't deliver has an empty `tasks_left`, so it isn't "resumable".
- `crates/roko-serve/src/routes/plans/run_control.rs::start_plan_run_with` refuses only when `origin.is_chat()`.
  `execute_plan` (HTTP origin, the Run button) starts over without asking.

## Current state

bug-cbdb6b (done) added the refusal for chat hosts over a failed or interrupted run with tasks left. Neither
"delivery pending" nor the portal's origin is covered.

## Plan

1. In `checkpoint_run_of`, count a checkpoint whose `roko.delivery@1` state is below `delivered` (or which has passed
   tasks and no delivery) as resumable, with `tasks_left` empty and a `delivery_pending: true` field.
2. Extend the refusal message: "… passed T1 and T2; its delivery didn't finish. Pass resume=true to deliver it …".
3. Ask the Run button too: 409 over a resumable checkpoint unless the body says `fresh` or `resume`. The portal then
   shows the choice. If the portal can't ask yet, keep HTTP as it is and file the portal side.

## Done when

- A plain chat-host `plan_run` over an undelivered, fully passed run is refused, and so is `POST …/execute` without
  `fresh` (test `a_plain_plan_run_over_an_undelivered_passed_run_is_refused`).
- The kill matrix's E6 passes at e and e' (`harness/check.py --only E6`).

## Notes

- Related:
  - bug-cbdb6b: the refusal this extends.
  - The roko-recovery p1: a resume that can't continue answers 409.
- The portal change is UI work. Coordinate with the plugin, whose run form has no resume toggle (P-34 in
  `tmp/hermes-roko-v7/findings/hermes-pilot.md`).
