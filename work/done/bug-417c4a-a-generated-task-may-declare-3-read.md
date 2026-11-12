+++
id = "bug-417c4a"
kind = "bug"
title = "A generated task may declare 3 read files with its spec counted, and an anchored symbol in a file it doesn't read sends the whole draft back: phase 4.2's write paid two gpt-5.4 regenerations ($0.29) for Look::drawStaticLayer and UndertowEditor::initialiseControls, both in the repo"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "23d12be5f"
source = "round-v7-team-lead#plan-context-symbol-repair"
discovered_from = "agent:hermes-pilot and run-analyst (round v7, phase 4.2 plan write op 1b0a97ed), the lead's ask"
anchors = ["crates/roko-cli/src/plan_policy.rs::PlanExecutionPolicy::generated", "crates/roko-cli/src/plan_generate.rs::render_generated_plan_limits", "crates/roko-cli/src/plan_generate/pipeline.rs::repair_generated_draft"]
links = { depends_on = [], blocks = [], related = ["bug-60ad88", "gap-4dd77d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_symbol_in_a_file_the_task_does_not_read_is_repaired_not_regenerated' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_symbol_in_a_file_the_task_does_not_read_is_repaired_not_regenerated"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:24:13Z"
commit = "4f2c57f0e"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 4f2c57f0e (branch d21a41d2d; main merged in at 85f7822b9). Post-merge verify in main at 4f2c57f0e: the grep finds the test and a_symbol_in_a_file_the_task_does_not_read_is_repaired_not_regenerated passes (fixtures tests/fixtures/p4_2_symbols: phase 4.2's spec at Undertow 4db168c and its two rejected drafts; each written after one call with the file that holds the symbol among T1's reads, 19 of 19 checks kept). Branch suites at d21a41d2d: roko-cli lib 4011 passed, 0 failed, 7 ignored; plan_generate and plan_policy 189/189; nightly fmt; clippy -p roko-cli -D warnings clean. $0 replay on Undertow 4db168c with a fake planner answering drafts 1, 2 and the as-written plan: before (bug-ce1e4b's binary) 3 planner calls, after 1 call, 19 of 19. A generated task may now read max(3, the files it owns) besides the plan's sources (prompt, PLAN_BUDGET_READ_FILES and the truncation agree); FAST keeps 3."
+++

## Problem

Round v7's phase 4.2 plan write (op 1b0a97ed, serve log 10:38:56–10:40:20Z on 2026-10-26, gpt-5.4) cost $0.473
against its $0.50 cap for a 3-task plan: one draft and two full regenerations. Validation rejected two drafts on
PLAN_CONTEXT_SYMBOL (Undertow `.roko/plan-drafts/p4-2-the-editor-laid-out-as-designed/rejected-{1,2}.md`):

- draft 1: "[T1]: explicit symbol anchor `Look::drawStaticLayer` was not found in any declared read_files entry";
- draft 2: "`UndertowEditor::initialiseControls` was not found …".

Both symbols exist at the base 4db168c (`plugin/ui/Look.cpp:100`, `plugin/PluginEditor.cpp:47`). Draft 3 passed only
by writing them as prose. `max_retries = 2` was the real limit: a third rejection would have failed the write.

The root cause (run-analyst, findings §7) is the read budget. A generated plan gets the FAST lane's read budget, 3
files per task (`PlanExecutionPolicy::generated`, `FAST_MAX_READ_FILES`), and the planner prompt states it as "at
most 3 entries per task". The spec-first planner note takes one slot. Phase 4.2's T1 owned 6 files in 3 components:

- draft 1 declared `PluginEditor.cpp` and anchored `Look::drawStaticLayer`;
- draft 2 swapped in `Look.cpp` and anchored `UndertowEditor::initialiseControls`.

Validation exempts the plan's sources from the count (bug-60ad88), but the prompt doesn't say so. And
`repair_generated_draft` drops every read past the 3rd before the sources repair runs, the spec counted.

Over the round, PLAN_CONTEXT_SYMBOL threw away 3 drafts for $0.372, $0.29 of it in phase 4.2.

## Why it matters

- Every rejection costs a whole planner call, and the planner re-reads the repository with its tools each time.
- A third rejection fails the write outright.
- Phase 5's write meets the same rule.

## Where

- `crates/roko-cli/src/plan_policy.rs`: `PlanExecutionPolicy::generated` (`max_read_files_per_task`), the
  PLAN_BUDGET_READ_FILES and PLAN_CONTEXT_SYMBOL checks.
- `crates/roko-cli/src/plan_generate.rs`: `render_generated_plan_limits`, the limit the prompt states.
- `crates/roko-cli/src/plan_generate/pipeline.rs`: `repair_generated_draft`'s read truncation, and the repairs
  before validation.

## Plan

1. A generated task may read max(3, the number of files it owns) files, not counting the plan's sources. The same
   rule applies in the prompt's stated limit, PLAN_BUDGET_READ_FILES and `repair_generated_draft`'s truncation. The
   FAST lane keeps 3.
2. Before validation, with no model call: for each `symbols` anchor that no declared read holds, look for it with
   the validator's own matcher, first in the task's own existing files, then in the workspace.
   - One file holds it: add that file to `read_files`, whole when it fits the range limit, else a range around the
     anchor. Log a repair line.
   - Several workspace files hold it: no pick, and validation rules as today.
   - No file holds it (a symbol the task will write): rewrite the entry as prose with no anchor, and log a repair
     line.
3. Validation runs after the repairs; regenerate only when a violation is left.

## Done when

Phase 4.2's two rejected drafts, replayed at $0 in a workspace with the files they name, are written after one call
with repair lines and no PLAN_CONTEXT_SYMBOL. A task that owns 6 files may read 6 besides the spec.
