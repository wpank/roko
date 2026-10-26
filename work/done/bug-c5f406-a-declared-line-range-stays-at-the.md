+++
id = "bug-c5f406"
kind = "bug"
title = "A declared line range stays at the base file's length on a retry: fitted to 1-254 when the plan was written, it showed phase 4.3's T2#3 lines 1-247 of a kept tree's 290-line Displays.cpp, and the \"not shown\" note named only 248-254, not the 36 lines past the range"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_policy"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "70c408cd2"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 9, run 3752b8ff), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_policy.rs::fit_range", "crates/roko-cli/src/plan_policy.rs::render_declared_files"]
links = { depends_on = [], blocks = [], related = ["bug-7ff055", "gap-b95913", "bug-a8a787", "gap-56f761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_grown_file_shows_or_names_the_lines_past_its_planned_range' crates/roko-cli/src && cargo test -p roko-cli --lib a_grown_file_shows_or_names_the_lines_past_its_planned_range"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T20:58:19Z"
commit = "dd74a8ceb"
by = "roko-ladder (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T19:33:23Z"
forced = false
evidence = "dd74a8ceb, as merged with roko-exact's bug-fa22cc in af1855cb6: render_declared_files names the lines a file has grown past a declared range that starts at line 1 (as bug-fa22cc's fitted_lines fits it), less any anchor block's, merged with any cut's, so T2#3's 1-254 of a file grown to 290 lines and cut by the budget names the cut and the grown lines in one note. A range from line 1 past the file's end becomes the whole file when the plan is written and at dispatch: bug-fa22cc's fit_line_range, which the merge kept in place of dd74a8ceb's own rule, so an exact 1-N stays ranged and its note names what the file grows past. A part the planner chose stays as it is. Test a_grown_file_shows_or_names_the_lines_past_its_planned_range; red at 20:54 on the old code (the write-time fit cut 1-12 to 1-10, and no grown line was named). Chain on af1855cb6, with main b3c2b4746 merged in: 21 targeted tests pass (the four items', bug-fa22cc's a_range_past_the_end_fits_the_file, roko-planner's three generation tests), lib suites roko-cli 4049/0, roko-compose 528/0 and roko-learn 1409/0, the two crates' integration tests pass, clippy -D warnings and nightly fmt clean."
+++

## Problem

A task's declared line ranges are fitted to the files once, as they are when the plan is written. A retry runs on
the kept tree, where the files have grown, but its prompt still shows the write-time range. The "not shown" note
names only lines inside that range, so the lines past it are hidden without a word.

**Phase 4.3** (Undertow, run 3752b8ff, plan `p4-3-the-editor-s-finish`, T2):
- At write time the planner declared `plugin/ui/Displays.cpp` `1-260` and `plugin/ui/Status.cpp` `1-220`.
- roko's repair cut them to the base files' lengths, `1-254` and `1-120` (the plan's `plan.md` "Repairs").
- **T2#2's prompt** (`.roko/prompt-logs/p4-3-the-editor-s-finish-T2-1793025422990.txt`):
  - `Displays.cpp` shown 1–253 of 276 lines (`total="276"`), with a note for line 254 only;
  - `Status.cpp` shown 1–120 of 122, with no note.
- **T2#3's prompt** (`…-T2-1793025628531.txt`):
  - `Displays.cpp` shown 1–247 of 290, with a note for 248–254. Lines 255–290 weren't mentioned: 36 lines the
    earlier tries' edits had pushed past the planned range.
  - `Status.cpp` shown 1–120 of 123, with no note.
- The prompt's own intro says "A file cut short names the lines not shown: read those with read_file", so the
  agent is told the note is complete.

**Expected:** a retry's prompt shows a grown file up to its budget, or its note names every line it leaves out.

## Why it matters

- A retry starts from the tree roko kept (gap-56f761), which usually holds the last try's new code. That code is
  exactly what a retry's prompt can hide.
- No loss in phase 4.3: T2#2 re-read both files whole, and T2#3 read `Displays.cpp` from line 247 itself. An agent
  that trusts the prompt would miss them, and the re-read fix (gap-744bd0) asks agents to trust it.
- The same holds for a task whose files an earlier task grew. bug-7ff055's fit only cuts a range that runs past
  the end of a file; it never extends one.
- Goal: golden-path.

## Where

- `crates/roko-cli/src/plan_policy.rs::fit_range`: cuts `1-260` on a 254-line file to `1-254` rather than leaving
  it whole. It's used by `fit_ranges_to_files` (gap-b95913, at write time) and `fit_ranges_to_earlier_changes`
  (bug-7ff055, at dispatch).
- `crates/roko-cli/src/plan_policy.rs::render_declared_files`: shows a ranged read's range. `hidden`, which feeds
  `not_shown_line`, gets only an unranged file's lines past the policy limit, never lines past a declared range.
- **Entry point:** `crates/roko-cli/src/dispatch/prompt_builder.rs::assemble` → `render_declared_files` for each
  try's `file_context` layer.

## Current state

- The fit was built for files an earlier task changes (bug-7ff055) and for the planner's guessed lengths
  (gap-b95913). Neither knows that a task's own earlier tries changed its files.
- Done items to read first: bug-7ff055, gap-b95913, bug-a8a787 (the declared-files section).

## Plan

1. In `fit_range`, a range that starts at line 1 and runs to or past the file's end becomes unranged, i.e. the whole
   file, as a range that starts past the end already does. The prompt then shows the file as it is at dispatch,
   within the budget, and an unranged file's cut already gets a note.
2. In `render_declared_files`, when a ranged read's file now has more lines than the range covers, add those
   lines to `hidden`. The note then names them: `[plugin/ui/Displays.cpp lines 255-290 not shown: read_file them]`.
   Step 1 covers new plans; this step covers plans already written with exact ranges.
3. Add the test `a_grown_file_shows_or_names_the_lines_past_its_planned_range`. A file declared `1-10` that grows
   to 15 lines renders its new lines, or a note naming 11–15.

## Done when

- The `[[verify]]` passes.
- A retry prompt for a grown file shows the new lines or names them. Check the next run's prompt logs with
  `tmp/hermes-roko-v7/run-analyst/prompt_logs.py` (`data/prompt_files.csv`: `lines_shown`, `total_lines`,
  `not_shown_note`).

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 9, "A retry's prompt can hide the lines earlier
  tries added".
- Keep a partial range that the planner chose on purpose (say `40-80` of a 300-line file) as it is. Only a range
  that covered the whole file at write time should follow the file.
- **A second case: phase 5, run 1f677933 (2026-10-26).** T8#2 was the hand-back try after the plan check failed. It
  ran on T8#1's accepted tree, where `plugin/ui/Displays.cpp` had grown from 301 to 304 lines and
  `plugin/ui/Displays.h` from 53 to 54.
  - Its prompt showed `Displays.cpp` 1–90 of 304, with a note for "lines 91-301" only. Lines 302–304 weren't
    mentioned.
  - It showed `Displays.h` 1–53 of 54, with no note.
  - The prompt log is `.roko/prompt-logs/p5-ship-T8-1793030314833.txt`, and the rows are in
    `tmp/hermes-roko-v7/run-analyst/data/prompt_files_1f677933.csv`.
