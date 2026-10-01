+++
id = "bug-009c0e"
kind = "bug"
title = "roko prd plan's planner prompt shows max_parallel = 1 in its required plan structure, so most plans run serially"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-repin's report)"
anchors = ["crates/roko-cli/src/prd.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-272448", "gap-a8d786"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/^async fn generate_plan_from_prd_with_outcome(/,/^}/p' crates/roko-cli/src/prd.rs | grep -q 'max_parallel = 1'"
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
