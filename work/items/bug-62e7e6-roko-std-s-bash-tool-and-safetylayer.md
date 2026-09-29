+++
id = "bug-62e7e6"
kind = "bug"
title = "roko-std's bash tool and SafetyLayer's bash policy never check commands for key files"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:55, wk-guard2's report on bug-f4e133, bug-66f5a1, bug-63327d and find-570af2)"
anchors = ["crates/roko-std/src/tool/builtin/bash.rs", "crates/roko-agent/src/safety/mod.rs", "crates/roko-core/src/child_env.rs::is_key_file"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-63327d", "bug-a66941", "find-570af2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bash_commands_naming_key_files_are_refused' crates/roko-std/src/ crates/roko-agent/src/ && cargo test -p roko-agent --lib bash_commands_naming_key_files_are_refused"
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
