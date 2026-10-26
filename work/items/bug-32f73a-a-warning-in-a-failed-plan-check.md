+++
id = "bug-32f73a"
kind = "bug"
title = "A warning in a failed plan check's output implicates a task: phase 5's rehearsal handed spec_p5_demos and spec_p5_install's failure to T6 because a clang warning named core/src/filter.cpp, not to T4 and T5, whose render-demos.sh and install.sh the failure names"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 lead (phase 5 rehearsal 20261026-162312 and run 1f677933)"
discovered_from = "agent:team-lead (round v7, phase 5)"
anchors = ["crates/roko-cli/src/graph_execution/plan_handback.rs"]
links = { depends_on = [], blocks = [], related = ["bug-fc05a3", "bug-3b3bb6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_warning_never_implicates_a_task' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_warning_never_implicates_a_task"
+++

## Problem

When a plan's whole-plan check fails, roko hands the failure back to the tasks that own the files its output names
(bug-fc05a3, find-5bcba7). It looks for those names anywhere in the output, compiler warnings included.

Phase 5's $0 rehearsal (`tmp/hermes-roko-v7/roko-ladder/rehearsals/p5/runs/20261026-162312/report.md`, on
roko-main-74c3594bc) shows the effect. `tools/check.sh --plugin` failed on two tests:
- `spec_p5_demos`: "render-demos.sh renders every preset into build/demos/".
- `spec_p5_install`: "install.sh --dest … must put this bundle here".

The only owned file the output named was in a clang warning written after the test output:
"filter.cpp:14:17: warning: unused variable 'kButterK1'". So roko reported "failed, naming core/src/filter.cpp;
handing it back to task T6". The failure itself pointed at T4's `tools/render-demos.sh` and T5's `tools/install.sh`.

This is F1 of bug-fc05a3's findings coming back. `named_files` scans the whole output, and a bare script name
(`render-demos.sh`) never matches an owned path (`tools/render-demos.sh`).

## Plan

- Find the named files only in the failure's own lines: the failure lines printed before the first case, and each
  failing case's header, failures and the lines under them (`roko_learn::failure_text`).
- Never search a warning, a build progress line or a runner's summary.
- Let a bare file name on those lines (`render-demos.sh`) name the one owned file with that base name.

## Design

roko-exact, 2026-10-26, built with bug-5e361a and bug-8b9468 (one change to `plan_handback`).

- **The failure's own lines** (`roko_learn::failure_text::failure_lines`): the failure lines outside any case, then
  each failing case's header and its failures with the lines under them (`values:`, `logged:`). Without failing
  cases, each failure line with the lines under it. A warning, build progress or a runner's summary is never one.
- **CTest's line closes a case.** `parse` reads `23/28 Test #23: spec_p5_demos ***Failed` as a line outside any case
  that closes the case before it, since the binary that printed that case is done. It used to count as one more
  failure of the previous binary's last case. `failure_excerpt` keeps up to 3 such lines, as before.
- **`implicate`** reads the named files from those lines only, and returns what the failure points at:
  `Implicated { files, runners, tasks }` (runners: bug-8b9468).
- **A bare file name** names the one owned file with that base name: the name has a `.`, no other owned file has
  it, and no path character is next to it. `install.sh --dest` names `tools/install.sh`;
  `build/demos/x.wav` names no `x.wav`.
- **The hand-back event** names the runners too: "failed in a test that task(s) T4, T5 run, naming
  tools/install.sh, tools/render-demos.sh; handing it back to task T5 …".
- **On the rehearsal's output** (fixture `crates/roko-cli/tests/fixtures/p5_rehearsal_check.txt`): the files are
  `tools/render-demos.sh` (T4) and `tools/install.sh` (T5). T4 and T5 run the failing tests, and T5 leads: both
  are focused, and T5 is last in plan order. `filter.cpp` names nothing.

## Done when

The test in `[[verify]]` passes. Given the rehearsal's output, a warning naming `core/src/filter.cpp` names no file,
and the failure's `render-demos.sh` and `install.sh` name T4's and T5's scripts.
