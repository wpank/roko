+++
id = "gap-ff78e1"
kind = "gap"
title = "plan validate does not warn about verify steps that cannot pass (a runner not installed, a test its file does not define, a test file no task writes); only plan generation checks them"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "roko-planner, gap-ecafde follow-up (round v6)"
discovered_from = "gap-ecafde"
anchors = ["crates/roko-cli/src/plan_validate.rs::validate_tasks_file", "crates/roko-cli/src/plan_generate/runnable.rs::problems"]
links = { depends_on = [], blocks = [], related = ["gap-ecafde", "bug-6fa025"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_about_steps_that_cannot_pass' crates/roko-cli/src && cargo test -p roko-cli --lib -- plan_validate_warns_about_steps_that_cannot_pass"
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
