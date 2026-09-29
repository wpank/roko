+++
id = "bug-7de5df"
kind = "bug"
title = "The agent git guard misses reset, stash and clean, and commands after the first in a chain"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_agent"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e3"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (main-checkout hazards, gate G2); tldr/research/B4-gates-qa-safety.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::build_settings_json"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_denies_destructive_git_anywhere_in_a_command' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_denies_destructive_git_anywhere_in_a_command"

[[verify]]
command = "! grep -q 'command -v python3 >/dev/null 2>&1 || exit 0' crates/roko-agent/src/claude_cli_agent.rs && grep -rqw 'fn settings_hook_fails_closed_without_python3' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_fails_closed_without_python3"
+++
## Problem

The only guard on a Claude CLI agent's shell commands is the PreToolUse hook that `build_settings_json` puts in the
`--settings` payload, a Python regex table (`claude_cli_agent.rs:57-65`). It has four gaps:

- It denies `git checkout`, `switch`, `branch -m` and `push` only at the start of a command, so
  `cd x && git checkout main` runs, and so does `git -C . checkout`.
- It has no rule for `git reset --hard`, `stash`, `clean` or `restore`.
- It fails open. With no `python3`, with a payload that isn't JSON, or with a command that isn't a string, it
  exits 0 and the command runs.

## Why it matters

Agents run in the operator's checkout by default, with `dangerously_skip_permissions: true`
(`claude_cli_agent.rs:128`). One `git stash` loses the operator's uncommitted work (research note B4). This is
tldr/05 P0 #3 and gate G2 in W8. It is part of epic spec-ba7bea, whose test C2 (gap-0e2c40) checks these denials.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::build_settings_json`: the hook script.
- Its tests in the same file use the helper `run_hook_command`.

## Current state

Unchanged at `41c7ffbd6`. Roko's own bash tool uses a separate Rust policy, `safety/git.rs`, which already splits
chains and strips prefixes.

## Plan

1. Split the command on `;`, `&&`, `||`, `|`, `&` and newlines. Strip `cd …`, `env VAR=…` and `sudo` prefixes, and
   git's global options (`-C`, `-c`, `--git-dir`, `--work-tree`), as `safety/git.rs` does.
2. Deny these in any segment:
   - `checkout`, `switch`, `restore` and `push`;
   - `branch -m`, `-M` and `-D`;
   - `reset --hard`;
   - `stash`, except `list` and `show`;
   - `clean -f`.
3. Fail closed: every error path exits 2 with a `BLOCKED:` message. Claude Code treats any other non-zero exit as a
   non-blocking error.

## Done when

- [ ] The hook denies `cd x && git checkout main`, `git -C . stash`, `git reset --hard`, `git clean -fdx` and
      `git restore .`.
- [ ] `git status`, `git stash list` and `echo ok` still pass.
- [ ] With no `python3` on `PATH`, the hook exits 2.
- [ ] Tests `settings_hook_denies_destructive_git_anywhere_in_a_command` and
      `settings_hook_fails_closed_without_python3` pass: both `[[verify]]` commands.

## Notes

- bug-a66941 edits the same function, so do it after this item. bug-7d7200's uncommitted work in `../roko-wt-env`
  edits this file but not this function.
- Codex, Cursor and Gemini CLI agents get no hook at all. That is out of scope here.
- Implemented on `work/bug-7de5df` at `22cd8c88b`; cargo verification deferred to the batch check.
