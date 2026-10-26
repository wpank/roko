+++
id = "bug-ce1e4b"
kind = "bug"
title = "The file-path repair points a new file the source names at a same-named file its task reads: phase 5's T3 `tools/render/CMakeLists.txt` became the top-level `CMakeLists.txt`, in its files and its text"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "f9ba45559"
source = "round-v7-roko-planner#phase-5-replay"
discovered_from = "agent:roko-planner (round v7, phase 5 $0 replay, replay-p5/ideal.toml on roko 4d22b94e8)"
anchors = ["crates/roko-cli/src/plan_generate/file_paths.rs::repair_file_paths", "crates/roko-cli/src/plan_generate/pipeline.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7c6f1b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_new_file_the_source_names_keeps_its_path' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_new_file_the_source_names_keeps_its_path"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T14:31:51Z"
commit = "e90161ec4"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged e90161ec4 (branch 278e624d3, main merged in at 977ce9bdb). Post-merge verify in main at e90161ec4: the grep finds the test and a_new_file_the_source_names_keeps_its_path passes (phase 5's spec and the ideal draft as a fixture: written after one call, no file-path repair, T3's files tools/render/main.cpp, tools/render/CMakeLists.txt, CMakeLists.txt and its text as planned, 13 of 13 checks kept). Branch suites at 278e624d3: roko-cli lib 4002 passed, 0 failed, 7 ignored (share test skipped); plan_generate 173/173; nightly fmt; clippy -p roko-cli -D warnings clean. The repair now runs only under 'No new files' and never rewrites a path the source names; phase 2.1's case still repairs (bug-7c6f1b's test). $0 replay of phase 5's plan write on the branch's binary (tmp/hermes-roko-v7/roko-planner/replay-p5, tag ideal-ce1e4b): 1 call, 13 of 13 kept, no file-path repair, plan validate --strict 0."
+++

## Problem

Round v7's phase-5 spec (Undertow `spec/p5-f15`, cbf09a0) gives task 3, the offline renderer, three files: the new
`tools/render/main.cpp` and `tools/render/CMakeLists.txt`, and the top-level `CMakeLists.txt`, which it edits.

A $0 replay of phase 5's plan write (`tmp/hermes-roko-v7/roko-planner/replay-p5/`, a fake planner answering with a
draft that follows the spec's notes exactly, roko 4d22b94e8) wrote 13 of 13 checks kept and `plan validate --strict`
0 diagnostics, but changed task 3:

> T3: `tools/render/CMakeLists.txt` became `CMakeLists.txt` in its files and its text: `tools/render/CMakeLists.txt`
> does not exist, and `CMakeLists.txt` is the file of that name it reads

- T3's `files` lose the renderer's own CMake list.
- Its description now tells the agent to add the target to the top-level `CMakeLists.txt`.
- A try that creates `tools/render/CMakeLists.txt`, as the spec asks, writes outside its files.

The cause is bug-7c6f1b's `file_paths::repair_file_paths`. It points any `files` entry the workspace lacks at the one
existing file of that name its task reads (`read_files`), whatever the source says. Phase 2.1's case was a wrong path
under a spec that says "No new files". Without that sentence, the entry is as likely a new file as a wrong one.

## Why it matters

The repair corrupts a correct draft silently: the plan validates, and only the run shows it.

It rewrites:

- a new file the source names by its path, as here;
- any new file with a common name (`CMakeLists.txt`, `lib.rs`, `mod.rs`, `main.cpp`, `__init__.py`) whose task also
  reads another file of that name. An example is a new crate's `src/lib.rs` beside a read of an existing crate's.

A planner that reads the top-level list before editing it, as task 3 should, triggers it every time. Phase 5's plan
write waits on this fix.

## Where

- `crates/roko-cli/src/plan_generate/file_paths.rs`: `repair_file_paths`, the `read_files` branch.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: its call (`no_new_files` from `says_no_new_files`).

## Plan

1. Rewrite an entry the workspace lacks only when the source says its work adds no file. The workspace-name branch
   already has that condition.
2. Never rewrite a path the source names itself.
3. Test with phase 5's spec. A draft that follows its notes keeps `tools/render/CMakeLists.txt` and gets no file-path
   repair. Phase 2.1's case (No new files, `core/src/envelope.h` for `core/include/undertow/envelope.h`) still repairs.

## Done when

Phase 5's ideal draft, replayed, writes no file-path repair, and the bug-7c6f1b tests still pass.
