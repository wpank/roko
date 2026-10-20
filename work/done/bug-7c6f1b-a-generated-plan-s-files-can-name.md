+++
id = "bug-7c6f1b"
kind = "bug"
title = "A generated plan's files can name a path that doesn't exist while the task's own read_files names the real file of that name: 2.1's plan lists core/src/envelope.h (and voice.h, filter.h, oscillator.h) for core/include/undertow/*.h, in files and in the task text, under a spec that says no new files"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "a12d2a60f"
source = "round-v7-hermes-pilot#K-p2-1-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2.1 plan, op 742f8afd)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_validate.rs::missing_prerequisite_diagnostic"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_missing_files_path_takes_the_read_files_path_of_the_same_name' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_missing_files_path_takes_the_read_files_path_of_the_same_name"

[closed]
at = 2026-10-21
at_ts = "2026-10-20T23:26:28Z"
commit = "825836174"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged be7c29f0b (work/bug-7c6f1b: 825836174). plan_generate/file_paths.rs, run right after the draft's first repairs: a files entry the workspace lacks, listed by no other task, takes the existing same-named file its task reads (read_files; two such reads are no answer), in files and in the task's title, description, acceptance, goal, non-goals, assumptions and open questions (a longer name left alone), logged as a repair. Only under 'No new files' (says_no_new_files) does an entry without such a read take the workspace's one file of that name, and any entry still missing is a refusal sent back to the planner (the lead approved the design). Post-merge verify at be7c29f0b: grep OK; a_missing_files_path_takes_the_read_files_path_of_the_same_name passed (with a_repaired_regression_check_step_passes_on_base). Branch: roko-cli lib 3944 passed, 1 failed (bug-3203c4's delivery-cancel flake, which passed alone with the 164 plan_generate tests), clippy -D warnings clean, nightly fmt. $0 replay of 2.1's draft (the written plan without roko's SC9 step; spec at 53bcee0): before (roko-main-d7d1fbb8b) the real 9206-byte write with T1/T2/T4/T5 on core/src/<name>.h; after: 1 call, 10/10 kept, files and text on core/include/undertow/<name>.h, validate 0 diagnostics; a draft listing core/src/envelope_state.h: asked again twice, then refused with 'T1 lists core/src/envelope_state.h in its files, which does not exist, and the source says no new files'."
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
