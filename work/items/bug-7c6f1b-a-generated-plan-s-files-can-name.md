+++
id = "bug-7c6f1b"
kind = "bug"
title = "A generated plan's files can name a path that doesn't exist while the task's own read_files names the real file of that name: 2.1's plan lists core/src/envelope.h (and voice.h, filter.h, oscillator.h) for core/include/undertow/*.h, in files and in the task text, under a spec that says no new files"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#K-p2-1-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2.1 plan, op 742f8afd)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_validate.rs::missing_prerequisite_diagnostic"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_missing_files_path_takes_the_read_files_path_of_the_same_name' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_missing_files_path_takes_the_read_files_path_of_the_same_name"
+++

## Problem

Round v7, Undertow phase 2.1. The plan `p2-1-undertow-voice-fixes` was written from the Hermes chat with
`spec_path = docs/specs/phase-2.1-voice-fixes.md` (roko d7d1fbb8b, op `742f8afd`). Four of its five tasks list a
header that doesn't exist:

| Task | `files` names | The real header (on the base, and in the task's own `read_files`) |
|---|---|---|
| T1 | `core/src/envelope.h` | `core/include/undertow/envelope.h` |
| T2 | `core/src/voice.h` | `core/include/undertow/voice.h` |
| T4 | `core/src/filter.h` | `core/include/undertow/filter.h` |
| T5 | `core/src/oscillator.h` | `core/include/undertow/oscillator.h` |

- The spec's notes name the files by basename only: "**The envelope** (`envelope.h`, `envelope.cpp`)". The planner put
  the header next to the `.cpp`.
- The descriptions repeat the wrong path. T1's says "Update the envelope internals in core/src/envelope.h and
  core/src/envelope.cpp…". So the agent's own task text points at a file that isn't there.
- Each task's `[[task.context.read_files]]` names the real header, e.g. `core/include/undertow/envelope.h`, lines
  1-48. The plan contradicts itself, and roko could see that.
- The spec's scope says "No new files". A cheap model following the task text may create `core/src/envelope.h`, a
  stray header. One that edits the real header is logged as outside its task's `files` (`diff_scope = "record"`).
- Nothing flags it: `roko plan validate` gives 0 diagnostics. PLAN_031 checks prerequisites only, since `files` may
  name files the task creates.

## Where

- `crates/roko-cli/src/plan_generate/`: the repair phase, between the planner's draft and validation
  (`pipeline.rs`). `source_checks.rs` already rewrites verify steps there deterministically.
- `crates/roko-cli/src/plan_validate.rs`: PLAN_030/PLAN_031 (`missing_prerequisite_diagnostic`) are the nearest
  existing lints.

## Plan

1. **Repair, deterministic.** When a task's `files` path doesn't exist in the workspace, no task creates it, and the
   task's own `read_files` names an existing file with the same basename, replace the path with that file. Replace it
   in `files` and wherever the task's text (description, acceptance, goal, non_goals) quotes it. Report the change in
   the generation report, as the source-check repairs are reported.
2. **Finding.** When the source says no file may be added ("No new files"), a `files` entry that doesn't exist on the
   base and isn't repaired by step 1 is a plan finding: a new lint after PLAN_059, an error in generated plans.
   Without that sentence, a missing path stays allowed, since a task may create it.

## Done when

On a plan whose task lists `core/src/envelope.h` while its read_files names `core/include/undertow/envelope.h` (which
exists), generation writes `core/include/undertow/envelope.h` in `files` and in the description. With "No new files"
in the source, an unrepaired missing path is reported. Checked by the test in `[[verify]]`.
