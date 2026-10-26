+++
id = "gap-8b1f4f"
kind = "gap"
title = "A source check that comes with its own command can be dropped with any reason that isn't a restatement, \"covered by the plan's final check\" included, though the planner's rules allow a drop only when no command can check it (phase 4.3's re-ask draft dropped P4.3.15 that way)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 9, run 3752b8ff), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage", "crates/roko-cli/src/plan_generate/source_checks.rs::is_restatement", "crates/roko-cli/src/plan_generate/source_checks.rs::planner_block"]
links = { depends_on = [], blocks = [], related = ["gap-4dd77d", "gap-77220d", "gap-343a23"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_with_its_own_command_is_never_dropped' crates/roko-cli/src && cargo test -p roko-cli --lib a_check_with_its_own_command_is_never_dropped"
+++

## Problem

A plan may drop a source check by listing it in `[meta] dropped_checks` with a reason. The planner's rules allow
that only "when no command can check it (it needs a person, hardware, or another plan's work first)". roko's
coverage check is looser: it accepts any reason that isn't a restatement of the check. That includes "it is covered
by the plan's final check", even for a check that comes with its own command, which by definition a command can
check.

**Phase 4.3's plan write** (Undertow, op ff02a005, gpt-5.4-mini):
- The spec gives each check to a task. Its "Notes for the planner" put P4.3.09–12 in task 1's steps and P4.3.13–16
  in task 2's (`docs/specs/phase-4.3-finish.md`, lines 146–156).
- **The first draft** left out P4.3.09–12 and P4.3.15. The coverage counted the 5 as missing and re-asked.
- **The re-ask's draft** (`.roko/plan-drafts/p4-3-the-editor-s-finish/rejected-1.md`) listed all five regression
  checks in `dropped_checks`, each with "covered by the plan's final check `tools/check.sh --plugin`".
  - It wrote four of them into T1's steps anyway.
  - It left P4.3.15, `tools/spec.sh 4 display 'P4.07*'`, out.
- **That reason passes `is_restatement`**: for P4.3.15, 8 of its 18 words are in the check's text, under the 70%
  threshold. So had the draft been kept (gap-4dd77d), P4.3.15 would have counted as dropped, not missing, and
  nothing would have asked for it.
- The person added it back by hand.

## Evidence: the phase-4.3 replay after gap-4dd77d (2026-10-26, roko-planner)

- A $0 replay of 4.3's write on Undertow 2ac4b36 (`tmp/hermes-roko-v7/roko-planner/replay-p43`: a fake planner
  answers with the as-written draft, then the re-ask's draft) on gap-4dd77d's binary kept the re-ask's draft, with T1
  raised to integrative: "Source checks: 16 of 17 kept as verify steps, 1 dropped with a reason".
- The one dropped is SC15, P4.3.15 (`tools/spec.sh 4 display 'P4.07*'`), with the draft's reason "The hazard-stripes
  regression is not a task-specific change in this phase; it is covered by the plan's final check
  `tools/check.sh --plugin`". So the path this item describes is now the live one.
- `assign_runs` gave it no step because `owner_of` found no owner. Its command's word `display` doesn't match T2's
  `plugin/ui/Displays.cpp`, whose stem is `displays`, and nothing in T2's text names P4.3.15.
- gap-4dd77d's test `a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised` pins (16 kept, 1 dropped,
  0 missing, 17) for now. This item's fix should turn it into 17 kept.

## Why it matters

- The checks a spec gives a task are how a task's own try catches its regressions. Moved to the plan's final
  check, a regression surfaces only at the end, as a hand-back round. In phase 4 that round cost $1.13 and was
  cancelled.
- A drop counted as "dropped with a reason" doesn't trigger the coverage re-ask, and reads as accepted.
- Goal: golden-path.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::SourceCoverage`: `of` builds `dropped` from
  `dropped_checks`, keeping any reason that isn't a restatement and whose check isn't kept.
- `crates/roko-cli/src/plan_generate/source_checks.rs::is_restatement`: the only test a reason must pass.
- `crates/roko-cli/src/plan_generate/source_checks.rs::planner_block` and `regeneration_prompt`: the rules the
  planner is given.
- A `SourceCheck` with `run` set is a check whose source gives its own command.

## Current state

- `missing()` drives the coverage re-ask, and dropped checks aren't in it.
- Related:
  - gap-4dd77d (p2): the re-ask's better draft was thrown away over T1's tier.
  - gap-77220d: missing checks joined into one `&&` step.
  - gap-343a23 (done): run a source's own command alone.

## Plan

1. In `SourceCoverage::of`, don't accept a `dropped_checks` entry for a check that has its own command (`run`). It
   stays missing, or `cannot_run` when a step that runs it can't pass. Such a check can always be run, so "no command
   can check it" can't hold.
   - If an exception is needed, take one where the command depends on another plan's work, and name that plan in
     the reason.
2. Say so in `planner_block` and `regeneration_prompt`: a check with its own command is never dropped, and it's
   never "covered by" the plan's final check.
3. Add the test `a_check_with_its_own_command_is_never_dropped`. A plan whose `dropped_checks` gives "covered by
   the plan's final check" for a check with a `run` command reports it as missing.

## Done when

- The `[[verify]]` passes.
- Rewriting phase 4.3's re-ask draft through the coverage check reports P4.3.15 as missing.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 9, "The plan write on mini against 4.2's". The
  pilot's as-written copies are in `tmp/hermes-roko-v7/pilot/phase4-3/`.
- The op's `source_checks` reported `dropped: 0, missing: 5` for the kept draft, so the first draft's loss was
  caught. The gap is the second draft's path.
