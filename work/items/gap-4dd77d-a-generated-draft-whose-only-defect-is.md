+++
id = "gap-4dd77d"
kind = "gap"
title = "A generated draft whose only defect is a verify written as a string, or a task over its tier's size limits that a higher tier fits, goes back to the planner for a whole new draft: 3 of round v7's 11 rejected drafts ($0.18) could have been repaired in place"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 7), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::validate_and_fix_generated_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::repair_generated_draft", "crates/roko-cli/src/plan_policy.rs::validate_tier_sizes"]
links = { depends_on = [], blocks = [], related = ["gap-b95913"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_string_verify_step_is_repaired_not_regenerated' crates/roko-cli/src && grep -rqw 'fn a_task_over_its_tier_is_raised_not_regenerated' crates/roko-cli/src && cargo test -p roko-cli --lib a_string_verify_step_is_repaired_not_regenerated && cargo test -p roko-cli --lib a_task_over_its_tier_is_raised_not_regenerated"
+++

## Problem

Plan generation checks each draft, and a draft that breaks a rule goes back to the planner for a whole new draft. Each
retry re-reads the repository with tools and rewrites the whole plan, at $0.02–0.16 a call. Two of the rules
round v7 tripped can be met mechanically:

1. **`verify` written as a string.** Phase 3's re-plan (op 3b13af5f, gpt-5.4-mini) wrote `verify =
   ["tools/check.sh"]` in `[meta]`, where step tables go. Parsing failed ("invalid type: string "tools/check.sh",
   expected struct VerifyStep"), and the draft was regenerated: $0.038.
2. **PLAN_TIER_SIZE.** Two drafts were regenerated, and the validator's own message names the fix: "or raise its
   tier to focused".
   - Phase 0 (op 437486f8) had an integrative task with 7 verify steps, against a limit of 6: $0.046.
   - Phase 4.1 (op 7cda5da7, gpt-5.4) had a mechanical task with 5 files and 4 steps, against limits of 3 and 3:
     $0.095.

Expected: roko repairs both in place and records each repair, as it already clamps line ranges and trims read files
(gap-b95913).

## Why it matters

Golden path, cost.
- In round v7, 11 of the 32 planner calls produced drafts that roko threw away: $0.756.
- These two shapes were 3 of them, $0.18.
- At gpt-5.4's prices a regeneration costs $0.10–0.16, and the planner model follows the ladder's strongest rung
  unless `[authoring] planner_model` is set.

## Where

- `plan_generate/pipeline.rs::validate_and_fix_generated_plan`: the text-level fix-ups before
  `TasksFile::parse_str`. A string `verify` fails parsing, so it has to be fixed here.
- `plan_generate/pipeline.rs::repair_generated_draft` (gap-b95913): mechanical repairs on the parsed plan before
  validation. It returns one line per repair, and the operation's result lists them (`repairs`).
- `plan_policy.rs::validate_tier_sizes`: already computes the smallest higher tier a task fits.

## Current state

- **Today's repairs:** `meta.total`; `max_loc` for document-only tasks; `read_files` beyond the budget; line ranges
  (`fit_ranges_to_files`).
- **Evidence:** Undertow's `.roko/plan-drafts/*/rejected-N.md` name each validator code. Each rejected draft is
  matched to its planner call and cost in `tmp/hermes-roko-v7/run-analyst/data/plan_writes_detail.csv`
  (`plan_writes.py`).

## Plan

1. **In `validate_and_fix_generated_plan`:** read the TOML as a value. Wherever `meta.verify` or a task's `verify`
   holds plain strings, turn each one into a step table (`{ phase = "test", command = "<string>" }`) and record the
   repair.
2. **In `repair_generated_draft`:** for each task `validate_tier_sizes` flags, raise its tier to the smallest one it
   fits, if any, and record it. For example: "T1: tier mechanical became focused, to fit its 5 files and 4 verify
   steps". A task no tier fits still goes back to the planner, because it needs a split.
3. Both repairs appear in the operation's `repairs`, so the person sees them when reviewing the plan.

**Design note:** raising a tier moves where the task starts on the ladder. The validator already suggests it, and a
repair costs nothing while a regeneration costs a full planner call doing the same. Whether a raise to architectural
should still ask for a split is a choice to make here.

## Done when

- A draft with `verify = ["cmd"]` is written with that step as a table, and no extra planner call. Test:
  `a_string_verify_step_is_repaired_not_regenerated`.
- A draft whose oversized task fits a higher tier is written with the tier raised, and no extra planner call. Test:
  `a_task_over_its_tier_is_raised_not_regenerated`.
- The `[[verify]]` passes.

## Notes

- The other rule behind round v7's rejections, PLAN_CONTEXT_SYMBOL ($0.372, two of them phase 4.2's), goes to
  roko-planner-2's item with its root cause: generated plans get the FAST lane's 3-file read budget
  (`plan_policy.rs:116`), and the spec-first note takes one slot. It is not part of this item.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 7.
