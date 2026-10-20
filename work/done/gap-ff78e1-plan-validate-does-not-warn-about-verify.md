+++
id = "gap-ff78e1"
kind = "gap"
title = "plan validate does not warn about verify steps that cannot pass (a runner not installed, a test its file does not define, a test file no task writes); only plan generation checks them"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-17
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "9842a6411"
source = "roko-planner, gap-ecafde follow-up (round v6)"
discovered_from = "gap-ecafde"
anchors = ["crates/roko-cli/src/plan_validate.rs::validate_tasks_file", "crates/roko-cli/src/plan_generate/runnable.rs::problems"]
links = { depends_on = [], blocks = [], related = ["gap-ecafde", "bug-6fa025"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_about_steps_that_cannot_pass' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_validate_warns_about_steps_that_cannot_pass"

[closed]
at = 2026-10-21
at_ts = "2026-10-20T22:33:53Z"
commit = "86c8d981f"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T19:21:35Z"
forced = false
evidence = "Merged 9842a6411 (work/gap-ff78e1: cf609c84f red test, 86c8d981f). runnable::unrunnable_steps runs generation's step_problem per step; Probe::portable() takes every command and module as present, so static plan validate reports the same everywhere (the lead's decision): PLAN_059 warning for a build tool without its manifest, a missing file no task writes before the step, or an undefined test. plan_validate::validate_plans_dir_on_this_machine (plan validate --spec-quality --dynamic) adds PLAN_060 for a runner or Python module this machine lacks. Steps PLAN_054/055/056/058 (not a guess) name are left to them. The scan's false positives fixed for generation too: files stop at the first cd, negated tests need no file, ( and ) words are no paths, missing-file and undefined-test checks need an &&-only plain line. Post-merge verify at 9842a6411: grep OK; cargo test -p roko-cli --lib plan_validate_warns_about_steps_that_cannot_pass: 1 passed. Branch: roko-cli lib 3939 passed, 0 failed; plan_validation 38/38; plan_validate 23/25 (the 2 failures are bug-2a31bc's PLAN_041 counts, open); clippy -D warnings clean; nightly fmt. Static scan with the branch binary: roko's 133 plans 0 hits (52 false ones before the cd/negation fixes); Undertow's phase plans 4, all in rumble-2 and real (cargo steps in a CMake repo); v6 mori imports 25, about half files their scan workspace lacks but bardo has."
+++

## Problem

gap-ecafde checks at generation that each verify step can pass:
- its runner is installed;
- a pytest or unittest id it names exists in its file;
- a test file it runs is written by its task or one before it.

`roko plan validate` does not run these checks. So a hand-written, imported (`plan import --mori`) or edited plan can
still carry a step that exits 127, or one that names a test its file does not define. bug-6fa025 is the run-time side
of the same case. The checks live in `crates/roko-cli/src/plan_generate/runnable.rs` (`problems`), which is private
to plan generation. They were not wired into `plan_validate.rs`, because roko-delivery's gap-b1f7b9 was changing
that file.

## Why it matters

`plan validate` is the lint a person runs before a plan runs. A check that cannot pass should be caught there, not on
the first attempt (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_validate.rs::validate_tasks_file`.
- `crates/roko-cli/src/plan_generate/runnable.rs::problems`.

## Plan

1. Make `runnable::problems` (and its `Probe`) reachable from `plan_validate`.
2. Report each problem as a warning: a new PLAN rule naming the task, the step and why.

## Done when

`[[verify]]` passes: `plan validate` warns about a step whose runner is missing, and about one that names a test its
file does not define.
