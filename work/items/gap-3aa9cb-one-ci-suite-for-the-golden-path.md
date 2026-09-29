+++
id = "gap-3aa9cb"
kind = "gap"
title = "One CI suite for the golden-path integration tests C1–C8, with a shared scripted fake provider"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests", ".github/workflows"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e11"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (package the canaries)"
anchors = ["crates/roko-cli/tests/common/scripted_provider.rs", "crates/roko-cli/tests/golden_path_suite.rs", ".github/workflows/ci.yml"]
lane = "rust-cold"
parent = "spec-f09094"
links = { depends_on = ["gap-cd3529", "gap-0e2c40", "gap-af00b1", "gap-b954ad", "gap-987064", "gap-9eebcb", "gap-e21595"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn golden_path_suite_covers_c1_to_c8' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_suite"
+++

## Problem

Seven items in five epics write the eight canaries C1–C8. Each would copy a fake provider into its own test file, as
`graph_budget_resume.rs` does today, and CI runs them only inside the one `cargo test --workspace` job. So the fakes
drift apart, and a canary failure does not show up as "the regulator regressed".

## Why it matters

Assessment W8: package the canaries as the regulator's regression suite and rerun it on every change to gate code.
Roko should never change its own regulator code without an outside check. Part of epic spec-f09094; gap-f30b8e
builds on the shared provider.

## Where

- **New file:** `crates/roko-cli/tests/common/scripted_provider.rs`, declared in `tests/common/mod.rs` (which already
  has `write_executable` and `seed_git_repo`).
- **New file:** `crates/roko-cli/tests/golden_path_suite.rs`: the guard test.
- `.github/workflows/ci.yml`: a new `golden-path` job.
- The canaries' own files, as named in each C item's `[[verify]]`: `honest_verdicts_canary.rs` (C1),
  `plan_branch_integration.rs` (C3, C4), `attempt_diff_canary.rs` (C5), and the others' when they are written.

## Current state

Checked at `41c7ffbd6`: no canary exists yet. `tests/common/mod.rs` has one fixed mock Claude script
(`MOCK_CLAUDE_SCRIPT`) that always emits the same stream.

## Plan

1. **Scripted provider:** a POSIX `sh` fake Claude CLI plus a JSON script, written into the test workspace and
   configured through `roko.toml`. It matches the task id in the prompt, and per task and attempt it can:
   - apply file edits;
   - emit stream-json with usage, cost and a model slug (for C8's ladder);
   - exit non-zero, stay silent for N seconds, or emit malformed or overlong output.

   It logs every call for the tests to assert on.
2. Move each canary onto it as it lands. Do not rename canary files or tests: closed items' verify commands name them.
3. **Guard test** `golden_path_suite_covers_c1_to_c8`: a table of the eight canaries and their test targets. It
   asserts that each file exists, uses `common::scripted_provider`, and appears in the CI job's command.
4. **CI job** `golden-path`: `cargo test -p roko-cli` with one `--test` per canary target, on every pull request,
   within a 10-minute budget for the suite.

## Done when

- [ ] All eight canaries use the shared provider and pass in the `golden-path` job.
- [ ] The `[[verify]]` command passes.

## Notes

- Start once two canaries exist. The item closes only when all eight are on the shared provider.
- Changing `ci.yml` changes CI for everyone; say so in the pull request.
