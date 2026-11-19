+++
id = "gap-d21da9"
kind = "gap"
title = "plan validate --dynamic runs nothing for strategist tasks or tasks whose steps are all pass_on_base, so a plan whose 11 checks all fail on the base gets 0 diagnostics and B grades"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "f33fd217c"
source = "round-v7-hermes-pilot#B-baseline"
discovered_from = "agent:hermes-pilot (round v7 baseline, 2026-10-19)"
anchors = ["crates/roko-cli/src/spec_red_on_base.rs::check_plan"]
links = { depends_on = [], blocks = [], related = ["bug-27f00b", "gap-ff78e1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pass_on_base_steps_that_fail_on_the_base_are_reported' crates/roko-cli/src && cargo test -p roko-cli --lib -- pass_on_base_steps_that_fail_on_the_base_are_reported"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T14:48:07Z"
commit = "b63de5643"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged f55e30d4f (b63de5643). spec_red_on_base.rs: RedOnBaseOptions::unscored, set only by plan validate --dynamic (the plan-load gate keeps its old skips, so roko run never starts by running a workspace's regression gates): a regression-only task runs its non-cargo steps, and a failing pass_on_base step is base_broken; a non-implementer task runs the steps the base settles, else it is not run with the reason. New outcomes: never_passes (a step that fails in both runs and reads only files no task writes, for any task, scored implementers included, with no SQ06 credit) and not_scored. render_text counts not run by each reason and lists every task whose step cannot pass or could not be told about. Verify at f55e30d4f: pass_on_base_steps_that_fail_on_the_base_are_reported passes. On the round v7 baseline plan, plan validate --spec-quality --dynamic now reports fail: no task can make a step pass 5, with one line per task, where it said not run: no implementer verify 5; the static part adds 11 PLAN_054 errors (exit 1, was 0). The speclint parity fixture keeps its expected outcomes (the quick-reviewer task stays not_run)."
+++

## Problem

The plan from the round v7 baseline (bug-27f00b) is
`/Users/Shared/undertow-oneliner/plans/create-a-roko-plan-for-a-macos-vst3-synthesizer`. It has 5 strategist tasks and
11 verify steps, all `expect = "pass_on_base"`. All 11 exit 1 on the base: each greps the hello-world demo's
`plan.md` for a phrase that isn't there.

`roko plan validate --spec-quality --dynamic plans/create-a-roko-plan-for-a-macos-vst3-synthesizer` (roko 3b34aed92):

```
0 diagnostics in 1 plan
…
5 tasks: 0 A, 4 B, 1 C, 0 D; 0 with hard fails

red on base (tasks)
  not run: no implementer verify               5
```

So `--dynamic`, the flag that exists to run checks on the base, ran none of them, and the plan reads as healthy.

## Why it matters

`plan validate --dynamic` is the check a person (and the round's runbook) uses before spending on a run. A plan whose
every check fails on the base, and can never pass, gets a clean report.

## Where

`crates/roko-cli/src/spec_red_on_base.rs::check_plan` skips two kinds of task:
- a task whose role is not implementer ("not run: a strategist task");
- a task whose every step is `expect = "pass_on_base"` ("not run: every step is a regression check").

The module's own contract lists "unknown: … a `pass_on_base` step fails" (`Outcome::BaseBroken`), but such a step is
never run when all of its task's steps are `pass_on_base`. The summary line then says "no implementer verify", though
the reason here is both skips.

## Plan

1. Run the steps of non-implementer tasks and of `pass_on_base`-only tasks too, when they are cheap: no cargo step,
   or with `CargoSteps::Skip` leaving the cargo ones out. That keeps the skip for the workspace's cargo regression
   gates, which are what made it worth skipping.
2. Classify what they show:
   - a `pass_on_base` step that fails on the base is BaseBroken, said as a diagnostic;
   - a non-implementer task's step that fails on the base, when no task writes the files it reads, can never pass.
     Report that as a hard fail.
3. Make the summary line name the real reason for each skip.

## Done when

On the baseline plan, `plan validate --spec-quality --dynamic` reports the 11 failing steps. A named test covers a
`pass_on_base`-only task whose step fails on the base: see the `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/baseline/validate-spec-quality-dynamic.txt` and `plan/`.
- Related: gap-ff78e1 (validate lacks generation's runnable checks), and bug-27f00b (the plan-writing side).
