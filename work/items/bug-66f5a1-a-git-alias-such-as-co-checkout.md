+++
id = "bug-66f5a1"
kind = "bug"
title = "A git alias such as co = checkout bypasses the agent git guard"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-guard's report on bug-7de5df)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-7de5df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_denies_git_aliases_to_denied_commands' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_denies_git_aliases_to_denied_commands"
+++

## Problem

The git guard matches git subcommand names, so an alias passes: with `alias.co = checkout` in the user's or the repo's git config, `git co main` is allowed. wk-guard found this on 2026-09-29.

## Why it matters

An agent can define a repo-local alias itself. Epic spec-ba7bea.

## Where

`crates/roko-agent/src/claude_cli_guard.py`.

## Current state

Aliases are not resolved.

## Plan

1. Before matching, resolve a subcommand that isn't a built-in git command. Run `git config --get alias.<name>` in the command's working directory, and treat `!`-aliases as shell.
2. Deny, failing closed, when resolution fails, or when an alias resolves to a denied command.
3. Test with a repo-local alias that resolves to a denied command.

## Done when

- [ ] A git alias that resolves to a denied command is denied.
- [ ] The `[[verify]]` command passes.
