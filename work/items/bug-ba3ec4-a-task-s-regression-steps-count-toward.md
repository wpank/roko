+++
id = "bug-ba3ec4"
kind = "bug"
title = "A task's regression steps count toward its tier's step limit, so roko raised phase 5.4's 5-line value retune (1 new check, 6 pass_on_base regressions, 7 steps) from integrative to architectural and its start rung to strong, where 5.2 and 5.3 passed on mid"
status = "blocked"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-28
updated = 2026-10-28
last_verified = 2026-10-28
source = "round-v7 team-lead: phase 5.4 plan write, op 17dc90dd, roko-main-38bedecea"
discovered_from = "agent:hermes-pilot (round v7, phase 5.4), the lead's ask"
anchors = ["crates/roko-cli/src/plan_policy.rs::TierSizeLimits", "crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers_for", "crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers", "crates/roko-cli/src/plan_policy.rs::validate_tier_sizes"]
links = { depends_on = [], blocks = [], related = ["gap-4dd77d", "gap-4cca5f", "gap-9e212c", "gap-de0b87", "gap-634954"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn regression_steps_do_not_raise_a_tasks_tier' crates/roko-cli/src && cargo test -p roko-cli --lib regression_steps_do_not_raise_a_tasks_tier"
+++

## Problem

A task's regression steps count toward its tier's verify-step limit as fully as its new checks, so roko raises a
small task to a higher tier, and with it a higher start rung and a higher price, for the steps that only re-run
checks that already pass.

Phase 5.4's real plan write on roko-main-38bedecea (operation 17dc90dd, plan
`~/dev/undertow/plans/p5-4-kick-companion-as-named`, Undertow 687e545) shows it:

- The spec sizes the work as "S, 1 task, about 5 lines of preset values", in `core/src/presets.cpp`.
- The planner wrote T1 as integrative, with one file, `max_loc = 20` and 7 verify steps. One step,
  `tools/spec.sh 5_4 presets`, is `fail_on_base`. The other six, P5.4.07 to P5.4.12, are `pass_on_base`
  regressions (`tools/spec.sh 5 presets`, `5_1 render`, `5_3 presets`, `5_2 presets` and the two render-demos
  checks).
- roko's repairs list "T1: tier integrative became architectural, to fit its 7 verify steps (limit 6)".
  Architectural starts on strong (gpt-5.4) under Undertow's `[routing.ladder.start]`. Phases 5.2 and 5.3, the same
  kind of retune, passed on mid (gpt-5.4-mini) for about $0.11 (the lead's figures).
- The spec's "about 5 lines" would make it mechanical, but `sizes::repair_tiers` doesn't lower a task below a tier
  whose other limits it breaks. Its 7 steps break mechanical's, focused's and integrative's, so it kept integrative.

## Why it matters

- **Cost.** The start rung follows the tier. A task priced at strong for its regression list costs several times
  what its own work needs.
- **More of these to come.** Later phases keep adding earlier phases' checks as regressions. 5.2 had 4, 5.4 has 6,
  so each later phase gets more likely to cross a limit.
- **Contradiction.** The tier ends up sized by checks that pass before any edit, against the spec's own size.
- Goal: golden-path. Related: gap-4dd77d (the raise for steps), gap-4cca5f (the early raise), gap-9e212c (the
  spec's stated size), decision 3203 (note in gap-de0b87).

## Where

- `crates/roko-cli/src/plan_policy.rs::TierSizeLimits` sets the step limits: mechanical 3, focused 4, integrative 6,
  architectural 8. `exceeded_with` and `exceeded_by` count every verify step, whatever its `expect`.
- `crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers_for` is gap-4cca5f's early raise, before
  `assign_runs`. It raised 5.4's T1, at the draft's own 7 steps; the line comes before the check-read lines.
- `crates/roko-cli/src/plan_generate/sizes.rs::raise_tiers` is gap-4dd77d's raise, last. It would raise the same
  task.
- `crates/roko-cli/src/plan_policy.rs::validate_tier_sizes`, rule PLAN_TIER_SIZE, binds at generation (3225,
  decision 3203). Without a raise, this draft would have gone back to the planner for a split or a higher tier.
- `crates/roko-cli/src/plan_generate/sizes.rs::repair_tiers` (gap-9e212c) won't lower a task below a tier whose
  limits, LOC aside, it breaks.

## Current state

- Steps carry `expect = "pass_on_base"`, and the source's checks carry `regression`, so regressions are known by the
  time tiers are judged.
- Since gap-4cca5f, a step roko adds for a missing source check counts at the early raise too. So a draft that leaves
  regressions out is raised just as a draft that writes them, as 5.4's draft did.
- The pilot is testing whether a person's edit back to integrative validates. `plan validate` reports
  PLAN_TIER_SIZE as a warning for a hand-edited plan, so `--strict` fails on it, and `plan run` does not check tier
  sizes.

## Plan

**Waits on Will.** Options A and B change what PLAN_TIER_SIZE measures, or whether it binds. Decision 3203 made it
an error at generation, with limits that move only on measured first-try pass rates (0.80 target, at least 20 tries
per bucket; note in gap-de0b87, gap-634954). Option C keeps 3203 as it is and needs only the lead's call.

- **A. Count only the steps that don't pass on the base** toward a tier's step limit, in `exceeded_with`, which
  PLAN_TIER_SIZE, both raises and `repair_tiers` read.
  - 5.4's T1 counts 1 step. The spec's "about 5 lines" then makes it mechanical, on cheap.
  - The step limit becomes a measure of the task's new work. The task must still keep the regressions passing,
    and no pass rate is measured yet for regression-heavy tasks on cheap rungs.
  - Variant: count all of a task's regression steps as one step. T1 counts 2 and is still mechanical.
- **B. Let a tier hold regression steps past its step limit**, with a warning in place of a raise or a refusal.
  - T1 stays integrative, on mid, as 5.2 and 5.3 ran.
  - PLAN_TIER_SIZE no longer binds for those steps.
- **C. Keep the limit, and fit the regressions into the tier's room.** When a task's steps would cross its tier's
  limit only because of `pass_on_base` steps, join those steps into fewer `&&` steps rather than raise the tier.
  The planner's own steps are joined too.
  - Each joined step runs the commands it joins once each, so two render-demos checks run render-demos once:
    `tools/render-demos.sh && tools/spec.sh 5_1 demos && tools/spec.sh 5 demos`.
  - T1 gets 2 steps and stays integrative, or mechanical with the spec's size.
  - Cost: in a joined step, the first regression to fail hides the ones after it (gap-77220d's concern). Each of
    them passes on the base, so any failure there still means the change broke something.
  - Needs no change to 3203.

Recommendation: A gives the plainest size, if Will accepts a step limit that counts only new work. Otherwise C.
Either way, test on phase 5.4's draft (fixture from the plan above): T1 is not raised for its six regressions.

## Done when

- Phase 5.4's draft is written without T1 raised for its regression steps: its tier is the one its own work needs,
  with the spec's size where the spec states one.
- 5.2 and 5.3's drafts keep their tiers. The `[[verify]]` passes.

## Notes

- Evidence: `~/dev/undertow/plans/p5-4-kick-companion-as-named/` (`tasks.toml`, and `plan.md` "## Repairs").
  Undertow is read-only for agents: copy it into a fixture with `git archive`.
- Don't code it before the choice. Under A or B this changes a recorded decision, so it goes to the lead and Will
  first.
