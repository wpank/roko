+++
id = "gap-0e2c40"
kind = "gap"
title = "Integration test C2: no provider key reaches an agent, a gate or a log, and the git guard denies destructive commands"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e3"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G2, canary C2)"
anchors = ["crates/roko-cli/tests/secrets_and_git_guard_canary.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-7d7200", "gap-8be530", "gap-5f4852", "bug-7de5df", "bug-a66941"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn secrets_and_git_guard_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test secrets_and_git_guard_canary"
+++

## Problem

Each fix in epic spec-ba7bea has its own unit test. Nothing checks, in one real plan run, that a provider key reaches
no agent, no gate and no file, and that the git guard handed to the agent denies destructive commands.

## Why it matters

This test is the exit check for epic spec-ba7bea. It is canary C2 in W8's gate G2, and it joins the golden-path
acceptance suite (plan epic E11). Paid benchmark arms that run Roko wait for it (W5, sequencing constraints).

## Where

- **New file:** `crates/roko-cli/tests/secrets_and_git_guard_canary.rs`.
- **Pattern:** `crates/roko-cli/tests/graph_budget_resume.rs`: a scripted fake `claude_cli` provider set in
  `roko.toml`, and the built binary run with `assert_cmd`.
- **Helpers to reuse:** those gap-5f4852 leaves in `tests/secret_canary.rs` and `tests/common/mod.rs`.

## Current state

No such test at `41c7ffbd6`. `secret_canary.rs` tests the scrubbers in isolation, and gap-5f4852 covers that gap.

## Plan

1. **Set up two canaries.** Use a temporary `HOME` whose `.roko/.env` holds canary keys (`ANTHROPIC_API_KEY`,
   `OPENAI_API_KEY`), and export a second canary in the `roko` process's own environment.
2. **Script the run.**
   - The fake provider saves its argv and `env` to files in the workdir, then prints a result event.
   - One task's verify step runs `env > gate-env.txt`.
3. **Run `roko plan run` and assert:**
   - neither canary is in the agent's environment dump or the gate's;
   - neither canary is in any file under the workdir or `.roko/`, apart from the `.env` itself;
   - the hook in the recorded `--settings` payload denies:
     - `git reset --hard`, `git clean -fdx`, `git stash` and `cd x && git checkout main`;
     - `cat ~/.roko/.env`;
     - any command when `python3` is missing from `PATH`;
   - argv carries `--setting-sources` (gap-8be530).

## Done when

- [ ] The test exists and passes.
- [ ] Reverting any one of the epic's fixes makes it fail. Check this once by hand and say so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test builds `roko-cli` but edits no hot file. It can be written now and merged last.
- A provider may receive the one key it is configured with. The fake provider has none, so it must receive none.
- Use the fake provider only, with no network, and keep the test under a minute.
- 2026-09-30 (wk-canary): Implemented on `work/gap-5f4852` at `0483ba61c`, on top of gap-5f4852 (`551a339e1`). `cargo test -p roko-cli --test secrets_and_git_guard_canary` passes in the worktree in about 1 s. The fake agent's reply quotes a home key, so the file scan also depends on gap-5f4852. `agent_tool_shells_exclude_provider_keys` (roko's own `run_tests` and `bash` through a fake OpenAI-compatible provider) is `#[ignore = "bug-0d9ac4"]`: run with `--include-ignored`, it fails only on `run_tests`; `bash` is clean. Un-ignore it once bug-0d9ac4 merges. The by-hand revert check in Done-when was not run (each revert needs a rebuild). Each fix maps to an assertion: agent and verify environments (bug-7d7200), `--setting-sources` (gap-8be530), the file scan (gap-5f4852), git denials (bug-7de5df), `cat ~/.roko/.env` (bug-a66941). Re-run after merging batch 12a (telemetry2) at `09e0a93ff`: still passes; batch 12b (model-truth) not yet covered.
- 2026-09-30 (wk-canary): the log scrubber learns config secrets only from providers' `api_key_env` (bug-5a6636), so `extra_headers`, file secrets and `serve.auth.api_key` can reach records; and it treats every `.env` value of 8+ characters as a secret (bug-cef888). C2 should cover both.
- 2026-09-30 (wk-canary): Un-ignored `agent_tool_shells_exclude_provider_keys` on `work/gap-0e2c40` at `ff1077bf8` (bug-0d9ac4 fully merged: batch 13 and 15a); cargo verification deferred to the batch check.
