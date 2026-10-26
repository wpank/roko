+++
id = "gap-4dd77d"
kind = "gap"
title = "A generated draft whose only defect is a verify written as a string, or a task over its tier's size limits that a higher tier fits, goes back to the planner for a whole new draft: 3 of round v7's 11 rejected drafts ($0.18) could have been repaired in place"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "4cfc6be60"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 7), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::validate_and_fix_generated_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::repair_generated_draft", "crates/roko-cli/src/plan_policy.rs::validate_tier_sizes"]
links = { depends_on = [], blocks = [], related = ["gap-b95913"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_string_verify_step_is_repaired_not_regenerated' crates/roko-cli/src && grep -rqw 'fn a_task_over_its_tier_is_raised_not_regenerated' crates/roko-cli/src && cargo test -p roko-cli --lib a_string_verify_step_is_repaired_not_regenerated && cargo test -p roko-cli --lib a_task_over_its_tier_is_raised_not_regenerated"

[[verify]]
command = "grep -rqw 'fn a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised' crates/roko-cli/src && cargo test -p roko-cli --lib a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:02:09Z"
commit = "8e3f70bbe"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 8e3f70bbe (branch f11154c5e; main merged in at 14dd020b3). Post-merge verify in main at 8e3f70bbe: the greps find the three tests and a_string_verify_step_is_repaired_not_regenerated, a_task_over_its_tier_is_raised_not_regenerated and a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised pass (3 of 3). Branch: plan_generate and plan_policy 193/193; roko-cli lib 4034 passed, its other failures the two frozen_learning_* tests already failing on main since 754f45ebd; nightly fmt; clippy -p roko-cli -D warnings clean. A task over its tier's limits takes the smallest tier it fits (sizes::raise_tiers, the last repair); a stated size never lowers a task below a tier whose limits it breaks; a verify of bare commands becomes step tables (wrap_command_steps). $0 replay of phase 4.3 on Undertow 2ac4b36 with the as-written draft then the discarded regeneration (tmp/hermes-roko-v7/roko-planner/replay-p43): before 12 of 17 written, after the regeneration kept with T1 integrative, 16 kept and P4.07 dropped with a reason; 4.2's drafts 1 call; phase 5's eight-task ideal draft 1 call, 21 of 21."
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

Golden path: cost, and, as phase 4.3 showed (below), a plan's coverage of its source's checks.
- In round v7, 11 of the 32 planner calls produced drafts that roko threw away: $0.756.
- These two shapes were 3 of them, $0.18.
- At gpt-5.4's prices a regeneration costs $0.10–0.16, and the planner model follows the ladder's strongest rung
  unless `[authoring] planner_model` is set.

## Evidence: phase 4.3, a coverage loss (2026-10-26, hermes-pilot; raised to p2 by the lead)

Undertow phase 4.3's plan was written from a chat: op `ff02a005`, roko f50dc58fa, planner gpt-5.4-mini. The log is
Undertow's `.roko/roko.log.2026-10-26`, 12:20:02–12:20:25Z.

1. **The draft dropped 5 of the spec's 16 checks** (34,990 in / 1,521 out, $0.0400).
   - It left out T1's four regression checks: `tools/spec.sh 4_2 layout`, `4 editor`, `4 display 'P4.05*'` and
     `4 steps`.
   - It also left out T2's `tools/spec.sh 4 display 'P4.07*'`.
   - It validated: T1 was focused, with 1 step.
2. **The source-check coverage re-ask's regeneration put T1's five steps back** (14,571 in / 1,475 out, $0.0286).
3. **roko threw the regeneration away and wrote the draft.** It logged "plan generate: regeneration not kept … why=its
   plan does not validate: … PLAN_TIER_SIZE [T1]: focused task is over its tier's size limits: 5 verify steps (limit
   4); split it into smaller tasks, or raise its tier to integrative".
   - The chat's plan card read "YOUR SPEC'S CHECKS 12 of 17 kept · 5 missing · asked again".

**So the tier rule discarded the regeneration that had restored coverage, and kept the draft that lacked it.**
- A person caught the gap: hermes-pilot's plancheck counted 11 of the 16 spec commands run.
- The person fixed it by hand: T1 raised to integrative with its four regression steps, T2's joined step split
  (gap-77220d) and P4.07 added. This is in Undertow `plans/p4-3-the-editor-s-finish/plan.md`, "Review edits
  (person)".
- roko logged "repaired the draft" for the regeneration too. So the raise this item asks for, in
  `repair_generated_draft`, would have kept the 17-of-17 plan for no extra call.

## Evidence: phase 5, two more regenerations (2026-10-26, hermes-pilot)

Undertow phase 5's plan was written from a chat: op `79f278ed`, roko e90161ec4, planner gpt-5.4-mini. The log is
Undertow's `.roko/roko.log.2026-10-26`, 15:08:02–15:09:03Z.

- **The draft and regeneration 1 were both rejected for the same thing:** "PLAN_TIER_SIZE [T8]: focused task is over
  its tier's size limits: 5 verify steps (limit 4); split it into smaller tasks, or raise its tier to integrative"
  (15:08:33Z and 15:08:53Z).
- **Regeneration 2 raised T8 to integrative** and was kept: "retry succeeded attempt=2".
- **The cost:** the two extra calls cost **$0.0918** ($0.0716 + $0.0202) of the write's $0.1595 planner spend. That's
  58%, for a raise the validator's own message names.
- **Coverage:** every draft kept all 21 source checks, so this case cost money only, unlike phase 4.3's above.
- **Generation:** 60.4 s (`total_ms=60401`), about 20 s of it on each rejected draft.

## Evidence: phase 5.1, a second coverage loss (2026-10-26, hermes-pilot)

Undertow phase 5.1's plan was written from a chat: op `42c4f74d`, roko 74c3594bc, planner gpt-5.4-mini. The log is
Undertow's `.roko/roko.log.2026-10-26`, 16:43:58–16:44:20Z.

1. **The draft dropped 7 of the spec's 17 commands** ($0.0329). It left out T1's three filter regressions (`tools/spec.sh
   5 filter`, `2_2 filter`, `2_1 filter`) and T3's four (`5 number`, `4_2 head`, `4_2 layout`, `4_3 finish`). It
   validated.
2. **The source-check coverage re-ask restored them** ($0.0340).
3. **roko threw the re-ask away:** "regeneration not kept … PLAN_TIER_SIZE [T1]: mechanical task is over its tier's
   size limits: 4 verify steps (limit 3); split it into smaller tasks, or raise its tier to focused". It wrote the
   draft, and the chat's card read "11 of 18 kept · 7 missing · asked again".

A person restored coverage by hand: T1 raised to focused with its three steps, and T3's four steps added (6 steps,
integrative). hermes-pilot's plancheck went from 1 finding to 0. The edits are in Undertow
`plans/p5-1-the-renders-as-they-play/plan.md`, "Review edits (person)".

**That's two coverage losses (4.3 and 5.1) in three plan writes,** plus phase 5's two cost-only regenerations, all for
the raise the validator's own message names.

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
- A coverage re-ask's regeneration that restores missing source checks, and goes over its tier by doing so, is kept
  with the tier raised, not thrown away for the draft that lacks them (phase 4.3). Test:
  `a_coverage_regeneration_over_its_tier_is_kept_with_the_tier_raised`.
- The `[[verify]]` passes.

## Notes

- The other rule behind round v7's rejections, PLAN_CONTEXT_SYMBOL ($0.372, two of them phase 4.2's), goes to
  roko-planner-2's item with its root cause: generated plans get the FAST lane's 3-file read budget
  (`plan_policy.rs:116`), and the spec-first note takes one slot. It is not part of this item.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 7.
