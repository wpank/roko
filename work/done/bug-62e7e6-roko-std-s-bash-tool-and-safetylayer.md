+++
id = "bug-62e7e6"
kind = "bug"
title = "roko-std's bash tool and SafetyLayer's bash policy never check commands for key files"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "89f4b09ea"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:55, wk-guard2's report on bug-f4e133, bug-66f5a1, bug-63327d and find-570af2)"
anchors = ["crates/roko-std/src/tool/builtin/bash.rs", "crates/roko-agent/src/safety/mod.rs", "crates/roko-core/src/child_env.rs::is_key_file"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-63327d", "bug-a66941", "find-570af2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bash_commands_naming_key_files_are_refused' crates/roko-std/src/ crates/roko-agent/src/ && cargo test -p roko-agent --lib bash_commands_naming_key_files_are_refused"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "roko_std sandbox::refuse_key_file_in_command (the guard's key-file rules on dequoted words and nested sh -c, words resolved with symlinks) runs in roko-std's bash handler and in SafetyLayer before the bash policy; ~/.roko/config.toml counts as a key file (9a9a2ba5e; merged). Batch 9 gate (dedicated target dir, batch tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-learn -p roko-agent -p roko-std -p roko-core -p roko-daimon -p roko-neuro --no-deps -D warnings clean; lib tests roko-cli 3098, roko-agent 2254, roko-core 1925, roko-learn 1181, roko-neuro 239, roko-std 222, roko-daimon 100, 0 failed."
+++

## Problem

bug-63327d made roko-std's file tools refuse key files, and the Claude CLI hooks deny them. roko-std's bash tool and SafetyLayer's bash policy don't check the command at all, so an API-provider agent can still run `cat .roko/.env` (wk-guard2, 2026-09-29). Also, since find-570af2 moved to key-file names only, `~/.roko/config.toml` is readable, and it can hold `serve.auth.api_key`.

## Why it matters

API-provider agents (OpenAI-compatible, Cerebras, Gemini API) have no Claude hooks. The bash policy is their only guard. Epic spec-ba7bea.

## Where

`crates/roko-std/src/tool/builtin/bash.rs`, the bash policy in `crates/roko-agent/src/safety/`, and `child_env::is_key_file`.

## Current state

Only the file tools and the Claude hooks refuse key files.

## Plan

1. Parse bash commands the way `claude_cli_guard.py` does: quote-aware, following wrappers and `sh -c`. Refuse any command that names a key file.
2. Decide whether `~/.roko/config.toml` counts as a key file when it holds `serve.auth.*` secrets. Recommended: treat any config holding a secret field as a key file.
3. Add the test the verify names.

## Done when

- [ ] Bash commands naming key files are refused on every provider.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `df47167f0` by reading the code: roko-std's bash handler ran any command, and `SafetyLayer::check_pre_execution` passed `command` only to the bash denylist and the git policy, neither of which names a key file, so `cat .roko/.env` ran. The Claude guard also let `cat .ro""ko/.e''nv` through (it read the raw text only).
- `roko_std::tool::builtin::sandbox::refuse_key_file_in_command` applies the guard's rules to a command line, dequoted and with nested `sh -c` strings, and resolves each word (and `--opt=` value) against the worktree with symlinks followed. roko-std's bash handler runs it itself; SafetyLayer runs it for `bash` and `run_tests` before the bash policy, so no allowlist prefix admits a key file. The guard's Bash check now reads the dequoted words too, resolves words against the hook's cwd, and no longer flags a plan-worktree path next to a project `.env`.
- Decision: `~/.roko/config.toml` is a key file whether or not it holds a secret yet (`KEY_FILE_NAMES` gains `config.toml`; the Claude deny rules follow). It can hold `serve.auth.api_key` and provider `extra_headers`, and a content check could not be repeated in the Claude permission rules or in the guard under python 3.9 (no TOML parser). This reverses find-570af2's trade-off note. Not covered: the project `roko.toml` (it can hold the same keys and stays readable) and the legacy `~/.config/roko/config.toml`.
- Left as is: `SafetyLayer::check_exec_command` (ExecAgent's operator-configured launch) keeps only the bash policy. bug-0d9ac4 (open) also anchors roko-std's `bash.rs`; this change adds three lines there.
- Implemented on `work/bug-62e7e6` at `9a9a2ba5e`; cargo verification deferred to the batch check.
