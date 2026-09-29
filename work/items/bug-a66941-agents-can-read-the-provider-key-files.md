+++
id = "bug-a66941"
kind = "bug"
title = "Agents can read the provider key files, such as ~/.roko/.env"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety", "roko-agent/claude_cli_agent"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e3"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #3: make key files unreadable to agents); workstreams/assessment/W10-benchmarks-proof.md (VB_SECRET leak)"
anchors = ["crates/roko-agent/src/safety/path.rs::canonicalize_with_policy", "crates/roko-agent/src/claude_cli_agent.rs::build_settings_json"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-7d7200"], blocks = [], related = ["bug-7de5df", "gap-5f4852"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn path_policy_denies_provider_key_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib path_policy_denies_provider_key_files"

[[verify]]
command = "grep -rqw 'fn settings_json_denies_reading_key_files' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_json_denies_reading_key_files"
+++

## Problem

Provider keys live in plain files:

- `~/.roko/.env` and `<workdir>/.roko/.env`, which `load_startup_env_files` loads (`main.rs:4499`);
- `<workdir>/.roko/secrets.toml` (`secrets.rs:51`) and `~/.roko/credentials.json` (`credentials.rs::credentials_path`).

Agents can read all of them:

- **The Claude CLI agent** runs with `dangerously_skip_permissions: true` (`claude_cli_agent.rs:128`), so its Read
  tool or a Bash `cat ~/.roko/.env` returns the keys.
- **Roko's own file tools** confine paths to the worktree (`safety/path.rs`). That keeps `~/.roko` out, but not
  `.roko/.env` or `.roko/secrets.toml` when the plan runs in the main checkout, which is the default.

## Why it matters

bug-7d7200 takes the keys out of children's environments, but that is moot while an agent can read the file. This is
tldr/05 P0 #3. W10 found a second leak: the benchmark's `VB_SECRET` sits in `~/.roko/.env`, so the bench's check SC4
fails by construction. It is part of epic spec-ba7bea.

## Where

- `crates/roko-agent/src/safety/path.rs::canonicalize_with_policy`: the path check every roko file tool goes through.
- `crates/roko-agent/src/claude_cli_agent.rs::build_settings_json`: the Claude CLI `--settings` payload.
- The list of key files belongs next to `roko_core::child_env`, which bug-7d7200's branch adds.

## Current state

Unchanged at `41c7ffbd6`. bug-7d7200's fix is uncommitted in `../roko-wt-env`. It records which variable names the
`.env` files supply (`roko_core::child_env::DotenvNames`), but it leaves the files readable.

## Plan

1. One function returns the key-file paths for a given home and workdir.
2. `canonicalize_with_policy` rejects those paths with a new `ToolError` reason, inside the worktree as well.
3. The Claude CLI payload denies them in three ways:
   - read-deny permission rules;
   - a PreToolUse hook on `Read|Edit|Write|Grep|Glob` that checks `file_path` and `path`;
   - a Bash rule that denies commands naming those files.

   Hooks run in every permission mode. Check whether the deny rules still hold under `--dangerously-skip-permissions`.

## Done when

- [ ] Roko's file tools refuse `.roko/.env`, `.roko/secrets.toml` and `~/.roko/*`.
- [ ] The Claude CLI hook denies a `Read` of `~/.roko/.env` and the command `cat ~/.roko/.env`.
- [ ] Tests `path_policy_denies_provider_key_files` and `settings_json_denies_reading_key_files` pass: the two
      `[[verify]]` commands.

## Notes

- **Wait for bug-7d7200.** Do this after bug-7de5df, which edits the same function.
- **This is best effort without an OS sandbox.** An obfuscated `python3 -c open(…)` still reads the file. Keeping
  keys out of the agent user's reach (a sandbox, the keychain or a separate user) is a decision for the author.
- Implemented on `work/bug-7de5df` at `9c0d7196b`; cargo verification deferred to the batch check.
