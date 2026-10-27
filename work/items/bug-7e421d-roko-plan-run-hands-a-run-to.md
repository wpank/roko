+++
id = "bug-7e421d"
kind = "bug"
title = "roko plan run hands a run to the workspace's server without --fail-fast, --promote or --force, and says nothing"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan-run"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-d2b605 (2026-10-27)"
discovered_from = "gap-d2b605"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan", "crates/roko-cli/src/serve_client.rs::run_plan_via_server"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-d60281", "gap-d2b605"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for v in fail_fast promote; do sed -n '/discover_workspace_server(&wd)/,/\\.await;/p' crates/roko-cli/src/commands/plan.rs | grep -qw \"$v\" || exit 1; done && sed -n '/discover_workspace_server(&wd)/,/\\.await;/p' crates/roko-cli/src/commands/plan.rs | grep -qE '(^|[^_a-z-])force([^_a-z-]|$)'"
+++

## Problem

When a live `roko serve` owns the workspace, `roko plan run <dir>` hands the run to it: the
`discover_workspace_server(&wd)` branch of `PlanCmd::Run` in `commands/plan.rs` calls
`serve_client::run_plan_via_server`. Before that it refuses the flags the server cannot honour:
`--config`, `--frozen-learning`, `--no-holdout` and `--no-audit` in that branch, and `--max-retries`,
`--max-tasks`, `--budget-override`, `--no-budget`, `--model`, `--dangerously-skip-permissions`,
`--log-file`, `--[no-]worktree-per-task`, `--rich-topology` and `--resume-plan` in
`run_plan_via_server`. Three flags are neither passed on nor refused, so the server's run goes ahead
without them and nothing says so. The server builds its run with `fail_fast: false`,
`promote: None` and `force_disk_check: false` (`serve_runtime.rs`):

- `--fail-fast`: plans still start after a plan fails, and spend;
- `--promote <branch>`: the batch is not promoted into BRANCH or tagged `roko/run/<run-id>`;
- `--force`: the run still makes its disk-space check, which can refuse it.

How to see it: `sed -n '/discover_workspace_server(&wd)/,/\.await;/p' crates/roko-cli/src/commands/plan.rs`
names none of `fail_fast`, `promote` or `force`.

## Why it matters

gap-d60281 settled that `plan run` stops on a flag it cannot honour rather than run without it. An
operator who passes `--fail-fast` to stop spending after a failure, or `--promote` to get a branch,
gets neither whenever a server owns the workspace.

## Where

- `crates/roko-cli/src/commands/plan.rs::cmd_plan`, `PlanCmd::Run`: the `discover_workspace_server`
  branch and its call of `run_plan_via_server`.
- `crates/roko-cli/src/serve_client.rs::run_plan_via_server`: the `refused_*` parameters and the
  refusal list. `ServeClient::submit_plan_run` sends only `target`, `resume` and
  `max_parallel_plans`.
- `crates/roko-cli/src/serve_runtime.rs` (the server's `GraphPlanRunParams`) and
  `crates/roko-serve/src/runtime.rs::PlanRunOptions`, which has no field for any of the three.

## Current state

Checked 2026-10-27 at 0c8c3d69a. `--force-resume` is not affected: a server run sets
`force_resume` from `resume` (`crates/roko-serve/src/routes/plans/run_control.rs`), so every server
run that resumes already sets a mismatched checkpoint aside.

## Plan

1. Refuse the three in `run_plan_via_server` as it refuses the other server-incompatible flags
   (`refused_fail_fast`, `refused_promote`, `refused_force`), with the same message.
2. Test that each is refused while a server owns the workspace.
3. Say so in `docs/v3/28-CLI.md` §`roko plan run`.

Forwarding `--fail-fast` instead would need a `fail_fast` field in the execute body and in
`PlanRunOptions`; `--promote` moves branches of the server's checkout, so refusing it is the safer
choice.

## Done when

`roko plan run plans/<x> --fail-fast` (likewise `--promote <branch>` and `--force`) while a server
owns the workspace exits with the refusal instead of starting the run, a test covers it, and
`[[verify]]` passes. The verify is static: the server branch of `PlanCmd::Run` must name each of the
three, as it has to in order to refuse or forward them.

## Notes

Found while doing gap-d2b605 (28-CLI.md's `plan run` flags).
