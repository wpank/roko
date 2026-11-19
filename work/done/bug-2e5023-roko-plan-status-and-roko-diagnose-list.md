+++
id = "bug-2e5023"
kind = "bug"
title = "roko plan status and roko diagnose list a plan's writing as tasks: on a plan roko wrote, the planner call's spend (pseudo task generate) and its episode (plan:generate:<plan>) show as 'task 7 (not in tasks.toml) running' and 'task 8', and count as the run's calls"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "85799278f"
source = "round v8 lead: phantom rows in phase 6's live plan status (Undertow p6-the-window, run 23837ede)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::diagnose_tasks", "crates/roko-cli/src/commands/diagnose.rs::RunRecords"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_lists_only_the_plans_tasks' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_lists_only_the_plans_tasks"

[closed]
at = 2026-11-13
at_ts = "2026-11-12T23:00:56Z"
commit = "85799278f"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-12T22:17:28Z"
forced = false
evidence = "85799278f on work/bug-2e5023: roko plan status and roko diagnose list only the plan's tasks. The phantom rows were the plan write's records, not cells: the planner call's cost row (task_id generate) and its episode (plan:generate:<plan>). RunRecords::load now leaves out authoring spend and direct-call (capture) episodes unless the plan has a task by that id, and diagnose_tasks lists recorded ids only when tasks.toml can't be read. Red first: plan_status_lists_only_the_plans_tasks failed on bd49800b5 with '■ task 4 (not in tasks.toml) stopped' / '■ task 5 …', now passes. On an APFS clone of ~/dev/undertow, every phase 1-6 showed two such rows with roko-item7d (repro/before-item8.txt); roko-item8a shows 0 and the tables are otherwise identical (repro/after-item8.txt). Branch checks: nightly fmt; roko-cli lib 4149 passed, 0 failed; bin tests (diagnose, plan_status, status) 21 passed; workspace clippy -D warnings clean; the binary builds."
+++

## Problem

The round v8 lead saw `task 7 (not in tasks.toml) running` and `task 8 (not in tasks.toml) running` in the live
`roko plan status` of Undertow's phase 6 (`p6-the-window`, six tasks, run 23837ede). They are not tasks. They are the
records of the plan's writing, which roko attributes to the plan:

- **task 7** is the cost record of the planner call, in `.roko/learn/costs.jsonl`: `plan_id = "p6-the-window"`,
  `task_id = "generate"` (`plan_authoring::GENERATION_SPEND_TASK_ID`), $0.22, at 20:55, six minutes before the run
  started.
- **task 8** is the planner call's episode in `.roko/episodes.jsonl`: `task_id = "plan:generate:p6-the-window"`, with
  `extra.plan_id` set by `agent_episode::build_capture_episode` and `extra.task_kind = "plan-generate"`.

`diagnose::RunRecords::load` keeps every cost record and episode of the plan from `since_ms` on. `since_ms` is when
the checkpoint replaced an earlier one, so on a plan's first run it is `None`, and everything is kept.
`diagnose_tasks` then lists every recorded id that the plan does not define, after the plan's tasks. `run_model`
titles them "(not in tasks.toml)". They read as running while the run goes (attempted, never recorded), and as
stopped after it. Every plan roko writes has these records, so every first run shows them, in `roko plan status`
and in `roko diagnose`.

They also skew the report:

- The run's `total_agent_calls`, tokens and unpriced calls count the planner call.
- When the checkpoint has no committed spend, the planner call's cost is in the run's cost fallback.
- On a failed run whose plan tasks all passed, the `generate` row reads failed and becomes the report's
  `failed_task`.

## Why it matters

Goal tooling. The status view is what a person watches during a run, and it showed two tasks that don't exist, both
"running". It is also the guide's terminal surface for round v8, and phantom rows mean a retake.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::RunRecords::load` keeps the writing's records.
- `crates/roko-cli/src/commands/diagnose.rs::diagnose_tasks` lists ids the plan doesn't define.
- `run_model` titles them "(not in tasks.toml)".

## Plan

1. Red test first: a run of a three-task plan that roko wrote, with the writing's cost record (written through
   `AuthoringSpend`) and its planner episode (`build_capture_episode`). `roko plan status` lists only the three
   tasks, and the run's calls leave the planner call out.
2. `RunRecords::load` drops the plan's writing: the spend that generation and revision record under their pseudo
   tasks, and the episodes of direct agent calls (`extra.task_kind`), unless the plan has a task by that id.
3. `diagnose_tasks` lists only the plan's tasks. It lists what the run recorded only when the plan's `tasks.toml`
   can't be read, as its note already says.

## Done when

The test passes, the `[[verify]]` passes, and a clone of Undertow's phase 6 shows its six tasks and nothing else.
