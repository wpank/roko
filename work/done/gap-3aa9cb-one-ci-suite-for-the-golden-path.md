+++
id = "gap-3aa9cb"
kind = "gap"
title = "One CI suite for the golden-path integration tests C1–C8, with a shared scripted fake provider"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests", ".github/workflows"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e11"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (package the canaries)"
anchors = ["crates/roko-cli/tests/common/scripted_provider.rs", "crates/roko-cli/tests/golden_path_suite.rs", ".github/workflows/ci.yml"]
lane = "rust-cold"
parent = "spec-f09094"
links = { depends_on = ["gap-cd3529", "gap-0e2c40", "gap-af00b1", "gap-b954ad", "gap-987064", "gap-9eebcb", "gap-e21595"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn golden_path_suite_covers_c1_to_c8' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_suite"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:16Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including golden_path_suite_covers_c1_to_c8 (2/2): all eight canaries use common::scripted_provider and run in the golden-path CI job. Merged b66043af3 (work/gap-3aa9cb 77d7baca2)."
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
- Implemented on `work/gap-3aa9cb` at `8e9a5b1b2`; cargo verification deferred to the batch check. The canaries ran
  in wk-specq's own target clone.
- 2026-10-01 (wk-specq): six of the eight canaries are on the shared provider: C2, C3 and C4, C5, C6 and C8.
  - `tests/common/scripted_provider.rs` installs a POSIX `sh` fake Claude CLI and the `Script` it plays. It takes a
    call's task from roko's `Task: <id>: ` system-prompt lines, numbers the task's attempts, and logs each call
    (prompt, argv, environment, cwd, task, attempt, `--model`). Per task and attempt, a turn writes or appends
    files, holds while a file exists, stays silent, prints a stream-json reply (text, usage, cost, model slug) or
    malformed or overlong output, and exits with its status. `@TASK@` and `@MODEL@` fill in edit paths and output.
    Turns compile to plain files, so the script needs no JSON parser; `script.json` records the `Script`.
  - Each canary's own fake is gone, and no canary file or test is renamed. C3 and C4 found their task by title,
    C8 by the second prompt line, and C5 by a `next-action` file the test wrote; all now key on task id and
    attempt. C5's T4 makes its honest change, then changes nothing on the `--fresh` rerun.
  - C2's `agent_tool_shells_exclude_provider_keys` keeps its OpenAI-compatible HTTP fake, which is a different
    provider kind. Its Claude CLI configuration uses the shared provider. `ScriptedPlanWorkspace::with_provider`
    puts the provider outside the repository and the fixtures.
  - Pass counts: C2 2/2, C3 and C4 2/2, C5 2/2, C6 1/1, C8 1/1. `secret_canary`, which shares
    `ScriptedPlanWorkspace`, passes 10/10. The canaries ran in about 40 s at load 110-140.
  - `golden_path_suite_covers_c1_to_c8` fails, as it should for now, on C1 (`honest_verdicts_canary`, on
    work/bug-7e1b6b) and C7 (`supervision_canary`, gap-9eebcb): their files do not exist yet and the CI job does
    not list them.
  - CI changes for everyone: `.github/workflows/ci.yml` gains a `golden-path` job. On every pull request and push
    to main it builds and runs the six canary targets, with a 10-minute step for the canaries in a 45-minute job.
    Until C1 and C7 land, the guard test also fails the `test` job's `cargo test --workspace`.
- 2026-10-01 (wk-specq): C1 and C7 landed at `ba470bec8`, merged in at `60102c497`, and moved onto the shared
  provider in `a0cd40526`. All eight canaries are now on it, and the golden-path job runs all seven targets.
  - C1 (`honest_verdicts_canary`) plays one default turn that appends to `NOTES.md`.
  - C7 (`supervision_canary`) needed two things the provider lacked, so it now has them: `Output::Message` (one
    assistant message and no result) and `Turn::then_silent_for` (after its output, the provider becomes
    `sleep N` with the same pid). Each call also logs the provider's pid.
  - Checked statically only, because free disk was 20-23 GB: nightly fmt is clean, the verify's grep passes, and a
    script that reads ci.yml the way the guard test does finds every row met.
  - The coordinator's batch gate builds the branch and runs the eight canaries and `golden_path_suite`.
