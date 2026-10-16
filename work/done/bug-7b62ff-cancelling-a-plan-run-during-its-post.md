+++
id = "bug-7b62ff"
kind = "bug"
title = "Cancelling a plan run during its post-merge regression check leaves the check running and the workspace runner lock held, so the next run fails at once"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "6dd936e4b"
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: demo plans run from Hermes in the roko repo"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify", "crates/roko-cli/src/workspace_lock.rs"]
links = { depends_on = [], blocks = [], related = ["bug-1d09e8", "bug-0c7f25"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancel_stops_the_post_merge_regression_check' crates/roko-cli/ && cargo test -p roko-cli --lib -- a_cancel_stops_the_post_merge_regression_check a_cancel_stops_the_whole_plan_check_and_frees_the_workspace a_stop_ends_the_plan_check_and_kills_its_steps a_lock_held_by_this_process_is_not_a_process_to_kill"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T01:39:19Z"
commit = "6dd936e4b"
by = "roko-credfix"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-16T00:58:34Z"
model = "claude-opus-5-5"
forced = false
evidence = "Whole-plan and post-merge regression check steps now run in their own process group (roko_gate::output_in_own_group, registered with the process registry) and race the run's stop request (PlanRunInterruptHandle::stopped). A stop kills the group, records stopped_by=cancel (PlanVerifyFailure / DeliveryCheck, summary 'post-merge regression stopped'), and the plan ends Interrupted, so the run ends cancelled (exit 130) and releases its runner lock. A lock refusal held by this process no longer says to kill it. Tests pass: a_cancel_stops_the_post_merge_regression_check (group gone, regression checkout free), a_cancel_stops_the_whole_plan_check_and_frees_the_workspace (end to end in both modes: exit 130, sh and its sleep gone, stop cause cancel, runner lock free), a_stop_ends_the_plan_check_and_kills_its_steps, a_lock_held_by_this_process_is_not_a_process_to_kill. roko-cli 3688 + roko-gate 701 lib tests pass; clippy -D warnings clean (tmp/hermes-roko-v5/findings/roko-credfix.md section 10b)"
+++

## Problem

2026-10-15, roko-main 7b6a5af44, `roko serve` in a clone of the roko repo: run `6ee167ae…` (`demo-hello-world`, started
from Hermes) passed its task, delivery merged it, and the post-merge regression check started
`cargo clippy --workspace --no-deps -- -D warnings` in `.roko/state/regression-checkout` (a child of roko serve).
`run_cancel` over `/mcp` at 14:04:58 made serve and Hermes report "cancelled", but:

- the clippy kept running (until I killed it at 14:07:12; otherwise until the step's 15-minute timeout);
- the run's thread kept the workspace runner lock, so a plan run started at 14:05:39 failed in 0.7 s with
  `plan execution failed for demo-hello: A roko plan runner is already active in this workspace (PID 38618).
  hint: wait for it to finish, or kill it with \`kill 38618\`` — PID 38618 being roko serve itself.

Expected: a cancel stops the regression check (its whole process group), the run ends cancelled and releases the
lock, and no hint tells anyone to kill the server.

## Why it matters

Hermes and the portal show the run as cancelled while it still holds the workspace; the next run the person starts
fails at once, and (see the related item on pre-start failures) its card does not say why.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs`: the post-merge regression check (`sh -c`).
- `crates/roko-cli/src/graph_execution/plan_verify.rs::run_plan_verify`: `kill_on_drop` on the `sh` child only,
  and nothing selects on the run's cancel token while it waits.
- `crates/roko-cli/src/workspace_lock.rs`: the runner-lock message and its `kill <pid>` hint.

## Plan

1. Run the regression and whole-plan steps in their own process group, and select on the run's interrupt: on a
   cancel, kill the group and record the delivery as cancelled.
2. When the lock holder is the current process (serve), say that a run in this server still holds the workspace.
3. Test: a regression step that sleeps is cancelled; the step's process group is gone and the lock is free.

## Done when

`a_cancel_stops_the_post_merge_regression_check` passes (the `[[verify]]` command), with the end-to-end test
`a_cancel_stops_the_whole_plan_check_and_frees_the_workspace` (both check modes: the run ends cancelled, the check's
processes are gone, the runner lock is free). `--lib` keeps the verify from building the ~70 integration test binaries.
