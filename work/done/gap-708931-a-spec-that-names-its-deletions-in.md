+++
id = "gap-708931"
kind = "gap"
title = "A spec that names its deletions in words (\"the twelve retired files deleted\") or its files in shorthand (PluginEditor.h/.cpp) gets no repair and no warning: bug-aa93e9 reads only full paths in sentences of change, so phase 1's first spec passed plan validate --strict with every deletion missing"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "5bdac29ab"
source = "round v8: the lead's and undertow-spec-v8's narrower diagnosis of bug-aa93e9 (Undertow 19943c9's phase 1 spec)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/named_files.rs::named_files", "crates/roko-cli/src/plan_generate/named_files.rs::unlisted_named_files_line", "crates/roko-cli/src/plan_validate.rs::validate_plans_dir_impl"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-aa93e9", "bug-ce1e4b", "bug-7c6f1b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_spec_that_deletes_files_in_words_is_warned_about' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_spec_that_deletes_files_in_words_is_warned_about"

[[verify]]
command = "grep -rqw 'fn a_shorthand_pair_stays_in_one_task' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_shorthand_pair_stays_in_one_task"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T18:10:14Z"
commit = "c1f506b8f"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "M"
forced = false
evidence = "A writing task that owns one file of a shorthand pair (X.h/.cpp, by path or bare name) gains the other at generation, beside its file or the workspace's one of that name, and plan validate names a missing partner in PLAN_062; PLAN_063 warns when the sources say files are deleted in words ('retired ... files' within a clause, or a Delete/Remove-led sentence naming source files only bare) and name no deletion by path. Red first (919a202b9 unwired): no PLAN_063 (0, want 1). Green at c1f506b8f on the trial 99ebe01bd (main 395d58380 + bug-1bbe2b + this branch): a_spec_that_deletes_files_in_words_is_warned_about (the 19943c9 spec: PLAN_063 quotes 'and the retired files removed' and 5 more, PLAN_062 names plugin/PluginEditor.h; e7131fb: no PLAN_063) and a_shorthand_pair_stays_in_one_task pass; roko-cli lib 4145 passed 0 failed, plan_validate 27/27, plan_validation 40/40, clippy and fmt clean. $0 scans with the built binary: of Undertow v1's 16 specs and phase 1's three, PLAN_063 fires only on 19943c9; of 289 roko work items, on 2, both real (gap-708931 itself, gap-c332f6's 'Delete `holdout.rs` ...'), after two tightenings removed the 4 false hits of runtime prose."
+++

## Problem

bug-aa93e9 (merged ba5866664) puts in a task's `files` each file a spec names, by its full path, in a sentence that
tells a task to delete or change it ("Deletes: `core/src/lfo.cpp`, …", "Edits: …"). Phase 1's first spec (Undertow
19943c9) named most of its changes another way, and roko still reads nothing there.

- **In words.**
  - "and the twelve retired files deleted"
  - "the retired `plugin/ui` files deleted"
  - "delete the retired files and finish the engine"
  - "2. **The retired core files** (delete them)", whose sub-item lists `core/include/undertow/oscillator.h`,
    `filter.h`, … and "their `core/src/*.cpp`" with no verb.
- **In shorthand.**
  - `plugin/PluginEditor.h/.cpp` (both files) in the "What to build" heading.
  - `PluginProcessor.h/.cpp` and `engine.h/.cpp` (bare names) in the planner's notes.
  - "Delete the rest: `Displays.h/.cpp`, `Look.h/.cpp`, …" (bare names under `plugin/ui/`).

The planner kept the paths the notes spelled out and dropped the rest:
- every deletion;
- `plugin/PluginEditor.h`, though the task listed `plugin/PluginEditor.cpp`;
- `plugin/ui/Theme.h`.

`plan validate --strict` passed that plan with 0 diagnostics, so nothing told the person that the spec's deletions
were nowhere in it.

## Why it matters

- A spec author writes the old way first. undertow-spec-v8 wrote phase 1 like this, and only the pilot's reading
  found the gap, after a paid plan write.
- The lead's ask for bug-aa93e9's (d): the warning fires for the vague form.
- Goal: core (plan writing).

## Where

- `crates/roko-cli/src/plan_generate/named_files.rs` (bug-aa93e9): `named_files`, `instruction`,
  `unlisted_named_files_line`.
- `crates/roko-cli/src/plan_validate.rs::validate_plans_dir_impl`: PLAN_062.
- `crates/roko-cli/src/plan_generate/sizes.rs::code_span_files`, which already reads `X.h/.cpp` as two files.

## Plan

1. **Shorthand pairs stay together** (a repair at generation, and part of PLAN_062 in validate). Take a pair the
   sources write as `X.h/.cpp`, by path or by bare name, where a writing task lists one file of it and no task lists
   the other. That task gets the other too: the file beside the one it lists, else the workspace's one file of that
   name. Phase 1's T2, which listed `plugin/PluginEditor.cpp`, gets `plugin/PluginEditor.h`.
2. **A deletion in words is warned about** (PLAN_063, at validate, and a note at generation). This fires when the
   sources say files are deleted, retired or removed (the words "file" or "files" with one of those verbs, or a
   deletion sentence whose spans name files the workspace has under another path) but name none of them as a
   deletion by full path. The warning quotes the sentences and asks for each path. It stays quiet when the sources
   name their deletions explicitly too: the fixed spec (e7131fb) keeps its "twelve retired files deleted (listed in
   item 2)" without a warning.
3. **Tests, on phase 1's real spec shapes.**
   - `a_spec_that_deletes_files_in_words_is_warned_about`: the 19943c9 spec and its plan as written, from fixtures.
     PLAN_063 quotes "the twelve retired files deleted" and "the retired `plugin/ui` files deleted". The e7131fb
     spec gets no PLAN_063.
   - `a_shorthand_pair_stays_in_one_task`: T2 lists `plugin/PluginEditor.cpp` and gains `plugin/PluginEditor.h`.
     `engine.h/.cpp`, whose two files sit in different directories, resolves by name.

## Done when

- Both `[[verify]]` commands pass.
- `plan validate` on the 19943c9 spec's plan as written gives PLAN_063, and gives nothing new on Undertow v1's 22 plans
  or roko's plans.

## Notes

- Found by the lead and undertow-spec-v8 (the narrower diagnosis of bug-aa93e9, 2026-11-12).
- Related: bug-aa93e9 (named files), bug-ce1e4b and bug-7c6f1b (paths at generation).
