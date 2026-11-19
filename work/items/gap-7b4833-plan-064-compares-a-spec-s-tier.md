+++
id = "gap-7b4833"
kind = "gap"
title = "PLAN_064 compares a spec's tier word with its size only: 7.1's \"Focused: about 20 lines\" task states five steps (three regressions), more than focused's 4, so it was planned integrative and ran on mid with no warning"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "48fb93ca2"
source = "round v8: the lead's note on phase 7.1 (plan p7-1-silent-until-played, op 897233c8)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/sizes.rs::tiers_under_sizes", "crates/roko-cli/src/plan_policy.rs::for_tier"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-ba3ec4", "gap-29c1e8", "gap-4dd77d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_tier_word_too_small_for_its_steps_is_found' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_tier_word_too_small_for_its_steps"
+++

## Problem

PLAN_064 (gap-29c1e8) compares a spec's tier word with the size it states, and nothing else. A tier also bounds a
task's verify steps (focused 4, integrative 6), files and description words. So a task whose stated checks no task of
that tier can hold gets a higher tier with no warning.

- **The spec.** Round v8's phase 7.1 (Undertow `docs/specs/phase-7.1-silent-until-played.md`) says "1. **The own
  clock.** Focused: about 20 lines". Its files are 3 paths, and its checks are "P7.1.01 to P7.1.07 (one step),
  P7.1.08, then the regression checks P7.1.09, P7.1.10 and P7.1.11 (`expect = "pass_on_base"`)". That is five steps.
- **The plan.** The plan as written (`p7-1-silent-until-played`, op 897233c8) made T1
  integrative with `max_loc = 40`. Its five steps are two `fail_on_base` steps and three `pass_on_base` regressions.
- **The tier.** No repair raised it: the planner chose integrative, because focused holds only 4 steps. So T1 ran on
  mid ($0.109) and not the cheap rung the spec named.
- **Intended under today's rules.** The step limit counts regression steps. bug-ba3ec4 asks whether it should, and is
  blocked on Will's decision. With regressions not counted, T1 has 2 steps and is focused.

## Why it matters

- A spec author can't see from `plan validate` that a tier word is too small for the checks the spec gives the task.
  They learn it from the run's rung and cost.
- Goal: core.

## Where

- `crates/roko-cli/src/plan_generate/sizes.rs`: `tiers_under_sizes`, which reads only sizes.
- `crates/roko-cli/src/plan_policy.rs`: `TierSizeLimits::for_tier`, the tiers' step and file limits.

## Plan

1. When the notes for the planner state a task's checks or files (the "Checks:" and "Its files are those N paths"
   lines), PLAN_064 also compares the tier word with the steps and files they come to. The warning names the limit
   the tier word breaks: "focused at five steps, which needs integrative: focused holds 4".
2. Whether regression steps count follows bug-ba3ec4's decision.
3. Add the test `a_tier_word_too_small_for_its_steps_is_found`, on phase 7.1's notes for the planner: T1 is found
   (five steps, focused holds 4), and phase 7's notes stay quiet.

## Done when

- The `[[verify]]` command passes.

## Notes

- From the lead's note on phase 7.1 (round v8). Investigated read-only: the plan's T1 has 5 steps, and the plan
  writing status has no tier repair.
- Related: bug-ba3ec4 (regression steps and the step limit), gap-29c1e8 (PLAN_064), gap-4dd77d (`raise_tiers`).
