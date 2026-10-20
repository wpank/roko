+++
id = "gap-1a30ca"
kind = "gap"
title = "16 roko-cli integration test files spawn roko with the real HOME, so ~/.roko keys and global config leak into fixtures"
status = "superseded"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "34baabd5b"
source = "roko-delivery, 2026-10-15 (team lead): integration tests spent on Will's OpenAI key and failed at random"
anchors = ["crates/roko-cli/tests/plan_control_ack.rs", "crates/roko-cli/tests/worktree_task_diff.rs", "crates/roko-cli/tests/plan_branch_integration.rs"]
links = { depends_on = [], blocks = [], related = ["reg-e2038b", "bug-c63cca"], supersedes = [], duplicate_of = "bug-eed697" }

[[verify]]
command = "test -z \"$(grep -rl 'cargo_bin(\\\"roko\\\")' crates/roko-cli/tests/*.rs | xargs grep -L 'env(\\\"HOME\\\"')\""

[closed]
at = 2026-10-16
at_ts = "2026-10-15T23:18:23Z"
by = "roko-init"
forced = false
evidence = "Duplicate of bug-eed697 (filed by roko-delivery the same night), whose plan step 1 is exactly this: one helper that runs roko with a scratch HOME in every integration test that spawns it. Its step 2 (an explicit model wins over the built-in ladder) is done by reg-e2038b."
+++

## Problem

16 of the 34 roko-cli integration test files that spawn the `roko` binary (`Command::cargo_bin("roko")`) start it
with the invoking user's environment. That includes HOME, so the binary loads `~/.roko/.env` (provider keys) and
merges `~/.roko/config.toml` (providers, models). A fixture's configuration then competes with the operator's real
keys and models. On Will's machine, fixtures with a fake `claude_cli` provider dispatched to the real
`gpt-5.4-mini` (reg-e2038b), which spent money and made `worktree_task_diff` and `plan_branch_integration`
pass or fail at random. The other 18 files already set `HOME` (e2e.rs `isolated_roko`, plan_control_ack.rs
`roko(dir, home)`), each its own way.

## Why it matters

Tests must not depend on, or spend on, the machine they run on. Until this is fixed, every agent has to run the
integration tests with a scratch HOME (round v5 rule, 2026-10-16).

## Where

- `crates/roko-cli/tests/*.rs`: one `roko(…)` helper per file; there is no shared test-support module.
- `crates/roko-cli/tests/plan_control_ack.rs`: the most complete isolation today. It sets HOME and removes the
  provider keys (`roko_core::child_env::PROVIDER_KEY_VARS`), `ROKO_*`, `RUST_LOG`, `XDG_CONFIG_HOME` and
  `CLAUDECODE`.

## Plan

1. Add a shared helper (e.g. `tests/support/isolated.rs`, included by `#[path]`) that builds a `roko` command
   with a scratch HOME (and USER), and the scrubbing plan_control_ack does.
2. Use it in every test file that spawns `roko`.
3. Guard: a test (or `[[verify]]` check) that every file spawning `roko` goes through the helper.

## Done when

The `[[verify]]` command passes: no roko-cli test file spawns `roko` without the isolating helper.
