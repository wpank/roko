+++
id = "gap-9e212c"
kind = "gap"
title = "A generated plan ignores the task sizes a spec's notes state (phase 2: about 80 lines for T4), so every task comes out architectural and starts on the top rung"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "d7d1fbb8b"
source = "round-v7-hermes-pilot#E-p2-rumble-3"
discovered_from = "agent:hermes-pilot (round v7 phase 2, plan p2-undertow-rumble-3)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_generate/source_checks.rs::assign_runs"]
links = { depends_on = [], blocks = [], related = ["bug-72dc0d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_specs_stated_sizes_set_its_tasks_tiers' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_specs_stated_sizes_set_its_tasks_tiers"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T18:16:49Z"
commit = "3143d16db"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged d7d1fbb8b (work/plan-058-tests: 3143d16db), on top of roko-exact's bug-56a806. plan_generate/sizes.rs: a list item of the source (or of a document it names) that says \"about N lines\" and names files in code spans (and the short form `PluginProcessor.h/.cpp`) sizes the task that writes most of those files as a share of its files (a tie sizes none); a planner tier larger than the smallest tier whose line budget covers N is repaired down, max_loc clamped into that budget, and logged (\"T4: tier architectural became integrative, the smallest whose 150 lines cover the about 80 lines its source states\"); a smaller tier is left for the tier-size check, and a task the notes do not size keeps its tier. Verify at d7d1fbb8b: a_specs_stated_sizes_set_its_tasks_tiers passes (phase 2's spec at 3c84c86 and rumble-3's plan: sizes 200/160/80; T4 integrative with max_loc 150; T2, T3 architectural; T1, unsized, architectural; a smaller tier not raised), with a_short_form_names_two_files and the 83 other plan_generate::pipeline tests, roko-exact's included. Branch: roko-cli lib 3931 passed, 0 failed; clippy -D warnings clean. $0 replay of rumble-3 before (a pre-tiers binary): all four tasks architectural/300; the \"after\" replay runs on the lead's next binary."
+++

## Problem

Round v7, Undertow phase 2, plan `p2-undertow-rumble-3` (written 2026-10-20 17:34 on roko 9f0870e9a, planner
gpt-5.4-mini, from a Hermes chat). The spec's notes for the planner give the size of each task's work:

> 2. **The rumble** (`rumble.h`, `rumble.cpp`): P2.04 to P2.08. Needs task 1. About 200 lines, so architectural.
> 3. **The output stage** (`output_stage.h`, `output_stage.cpp`): P2.09 to P2.12, P2.20 and P2.21 … About 160 lines,
>    so architectural.
> 4. **Voice modulation and the engine wiring** (`voice.h`, `voice.cpp`, `engine.cpp`, `params.cpp` …): P2.13 to
>    P2.19, P2.22 and P2.23 … About 80 lines.

Task 1 (the LFO and the CMake glob) gives no size. It is small.

The plan made all four tasks `architectural`, with `max_loc = 300`. T4 (about 80 lines, within integrative's 150) and
T1 should be on a lower tier. In a chat run, an architectural task starts on the top rung, which falls back to
strong. So the tier decides which model runs the task, and what it costs.

## Why it matters

The person sized the work in the spec. roko's routing should follow that, and not the planner's habit of picking the
largest tier when unsure.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: the generation repairs.
- `crates/roko-cli/src/plan_generate/source_checks.rs`: the owner matching (`owner_of`, bug-72dc0d), which the notes'
  tasks could reuse: the files a note names, or the checks it lists.

## Plan

1. Lift each "About N lines" from the source's notes for the planner, with the files (`rumble.h`, `rumble.cpp`) or
   check ids (P2.04 to P2.08) the same note names.
2. Match each to the plan task that writes those files or owns those checks.
3. Set that task's tier to the smallest tier whose `max_loc` covers N, and its `max_loc` to N (rounded up to the
   tier's step). Log it as a repair. Decision 3203 forbids roko raising a tier on its own; here the spec's author sets
   it, so raising is fine too.
4. Leave a task whose note gives no size as the planner wrote it.

## Done when

Generated from phase 2's spec with rumble-3's draft, T2 and T3 stay architectural, T4 becomes integrative, and T1
keeps the planner's tier. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `~/dev/undertow/plans/p2-undertow-rumble-3/tasks.toml` and `docs/specs/phase-2-rumble.md` at 3c84c86
  ("Notes for the planner").
- Found by hermes-pilot's review of rumble-3; the lead asked roko-planner to file it.
