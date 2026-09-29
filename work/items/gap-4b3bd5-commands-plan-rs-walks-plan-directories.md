+++
id = "gap-4b3bd5"
kind = "gap"
title = "commands/plan.rs walks plan directories itself instead of reusing plan_validate's collect_tasks_files"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands/plan", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report on gap-46ab3f, branch work/gap-46ab3f)"
anchors = ["crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/commands/plan.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-46ab3f"], blocks = [], related = ["gap-46ab3f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'pub(\\(crate\\))? fn collect_tasks_files' crates/roko-cli/src/plan_validate.rs && grep -q 'collect_tasks_files' crates/roko-cli/src/commands/plan.rs"
+++

## Problem

`plan_validate.rs` has a private `collect_tasks_files` (:261), with `collect_tasks_files_recursive` (:285), that finds every `tasks.toml` under a plan directory.

On `work/gap-46ab3f` (not merged at ad391f99a), `commands/plan.rs` adds `validated_tasks_files` (:2030 on the branch), which walks the directories again with its own `std::fs::read_dir` loop.

## Why it matters

Hygiene (epic spec-9a3131): two walks can disagree about which plans exist, for example on symlinks, hidden directories or nesting depth, so `plan validate` and the command that relies on it can see different plan sets.

## Where

`collect_tasks_files` in `plan_validate.rs`, and `validated_tasks_files` in `commands/plan.rs` on the branch.

## Current state

The duplicate exists only on the branch. This item depends on gap-46ab3f.

## Plan

1. Make `collect_tasks_files` `pub(crate)`, and have `commands/plan.rs` call it instead of walking itself.
2. Keep one set of rules for which directories count as plans.

## Done when

- [ ] One function finds `tasks.toml` files for both commands.
- [ ] The `[[verify]]` command passes.
