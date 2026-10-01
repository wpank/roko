+++
id = "bug-63327d"
kind = "bug"
title = "roko-std's file tools don't check key files, so a dispatch without SafetyLayer can read .roko/.env"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "abc655b5e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-guard's report on bug-a66941)"
anchors = ["crates/roko-std/src/tool/builtin/read_file.rs", "crates/roko-core/src/child_env.rs::is_key_file"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn std_file_tools_refuse_key_files' crates/roko-std/src/ && cargo test -p roko-std --lib std_file_tools_refuse_key_files"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "roko-std's file tools refuse key files in sandbox::require_within_worktree and grep skips them (c37492fd8; merged abc655b5e). Batch 7 gate (work/rust-batch-5 tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only 15d3eb5f2; clippy -p roko-cli -p roko-learn -p roko-gate -p roko-agent -p roko-std -p roko-core --no-deps -D warnings clean; lib tests (8 threads) roko-cli 3091, roko-agent 2252 (after test fix f7ad8de76), roko-core 1923, roko-learn 1178, roko-std 221, roko-gate 685 (one pre-existing flaky test, tautology_filter_discards_preexisting_passing_tests, failed once and passed on rerun)."
+++

## Problem

bug-a66941 made `SafetyLayer`'s path policy refuse provider key files, through `roko_core::child_env::is_key_file` and `ToolError::KeyFileBlocked`. roko-std's builtin file tools don't call that check themselves. Any dispatch path that runs them without `SafetyLayer` can still read `.roko/.env` (wk-guard, 2026-09-29).

## Why it matters

The key-file block should hold whichever dispatcher runs the tool. Epic spec-ba7bea.

## Where

`crates/roko-std/src/tool/builtin/` (`read_file.rs` and its siblings that read, list or search files), plus `child_env::is_key_file`.

## Current state

Only `SafetyLayer` checks for key files.

## Plan

1. Check `is_key_file` in every builtin tool that reads file contents. Check both the path as given and the resolved path.
2. Return the same `KeyFileBlocked` error.
3. Add the test `std_file_tools_refuse_key_files`.

## Done when

- [ ] The builtin file tools refuse key files without `SafetyLayer`.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `407ce30d5` by reading the code: roko-std's `sandbox::require_within_worktree` was purely lexical and never called `is_key_file`, so `read_file` on `.roko/.env` returned it.
- Fixed in `crates/roko-std/src/tool/builtin/sandbox.rs` (`refuse_key_file`, called by `require_within_worktree`, which every path-taking builtin uses) and in grep's walk, which skips key files and symlinks to them. `read_file.rs` itself is unchanged.
- New problem (report): roko's own `bash` builtin and SafetyLayer's bash policy do not check commands for key files, so an API-provider agent can still `cat .roko/.env` through the bash tool.
- Implemented on `work/bug-f4e133` at `c37492fd8`; cargo verification deferred to the batch check.
