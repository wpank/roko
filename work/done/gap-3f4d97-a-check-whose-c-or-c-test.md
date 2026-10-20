+++
id = "gap-3f4d97"
kind = "gap"
title = "A check whose C or C++ test uses a type or includes a header that only a later task writes cannot compile when its task ends, and PLAN_058 does not see it (Undertow phase 4's first spec, P4.02)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "c02d7ac8d"
source = "round-v7-team-lead#phase-0-finding-2"
discovered_from = "agent:undertow-spec (round v7, phase 4 spec proof)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::later_files"]
links = { depends_on = [], blocks = [], related = ["gap-970f67"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_test_that_uses_a_later_tasks_type_is_plan_058' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_test_that_uses_a_later_tasks_type_is_plan_058"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T19:42:26Z"
commit = "1bc0bd6cb"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "b1583eb04 (merged 0fe899c95) and 1bc0bd6cb (merged c02d7ac8d). In a CMake workspace runnable::TestCode reads a step's C/C++ tests (the file its command names, or <word>_test.<ext>), follows quoted includes 3 levels and strips comments, strings and preprocessor lines; a later task's file is needed when an unresolved include ends with its path (PLAN_058 error when every writer runs after the task) or, a guess, when the test uses its CamelCase name (4+ chars) that no non-test code uses (PLAN_058 warning that never sends a plan back, the lead's condition). Post-merge verify at c02d7ac8d: grep OK; cargo test -p roko-cli --lib a_test_that_uses_a_later_tasks_type_is_plan_058 a_check_needing_a_later_tasks_file_is_plan_058: 2 passed. Branch: roko-cli lib 3935 passed (the one failure, a severity-order assertion, fixed and rerun green), clippy -D warnings and nightly fmt clean. Scan with the branch binary: repo plans 0/133, Undertow phases p0/p1/rumble-2/rumble-3 at their bases 0/4, v6 mori imports 8 hits (7 real in 05-terminal-widgets, 1 likely an artifact in 71-mvp-gate), a4108ae's P4.02 shape: the UiFrame guess as a warning (the true case)."
+++

## Problem

Undertow's first phase-4 spec (a4108ae) gave the plumbing task P4.02, checked by `tools/spec.sh 4 state`. Its test,
`tests/spec/p4/state_test.cpp`, included the editor test helpers (`tests/spec/support/spec_ui.h`), and those use the
type `UiFrame`. Only the next task's `plugin/ui/UiFrame.h` declares that type. So the check could not compile when
its task ended. undertow-spec found it while proving the spec, and fixed the test in a8031ad.

PLAN_058 (gap-970f67) sees only files a step names. Every file this test includes existed already; what was missing
was a type. The same goes for a test that includes a header only a later task writes.

## Why it matters

In a C or C++ project, a task's check compiles a test. A test that needs a later task's work makes its task fail
every try, and the spec, the planner and roko should all see that before the run.

## Where

- `crates/roko-cli/src/plan_generate/runnable.rs`: `later_files` (PLAN_058) and generation's `problems`.

## Plan

In a CMake workspace, read the C or C++ tests a step runs: a file its command names, or `<word>_test.<ext>` for a
word of the command. Follow their `#include "…"` lines through the workspace's own headers (3 levels), with
comments and strings stripped.

A later task's file is needed when either:
- an include no workspace file answers ends with its path; or
- the test code uses its CamelCase name as an identifier (`UiFrame`), and no non-test code uses that name yet.

Report it as PLAN_058, with how the test needs it.

## Done when

A P4.02-shaped plan gets PLAN_058 for the type case and the include case. A name used only in a comment or a string,
and a test whose task waits for the writer, get nothing. Checked by the test in `[[verify]]`.

## Notes

- Evidence: Undertow a4108ae (`tests/spec/p4/state_test.cpp`, `tests/spec/support/spec_ui.h`) and phase-4-ui.md's
  notes for the planner (task 1, P4.02).
- Related: gap-970f67 (PLAN_058).
