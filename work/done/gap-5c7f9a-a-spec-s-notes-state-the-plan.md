+++
id = "gap-5c7f9a"
kind = "gap"
title = "A spec's notes state the plan's task count (phase 4.1: \"One task\"), and nothing compares it with the draft: 4.1's plan had 2 tasks, the split went unasked and unsaid"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "a9a7a191f"
source = "round-v7-team-lead#task-count-lint"
discovered_from = "agent:hermes-pilot (round v7, phase 4.1 op 7cda5da7), the lead's ask"
anchors = ["crates/roko-cli/src/plan_generate/sizes.rs", "crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_generate/source_checks.rs::regeneration_prompt"]
links = { depends_on = [], blocks = [], related = ["gap-dc5d7c", "bug-3d0414"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_draft_with_a_different_task_count_than_the_notes_is_asked_again' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_draft_with_a_different_task_count_than_the_notes_is_asked_again"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T02:39:14Z"
commit = "9c8c78abb"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 9c8c78abb (branch 1e85f7775; main 73630b8a8 merged in at d0c050bb2, no file in common). Post-merge verify in my worktree, detached at 9c8c78abb: the grep finds the test, and a_draft_with_a_different_task_count_than_the_notes_is_asked_again passes (1 of 1). The test runs phase 4.1's spec at Undertow 8643c98 (fixture tests/fixtures/p4_1_tasks, the notes' \"One task\"): its two-task draft (op 7cda5da7) is asked again once in the coverage re-ask, which names the stated count, and a one-task answer is kept; a second two-task answer is written with the warning line among the repairs. the_notes_state_the_task_count reads 1 from 4.1 and 5.2 and 7 from the phase-5 fixture, and nothing from \"not any one task's\". Branch suites: plan_generate and plan_policy 201/201; roko-cli lib 4060 passed, 0 failed; nightly fmt; clippy -p roko-cli -D warnings clean."
+++

## Problem

A spec's "Notes for the planner" in round v7 state how many tasks the plan has and list them: phase 4.1's "One task,
about 5 lines: **Bone only for the one number** (…)", phase 5.2's "One task: …", phase 5's "Eight tasks." Generation
never compares that count with the draft.

- Phase 4.1's plan from chat (op 7cda5da7, gpt-5.4) had 2 tasks, "T1 the colours, T2 the PNGs", where the notes
  give one. The pilot's plancheck found 7 findings from the split: the notes give all the files and steps to one
  task.
- The lead ran it as written. T2's only step passed on the base, and the rehearsal showed the PNGs landing in the base
  checkout.
- Nothing in roko said the count was off. The person had to notice.

## Why it matters

The notes' split is the spec author's design: which files a task owns, which checks it runs, what runs in
parallel. A draft that splits or merges against it changes all of that silently. A re-ask costs nothing extra when
the coverage re-ask runs anyway, and a stated warning lets the person see it before the run.

## Where

- `crates/roko-cli/src/plan_generate/sizes.rs`: reads the source's notes (`sized_items`); the count belongs beside it.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: the coverage re-ask (gap-dc5d7c) and how the better draft is kept.
- `crates/roko-cli/src/plan_generate/source_checks.rs::regeneration_prompt`.

## Plan

The lead's scope (2026-10-27): points 1, 2, 4 and 5. No `plan validate` warning: a person may split against the
notes on purpose.

1. Read the count the "Notes for the planner" section states, a number word up to twelve or digits directly before
   "task"/"tasks", in that section only. No section, no count, or two different counts means no check.
2. When the draft's task count differs, the coverage re-ask runs (its one regeneration, never an extra one) and
   names the stated count. The regeneration is kept when it keeps as many checks and fixes the count. If the counts
   still differ, the plan is written with a repair-style warning line in the result and plan.md, not refused.
3. Tests: phase 4.1's spec with its two-task draft is asked again and keeps a one-task regeneration; with a second
   two-task draft it is written with the warning. Phases 5 and 5.2 with their notes' drafts are not asked.

## Done when

The [[verify]] passes.
