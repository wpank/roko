+++
id = "gap-29c1e8"
kind = "gap"
title = "A spec that gives a task a tier its own stated size is over (\"Focused: about 90 lines\") gets no warning: phase 5's presets task started on a dearer rung than its spec named, and phase 6's spec sizes five tasks over their tiers"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "e5030bae2"
source = "round v8 hermes-pilot-v8: phase 5 plan write op 97a74018 (roko e722f0e1f, Undertow docs/specs/phase-5-sound.md); the lead chose option (b)"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/sizes.rs::sized_items", "crates/roko-cli/src/plan_validate.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-9e212c", "gap-4dd77d", "gap-708931"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_tier_word_too_small_for_its_size_is_warned_about' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_tier_word_too_small_for_its_size"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T21:16:38Z"
commit = "e5030bae2"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "S"
forced = false
evidence = "plan validate warns once per plan (PLAN_064) when a source's notes for the planner give a task a tier whose line budget its stated size is over, read one list item at a time in both word orders (\"Focused: about 90 lines\", \"About 200 lines, so architectural\"), naming each task as the notes do and the tier its size needs. Red first (15de12d90 on main 8c4eed182): a_tier_word_too_small_for_its_size_is_found and _is_warned_about failed, nothing found. Green at e5030bae2 (main 8c4eed182 + this branch): 38 sizes, named_files and plan_validate tests pass, roko-cli lib 4147 passed 0 failed, plan_validate 27/27, plan_validation 40/40, cargo clippy --workspace --no-deps -D warnings and nightly fmt --check clean. $0 probe scan with the built binary: PLAN_064 fires on phase 5's T3 (focused, 90 lines, needs integrative), phase 6's five sized tasks and phase 1's T1 (architectural, 350 lines, over every tier) in all four of phase 1's versions; quiet on phases 2-4, Undertow v1's 16 specs and all 1713 roko work items."
+++

## Problem

A spec can give a task a tier whose line budget its own stated size is over, and nothing says so before the run.
roko's tiers are line budgets: mechanical covers at most 20 lines, focused 50, integrative 150 and architectural 300
(`TaskTier::max_loc`).

- Round v8's phase 5 spec (Undertow `docs/specs/phase-5-sound.md`) says "3. **The presets.** … Focused: about 90
  lines." in its notes for the planner.
  - The plan as written (op 97a74018, roko e722f0e1f) gave T3 `max_loc = 90` and the smallest tier that allows it,
    integrative. Had it written focused, `raise_tiers` (gap-4dd77d) would have raised it.
  - So T3 started on the mid rung (gpt-5.4-mini, $0.062) and not the cheap one the spec named. The planner followed
    roko's budgets; the spec's tier word and size disagree.
- Phase 6's spec (Undertow 8945b3c) sizes all five of the tasks it sizes over the tiers it names:
  - "Integrative: about 170 lines", "about 420" and "about 250": integrative covers 150.
  - "Architectural: about 450 lines" and "about 420": architectural covers 300, the most any tier covers.
- Phase 1's spec does the same once: "Architectural: about 350 lines written". Its plan capped `max_loc` at 300, and
  the task passed, because `max_loc` reaches the agent only as "Max lines of change" in its brief.
- `plan validate` says nothing about any of these.

## Why it matters

- A task's tier picks the rung it starts on. A spec that names a tier too small for the task's size gets a dearer
  rung than it intended, or a brief that understates the work. The author learns why only from the run's cost.
- The mismatch sits in the spec, so a $0 lint over the plan's `[meta] sources` catches it before the run.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/sizes.rs`: `planner_notes`, `sized_items` (gap-9e212c, the sizes a source
  states).
- `crates/roko-cli/src/plan_validate.rs`: the plan-level warnings that read `[meta] sources` (PLAN_062, PLAN_063).

## Plan

1. `tiers_under_sizes(sources)` reads each source's notes for the planner, one list item at a time. A tier word is
   one of roko's four tier labels, either before its size ("Focused: about 90 lines") or after it ("About 200 lines,
   so architectural", round v7's order). Each tier word whose budget the size is over gives the task as the item
   names it (`task 3 (The presets)`), its tier, its size, and the smallest tier whose budget covers the size, if any
   does. A tier word whose budget covers its size, as in "Integrative: about 50 lines", is no mismatch.
2. `plan validate` warns once per plan (PLAN_064). The warning names each such task, its tier and size, and the tier
   its size needs, or says that no tier covers it. It also says what each tier covers, and to give the task the tier
   its size needs or split it.
3. The two tests below run on the notes for the planner of phases 1–6, copied verbatim from Undertow 8945b3c:
   - Test `a_tier_word_too_small_for_its_size_is_found` asserts that phase 5's T3 is found, that all five of phase
     6's sized tasks and phase 1's T1 are found, and that phases 2–4 and the v7 fixture (`p2_sizes`) are quiet.
   - Test `a_tier_word_too_small_for_its_size_is_warned_about` asserts that `plan validate` gives a plan written from
     phase 5's spec one PLAN_064 naming T3, and gives none for phase 4's.
4. Red first: the tests on stubs that find nothing.

## Done when

- The `[[verify]]` command passes.
- Over Undertow's specs and roko's work items, a $0 probe scan with the built binary shows PLAN_064 only where a tier
  word and its size disagree.

## Notes

- Found by hermes-pilot-v8 (evidence: `tmp/hermes-roko-v8/pilot/evidence/p5-*`). The lead chose this item, option (b),
  and declined option (a): reading the nested "Edits:" layout so that a stated size sets the tier again. Whether a
  stated size should beat a stated tier word is a design question.
- Related: gap-9e212c (stated sizes), gap-4dd77d (`raise_tiers`), gap-708931 (PLAN_063, the same kind of warning).
