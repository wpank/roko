+++
id = "gap-d21da9"
kind = "gap"
title = "plan validate --dynamic runs nothing for strategist tasks or tasks whose steps are all pass_on_base, so a plan whose 11 checks all fail on the base gets 0 diagnostics and B grades"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#B-baseline"
discovered_from = "agent:hermes-pilot (round v7 baseline, 2026-10-19)"
anchors = ["crates/roko-cli/src/spec_red_on_base.rs::check_plan"]
links = { depends_on = [], blocks = [], related = ["bug-27f00b", "gap-ff78e1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pass_on_base_steps_that_fail_on_the_base_are_reported' crates/roko-cli/src && cargo test -p roko-cli --lib -- pass_on_base_steps_that_fail_on_the_base_are_reported"
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
