+++
id = "bug-1bbe2b"
kind = "bug"
title = "Reading a spec's named files panics on a deletion sentence whose code span comes before its verb (`plugin/ui/`: delete …): plan writing, plan validate and, since the deletion gate, every task dispatch of such a plan crash"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "0465bbcb4"
source = "round v8 roko-planner-v8: the $0 probe of roko's work items with the gap-708931 binary (plan validate exit 101)"
discovered_from = "agent:roko-planner-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/named_files.rs::instruction"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-aa93e9", "bug-4e89b8", "gap-708931"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_span_before_the_verb_of_deletion_is_changed_not_deleted' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_span_before_the_verb_of_deletion_is_changed_not_deleted"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T19:39:00Z"
commit = "0465bbcb4"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "S"
forced = false
evidence = "instruction() calls a code span before the verb of a deletion sentence the file being changed, never deleted, and looks for from/in/of only between spans after the verb, so the inverted range that panicked ('byte range starts at 51 but ends at 0') is gone. Red first (9535316c4 on main 395d58380): a_span_before_the_verb_of_deletion_is_changed_not_deleted panicked at named_files.rs:329. Green at 0465bbcb4 on the trial 99ebe01bd (main 395d58380 + this branch + gap-708931): 66 named_files, plan_validate and verification tests pass, roko-cli lib 4145 passed 0 failed, plan_validate 27/27, plan_validation 40/40, cargo clippy --workspace --no-deps -D warnings and nightly fmt --check clean; the $0 probe of 289 roko work items that crashed plan validate on items 61-120 runs to the end."
+++

## Problem

Reading a spec's named files panics on a deletion sentence whose code span comes before its verb.

`plan_generate/named_files.rs::instruction` (bug-aa93e9, merged ba5866664) classifies each code span of a sentence led
by "Delete"/"Remove"/"Retire". It looks for "from"/"in"/"of" in `prose[since..span.start()]`, where `since` starts at
the end of the verb. A span before the verb, such as a lead label (`` `plugin/ui/`: delete `Look.h` ``) or a subject
(`` `src/a.rs`: remove the old function ``), starts before `since`. The range is then inverted and the slice
panics: "byte range starts at 51 but ends at 0".

- Found by my $0 probe of roko's work items with the gap-708931 binary: `plan validate` exited 101 on one of items
  61–120.
- The same reading runs in plan generation (bug-aa93e9's repair), in `plan validate` (PLAN_062), and since bug-4e89b8
  in every Graph task's `verify_steps`, the deletion gate. A plan whose `[meta] sources` holds such a sentence would
  crash its run.
- A sentence led by an edit verb never reaches the slice, which is why phase 1's specs passed.

## Why it matters

- A crash in the plan writer, the validator and the runner, on ordinary spec wording.
- A span before the verb names the file the sentence is about, which is edited, not deleted. Calling it a deletion
  would make the deletion gate fail every try.

## Where

- `crates/roko-cli/src/plan_generate/named_files.rs::instruction`.

## Plan

1. A span that starts before the verb is changed, never deleted. A span after it is deleted unless "from", "in" or
   "of" comes between it and the span before it (or the verb).
2. Test `a_span_before_the_verb_of_deletion_is_changed_not_deleted`: "`plugin/ui/`: delete `plugin/ui/Look.h` and
   `plugin/ui/Look.cpp`." deletes the two files, and "`core/src/engine.cpp`: remove the old clock." changes
   `engine.cpp`. Neither panics.

## Done when

- The `[[verify]]` command passes, and the probe of roko's work items runs to the end.

## Notes

- Related: bug-aa93e9 (the reading), bug-4e89b8 (the deletion gate that uses it), gap-708931.
