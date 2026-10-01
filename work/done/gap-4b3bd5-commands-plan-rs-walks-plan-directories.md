+++
id = "gap-4b3bd5"
kind = "gap"
title = "commands/plan.rs walks plan directories itself instead of reusing plan_validate's collect_tasks_files"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands/plan", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report on gap-46ab3f, branch work/gap-46ab3f)"
anchors = ["crates/roko-cli/src/plan_validate.rs", "crates/roko-cli/src/commands/plan.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-46ab3f"], blocks = [], related = ["gap-46ab3f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'pub(\\(crate\\))? fn collect_tasks_files' crates/roko-cli/src/plan_validate.rs && grep -q 'collect_tasks_files' crates/roko-cli/src/commands/plan.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:38Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:31Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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

## Notes
- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check.
  `plan_validate::collect_tasks_files` is now `pub` and documented. It must be `pub`, not `pub(crate)`:
  `commands/plan.rs` belongs to the `roko` binary crate. `cmd_plan_validate`'s `--spec-quality` lint calls it,
  and `validated_tasks_files`, the second walk, is gone.
