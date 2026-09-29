+++
id = "bug-e1327f"
kind = "bug"
title = "roko init without claude on PATH writes a roko.toml that fails every command with config invariant 3"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["cli", "config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "98a77c510"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:15, wk-readme's quick-start check for bug-09690f; details in bug-09690f's Notes)"
anchors = ["crates/roko-cli/src/commands/init.rs", "crates/roko-core/src/config/validation.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-09690f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_without_claude_cli_writes_a_loadable_config' crates/roko-cli/src/ && cargo test -p roko-cli --lib init_without_claude_cli_writes_a_loadable_config"
+++

## Problem

When `claude` is not on `PATH`, `roko init` comments out the `claude_cli` provider but leaves `[models.claude-sonnet-4-6]` pointing at it. Every command that loads the config then fails with `config invariant 3 violated`, even with `ANTHROPIC_API_KEY` exported, which is what init itself advises.

## Why it matters

A new user's first commands fail, and the README's quick start depends on them. Epic spec-ae5f94.

## Where

`crates/roko-cli/src/commands/init.rs` (the provider and model blocks it writes) and the invariant in `crates/roko-core/src/config/validation.rs`.

## Current state

Reproduced on 2026-09-29 with `target/debug/roko` built at `33e107da1`, in scratch directories with an empty `HOME` and no API keys (wk-readme).

## Plan

1. When the CLI is missing, point the default model at a provider that exists: the Anthropic API provider when `ANTHROPIC_API_KEY` is set. Otherwise comment out the model entry too.
2. Add a test that runs init with no `claude` on `PATH` and loads the result.

## Done when

- [ ] `roko init` without `claude` writes a config that every command can load.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `98a77c510` with `target/debug/roko` built at `33e107da1` (no config-code changes in between): without `claude` on `PATH`, `roko init` leaves `[models.claude-sonnet-4-6]` on the commented-out `claude_cli` provider and `roko config show` fails with config invariant 3.
- init now picks Claude CLI, else the Anthropic API when `ANTHROPIC_API_KEY` is set, else writes the model block commented out with its provider. The template passes `config_cmd::write_checked_config` (the offline checks of `roko config validate`) before it is written. Layout touch-ups followed in `3bf3af074`.
- Implemented on `work/bug-e1327f` at `435893094`; cargo verification deferred to the batch check.
