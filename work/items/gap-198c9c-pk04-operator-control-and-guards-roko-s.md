+++
id = "gap-198c9c"
kind = "gap"
title = "PK04 Operator control and guards: roko's own bash git guard lets git stash, clean, checkout and restore through (+9 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 4
size = "L"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK04"
anchors = ["crates/roko-acp/src/builtin_tools.rs", "crates/roko-agent/src/safety/git.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/graph_execution/plan_set.rs", "crates/roko-cli/src/graph_execution/plan_verify.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/supervision.rs", "crates/roko-cli/src/inject.rs", "crates/roko-cli/src/inject/transport.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/serve_client.rs", "crates/roko-cli/tests/secrets_and_git_guard_canary.rs", "crates/roko-core/src/config/execution.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-serve/src/routes/plans/run_control.rs", "crates/roko-serve/src/routes/plans/tests.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_git_policy_denies_stash_clean_checkout_restore' crates/roko-agent/ && cargo test -p roko-agent default_git_policy_denies_stash_clean_checkout_restore"

[[verify]]
command = "grep -rqw 'fn agent_tool_loop_refuses_git_stash' crates/roko-cli/tests/ && cargo test -p roko-cli --test secrets_and_git_guard_canary agent_tool_loop_refuses_git_stash"

[[verify]]
command = "grep -q 'check_git_command' crates/roko-acp/src/builtin_tools.rs && grep -rqw 'fn acp_bash_refuses_git_stash' crates/roko-acp/ && cargo test -p roko-acp acp_bash_refuses_git_stash"

[[verify]]
command = "grep -q 'inherit_gate_env' crates/roko-cli/src/graph_execution/plan_verify.rs && grep -rqw 'fn plan_verify_step_sees_no_provider_key' crates/roko-cli/ && cargo test -p roko-cli plan_verify_step_sees_no_provider_key"

[[verify]]
command = "grep -A25 'async fn regression_output' crates/roko-cli/src/graph_execution/delivery.rs | grep -q 'gate_env' && grep -rqw 'fn regression_command_sees_no_provider_key' crates/roko-cli/ && cargo test -p roko-cli regression_command_sees_no_provider_key"

[[verify]]
command = "grep -rqw 'fn pause_holds_next_task_until_resume' crates/roko-cli/tests/ && cargo test -p roko-cli --test pause_canary pause_holds_next_task_until_resume"

[[verify]]
command = "grep -rqw 'fn rest_pause_holds_the_run_without_cancelling' crates/roko-serve/ && cargo test -p roko-serve rest_pause_holds_the_run_without_cancelling"

[[verify]]
command = "grep -rqw 'fn plan_pause_without_a_running_plan_fails' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_control_ack plan_pause_without_a_running_plan_fails"

[[verify]]
command = "grep -rqw 'fn conductor_supervise_false_keeps_stall_watchdog' crates/roko-cli/ && cargo test -p roko-cli conductor_supervise_false_keeps_stall_watchdog"

[[verify]]
command = "grep -rqw 'fn default_config_builds_no_daimon_state' crates/roko-cli/ && cargo test -p roko-cli default_config_builds_no_daimon_state"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK04, slice 12xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1201 | S | p1 | roko's own bash git guard lets git stash, clean, checkout and restore through | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1201-bash-git-guard-denies-stash-clean-checkout-restore.md` |
| 2 | 1202 | S | p2 | Canary C2 does not check that roko's tool loop refuses git stash and keeps the operator's edit | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1202-canary-c2-tool-loop-refuses-git-stash.md` |
| 3 | 1203 | S | p2 | ACP's bash tool runs no git guard at all | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1203-acp-bash-runs-the-git-guard.md` |
| 4 | 1204 | S | p1 | The whole-plan check runs agent-written code with roko's full environment | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1204-whole-plan-check-starts-from-the-gate-environment.md` |
| 5 | 1205 | S | p1 | The delivery regression runs agent-written code with roko's full environment | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1205-delivery-regression-starts-from-the-gate-environment.md` |
| 6 | 1207 | M | p1 | Graph pause is acknowledged and shown as PAUSED, but nothing reads the flag | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1207-pause-holds-new-tasks-and-plans-until-resume.md` |
| 7 | 1208 | M | p2 | REST, portal and client-TUI pause cancel the run instead of holding it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1208-rest-portal-and-client-tui-pause-use-the-same-hold.md` |
| 8 | 1209 | M | p2 | roko plan pause, resume, cancel and retry exit 0 whether or not a run took the command | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1209-plan-control-commands-report-the-runs-answer.md` |
| 9 | 1210 | S | p3 | Add [conductor] supervise (default on) so the conductor can be switched off without losing the stall watchdog | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1210-conductor-tick-off-by-default-stall-watchdog-stays.md` |
| 10 | 1211 | S | p2 | Held affect is appraised on every plan run by default and shifts the routing tier unrecorded | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1211-affect-off-by-default-on-plan-runs.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1200-operator-control-held-items-off-secrets-and-guards.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-acp/src/builtin_tools.rs`, `crates/roko-agent/src/safety/git.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/graph_execution/delivery.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_execution/plan_set.rs`, `crates/roko-cli/src/graph_execution/plan_verify.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/graph_task_dispatch/supervision.rs`, `crates/roko-cli/src/inject.rs`, `crates/roko-cli/src/inject/transport.rs`, `crates/roko-cli/src/runner/types.rs`, `crates/roko-cli/src/serve_client.rs`, `crates/roko-cli/tests/pause_canary.rs`, `crates/roko-cli/tests/plan_control_ack.rs`, `crates/roko-cli/tests/secrets_and_git_guard_canary.rs`, `crates/roko-core/src/config/execution.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-serve/src/routes/plans/run_control.rs`, `crates/roko-serve/src/routes/plans/tests.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: nothing.
- Suggested model: opus.

## Progress

Worker run of 2026-10-02 on `work/gap-198c9c` (base `976220c3e`). No cargo ran; each task is implemented, its
static verify passes, and cargo verification is deferred to the batch gate.

- 1201: implemented at 2243425d7
- 1202: implemented at 6b2c93f0f
- 1203: implemented at ed2abe432
- 1204: implemented at ad5c477d1
- 1205: implemented at 256c9ab86
- 1207: implemented at 90db19560
- 1208: implemented at 1eed55b82
- 1209: implemented at 60d16cbb7 (its canaries run under /tmp from 302cb8d10)
- 1210: implemented at 257f3ff51
- 1211: implemented at 0ab6fcd11

Notes: 1201 also denies any `git push` behind the same switch (the task's recommended parity), so six existing
push tests in `safety/git.rs` now use a policy with the switch off. 1208 delivers pause and resume to a server-hosted
run through `.roko/state/control.json`, which the run still reads, because roko-serve cannot call roko-cli's inject
client. On macOS the default temp dir makes a run's inject socket path 104 bytes or more (1224), so `pause_canary`
and `plan_control_ack` root their workspace under `/tmp`.
