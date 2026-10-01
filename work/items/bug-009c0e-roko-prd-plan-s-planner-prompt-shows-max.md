+++
id = "bug-009c0e"
kind = "bug"
title = "roko prd plan's planner prompt shows max_parallel = 1 in its required plan structure, so most plans run serially"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "849449ded"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-repin's report)"
anchors = ["crates/roko-cli/src/prd.rs", "crates/roko-cli/src/commands/plan.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-272448", "gap-a8d786"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/^async fn generate_plan_from_prd_with_outcome(/,/^}/p;/^pub async fn generate_plan(/,/^}/p' crates/roko-cli/src/prd.rs | grep -q 'max_parallel = 1'"

[[verify]]
command = "grep -qF '# Add [[task]] entries below' crates/roko-cli/src/commands/plan.rs && ! grep -F '# Add [[task]] entries below' crates/roko-cli/src/commands/plan.rs | grep -q max_parallel"
+++

## Problem

132 plans under `plans/` set `max_parallel` explicitly, and 102 of them set it to 1. The source is `roko prd plan`'s planner prompt. In `generate_plan_from_prd_with_outcome` (`crates/roko-cli/src/prd.rs:1329`), the "MINIMUM REQUIRED STRUCTURE" example the model must follow includes `max_parallel = 1` in `[meta]` (:1830-1840). The newer `plan generate` path omits the field on purpose ("tasks that do not depend on each other run together", `plan_generate.rs:209`; gap-272448). `plan_generator.rs`'s `max_parallel = 1` lines are all in its tests.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): parallel execution of independent tasks is the thesis's claim. Plans generated through `roko prd plan` serialize everything. p3.

## Where

The prompt string in `generate_plan_from_prd_with_outcome`.

## Plan

1. Remove `max_parallel = 1` from the required-structure example (or show the field omitted, with the comment `plan_generate.rs` uses).
2. Optionally have `plan validate` warn when a plan sets `max_parallel = 1` while it has independent tasks (gap-a8d786 lints related cases).

## Done when

- [ ] `roko prd plan` no longer asks for `max_parallel = 1`.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/gap-2623b2` at `5e39cb9d5`, with gap-2623b2; cargo verification deferred to the batch check.
- gap-2623b2 moved the planner prompts out of `generate_plan_from_prd_with_outcome`, now a thin wrapper, into
  `generate_plan`, the one plan generator. The retry prompt's "MINIMUM REQUIRED STRUCTURE" example now shows the
  comment the shared system prompt uses (`# max_parallel is omitted: tasks that do not depend on each other run
  together`). The verify now reads `generate_plan` too, where the prompt lives; it read only the wrapper and would
  pass vacuously.
- `roko plan create`'s scaffold (`commands/plan.rs`) wrote `max_parallel = 1` into every hand-created plan's `[meta]`.
  It now omits the field (`849449ded`), and the second `[[verify]]` checks the scaffold. Test fixtures keep their
  `max_parallel = 1`.
- The 132 existing plans are unchanged. Plan step 2 (a `plan validate` warning) is not done.
