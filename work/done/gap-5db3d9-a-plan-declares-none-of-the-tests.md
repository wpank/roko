+++
id = "gap-5db3d9"
kind = "gap"
title = "A plan declares none of the tests its checks run (1 of 73 across phases 4.3 to 5.2) nor the headers beside the sources it declares, so a try finds them itself: 5.2's only try read its three spec tests, presets.h, params.h and params.cpp before its first edit in call 7"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "2b171f0c6"
source = "round-v7 roko-ladder, gap-744bd0's measurement on phase 5.2 (run 13260e60) and 4.3/5/5.1, the lead's ask"
discovered_from = "agent:roko-ladder (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/source_reads.rs::repair_source_reads", "crates/roko-cli/src/plan_generate/pipeline.rs::repair_generated_draft", "crates/roko-cli/src/dispatch/prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = ["gap-744bd0", "bug-417c4a", "bug-60ad88"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_reads_the_tests_its_new_checks_run' crates/roko-cli/src && cargo test -p roko-cli --lib a_task_reads_the_tests_its_new_checks_run"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T05:50:04Z"
commit = "2b171f0c6"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-27T01:41:41Z"
forced = false
evidence = "Merged 2b171f0c6 (branch 38aa65c24, with the bug-3d0414 shared-fragment test fcd55af51; main df347694b merged in at 7dc6fa498, no file in common). Post-merge verify in my worktree, detached at 2b171f0c6: the grep finds the test, and a_task_reads_the_tests_its_new_checks_run passes (1 of 1). The test runs phase 5.2's as-written draft (fixture tests/fixtures/p5_2_criteria): T1 gains core/include/undertow/presets.h and tests/spec/p5_2/presets_test.cpp, and not the p5 and p5_1 regression tests a source names; neither read counts toward a one-file budget; a second pass adds nothing. Branch suites: plan_generate and plan_policy 208/208, with the lead's three negative tests (a check that names no test adds no read; a Rust source gets no header read; two candidates that fit alike give no read and a line naming both) and one for a header the task owns; roko-cli lib 4074 passed, 0 failed; nightly fmt; clippy -p roko-cli -D warnings clean. $0 replays on the final binary of 5.2 (as written and ideal), 5 (8 tasks), 4.3 and 4.2: every check kept (9, 9, 21, 17, 19), plan validate --strict 0 diagnostics each, 24 reads added to the written plans, none wrong. The item's run measure (m744/undeclared.py on the next phases' runs) waits for a run; none yet."
+++

## Problem

A plan's tasks don't declare the tests their checks run, or the headers beside the sources they declare. A try
finds them itself in its first calls.

Across phases 4.3, 5, 5.1 and 5.2 (23 implementer tries), each try's checks ran 73 distinct test files in all. The
plans declared 1 of them: 4.3's T1 declared `tests/spec/p4_3/panels_test.cpp`. The specs name every one, as in
phase 5.2's "The spec test: `tests/spec/p5_2/presets_test.cpp`", and the plan is written from the spec.

**Phase 5.2** (Undertow run 13260e60, roko main 00ecbd158): the only try, T1#1, re-read nothing its prompt showed
(gap-744bd0). Before its first edit, in call 7, it read:
- the three spec tests its checks run: `tests/spec/p5_2/presets_test.cpp`, `p5/presets_test.cpp` and
  `p5_1/render_test.cpp`;
- `core/include/undertow/presets.h`, the header of its declared `core/src/presets.cpp`;
- `params.h` and `params.cpp`.
It also ran 6 greps (`P5.2.` in tests, `SeqRoot` in the headers) and one bash call that did nothing. The prompt
held the spec and presets.cpp; their 2 files were all the plan declared.

**Earlier phases**, reads before the first edit, by kind (`tmp/hermes-roko-v7/roko-ladder/m744/undeclared.py`):

| Phase (run) | Tries | Reads | Re-read of shown lines | Lines a note named | Test a check runs | Other test | A file the task changes | Header of a declared file | Other code |
|---|---|---|---|---|---|---|---|---|---|
| 4.3 (3752b8ff) | 4 | 18 | 9 | 1 | 0 | 0 | 6 | 2 | 0 |
| 5 (1f677933) | 13 | 46 | 11 | 2 | 1 | 1 | 11 | 5 | 15 |
| 5.1 (814a7340) | 5 | 19 | 11 | 3 | 1 | 1 | 3 | 0 | 0 |
| 5.2 (13260e60) | 1 | 7 | 0 | 1 | 3 | 0 | 0 | 1 | 2 |

The same reads over whole tries:

