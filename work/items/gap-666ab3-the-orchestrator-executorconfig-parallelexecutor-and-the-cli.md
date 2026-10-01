+++
id = "gap-666ab3"
kind = "gap"
title = "The orchestrator ExecutorConfig, ParallelExecutor and the CLI Config.executor field are parsed but never used in production"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7a3527"
anchors = ["crates/roko-cli/src/config.rs", "crates/roko-cli/src/orchestrator/"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'ParallelExecutor' crates/roko-cli/src --include=*.rs | grep -v test && grep -rqw 'fn removed_executor_section_still_loads' crates/roko-core/src/ && cargo test -p roko-core --lib removed_executor_section_still_loads"
+++

## Problem

`ExecutorConfig`, `ParallelExecutor` and `Config.executor` are parsed but have no production reader, so `[executor]` settings look effective when they are not.

## Plan

Delete them, or wire them where the Graph path needs them. A removed key goes into REMOVED_CONFIG_KEYS (gap-6bc156) with a warning on load.

## Done when

- The verify passes, and old roko.toml files still load.

## Notes

- Reported on 2026-10-01 by the worker on gap-7a3527, during the evening close-out round.
- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise checked at `c58c7c2ba`: outside its own tests, nothing built a `ParallelExecutor` (only
  `tests/merge_proof.rs` did), and nothing read `Config.executor`, which `Config::from_roko_config` always set to
  `ExecutorConfig::default()`. Deleted `ExecutorConfig`, `ParallelExecutor`, the `ResourceBudget`/`ResourceUsage`
  sub-config that only `ExecutorConfig` used, the CLI `Config.executor` field, the unused `ExecutorLayer` and the
  `executor.*` arms of `parse_value_for_key`. `ParallelExecutor` was the only user of the crate-private
  `orchestrator::safety::audit_chain`, which would otherwise be dead code, so that legacy module went too. The state
  types it drove stay: `ExecutorSnapshot` (the persisted `executor.json`), `PlanState`, `PlanStateMachine`,
  `RecoveryEngine` and the rest.
- Migration: `[executor]` is a top-level `REMOVED_CONFIG_KEYS` entry (`remove_dotted_key` now handles top-level keys).
  Validation reports it as removed and names `conductor.max_parallel_plans` and `--worktree-per-task`; the core loader
  strips it with that warning; `RokoConfig::from_toml` and the CLI's `Config::parse_toml` (`parse_toml_with_env`) drop
  it with the warning. Tests: `removed_executor_section_still_loads` (roko-core) and `old_executor_section_still_parses`
  (roko-cli, which replaces `parses_executor_section_from_toml`). `tests/merge_proof.rs` now checks the merge
  transitions on `PlanStateMachine::transition` directly. The verify also runs the loader test, since "old roko.toml
  files still load" is part of Done when.
- The LEGACY_GATES wording (`--engine legacy`) was already fixed in `5e31a5e14` (gap-7a3527).
- For gap-4ec59f (wk-tiers): its step 2 plans to honour `[executor] use_worktrees` by first adding the field to
  `RokoConfig`. If it keeps the `[executor]` name, it must remove the "executor" entry from `REMOVED_CONFIG_KEYS`.