| Phase | Reads | Re-read | Note lines | Check tests | Other tests | Task's files | Headers | Other code |
|---|---|---|---|---|---|---|---|---|
| 4.3 | 27 | 15 | 1 | 1 | 0 | 6 | 3 | 1 |
| 5 | 62 | 11 | 3 | 4 | 2 | 17 | 5 | 20 |
| 5.1 | 35 | 18 | 3 | 4 | 2 | 4 | 1 | 3 |
| 5.2 | 7 | 0 | 1 | 3 | 0 | 0 | 1 | 2 |

How to read them:
- **Before gap-744bd0, re-reads dominated.** In 4.3–5.1 the first calls went mostly on re-reading what the prompt
  held: 31 of 83 reads.
- **The task's own undeclared files:** 20 reads in 4.3–5.1. Those plans were written before bug-417c4a (merged
  17:38), which now lets a generated task read every file it owns; 5.2 had none.
- **Check tests came late.** Earlier tries mostly read their checks' tests after a check had failed: 2 before the
  first edit, 10 over whole tries, in 4.3–5.1. 5.2's try read all three before editing.
- **Headers:** headers of declared files account for 8 reads before the first edit.
- **Greps:** 47 greps ran before first edits across the 23 tries; 32 of them were in phase 5.

## Why it matters

- **Calls.** A try that must find its checks' tests and its sources' headers spends calls on it: 5.2's first
  edit came in call 7, against medians of 4–5 before. Each call re-sends the growing context.
- **Retries.** A check's test is what defines "done". Tries that first meet it as a failed check's output spend
  a retry learning what a read up front would have told them.
- **gap-744bd0's rule.** The rule now has agents work from the prompt. What the prompt doesn't hold is what costs
  turns.
- Goal: golden-path.

## Where

- `crates/roko-cli/src/plan_generate/source_reads.rs::repair_source_reads` makes every task read the plan's spec.
  That is the pattern a test read could follow, counted outside the read budget like the sources.
- `crates/roko-cli/src/plan_generate/pipeline.rs::repair_generated_draft` holds the draft repairs.
- `crates/roko-cli/src/plan_generate/symbol_reads.rs` adds the one file that holds a named symbol (bug-417c4a).
- `crates/roko-cli/src/dispatch/prompt_builder.rs`: the `# Verify` section, where a test file could be named
  without its content.

## Current state

- The planner may declare tests; 4.3's T1 did. Nothing asks it to, and the repair pass adds none.
- Specs name their test files in prose and in the check table's surroundings: "The spec test: …", "… (task 1) and
  … (task 2)".

## Plan

Not coded; the lead routes it (plan generation is roko-planner-2's area). Options:

- **A. Declare the tests the task must turn green (recommended).**
  - For each task, add a read of the test file that each `fail_on_base` check runs, taken from the spec's own
    words. The test's directory and stem must match the check's runner arguments, e.g. `tools/spec.sh 5_2 presets`
    with `tests/spec/p5_2/presets_test.cpp`.
  - Add a ranged read of the header beside each declared source the task changes (same stem, `.h`).
  - Count both outside the read budget, as the sources are, and let the declared-files budget cut them with a
    note when they don't fit.
  - Regression checks' tests stay undeclared: they pass on the base.
- **B. Name, don't show.** The prompt's `# Verify` section names the test file each check runs ("`tools/spec.sh
  5_2 presets` runs `tests/spec/p5_2/presets_test.cpp`"), with no content.
  - This saves the search but not the read, at a few dozen bytes per check.
  - Use it for the regression checks, or alone if A's prompt cost is too high.
- **Test:** `a_task_reads_the_tests_its_new_checks_run`. Phase 5.2's spec and draft give T1 a read of
  `tests/spec/p5_2/presets_test.cpp` and of `core/include/undertow/presets.h`, and none for the regression checks.

## Done when

- The `[[verify]]` passes.
- In the next phases' runs, `m744/undeclared.py` shows check tests read from the prompt, not found by search,
  and fewer calls before the first edit.

## Notes

- 5.2 is one try. The earlier phases are 22 tries written before gap-744bd0 and bug-417c4a, so the categories
  shift between them.
- The method is `tmp/hermes-roko-v7/roko-ladder/m744/` (`measure.py`, `undeclared.py`, `reads_before_edit.csv`).
  A read is a `read_file` call. Kinds come from the prompt log's declared blocks and notes, the task's verify
  commands (`tools/spec.sh <phase> <name>` runs `tests/spec/p<phase>/<name>_test.cpp`), its `files`, and the
  declared blocks' `#include` lines.
