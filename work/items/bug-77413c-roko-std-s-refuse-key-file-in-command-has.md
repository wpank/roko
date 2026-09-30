+++
id = "bug-77413c"
kind = "bug"
title = "roko-std's refuse_key_file_in_command has none of the guard's search checks (grep -r, rg, git grep, ag/ack, find/xargs reads, brace globs, cd)"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-std/sandbox"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/bug-7830f5 at c537d9739)"
anchors = ["crates/roko-std/src/tool/builtin/sandbox.rs", "crates/roko-std/src/tool/builtin/bash.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-41bea4", "bug-69a002", "gap-e9660f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bash_refuses_every_search_that_reaches_a_secret' crates/roko-std/src/ && cargo test -p roko-std --lib bash_refuses_every_search_that_reaches_a_secret"
+++

## Problem

The Claude CLI guard (`claude_cli_guard.py`) gained checks that refuse searches reaching a secret-holding `roko.toml` or key file: `grep -r` and `parallel` (bug-41bea4), and `git grep`, `ag`/`ack`, reads through `find`/`xargs`, brace globs and `cd` (bug-69a002, on guard2's branch). roko-std's bash tool refuses key files through `refuse_key_file_in_command` (`crates/roko-std/src/tool/builtin/sandbox.rs`, called from `bash.rs`), and it has none of those checks. guard2's branch changes only the Python guard, so agents that run commands through roko-std, rather than through Claude Code, still read secrets with any of these forms.

## Why it matters

Secrets and guard (epic spec-ba7bea): the same command is refused in one runtime and allowed in the other.

## Where

`refuse_key_file_in_command` in roko-std's `sandbox.rs`.

## Plan

1. Port the guard's search rules to `refuse_key_file_in_command`, or share one rule set (for example a table both implementations test against).
2. Add `bash_refuses_every_search_that_reaches_a_secret`, with the same cases as the guard's tests.

## Done when

- [ ] roko-std's bash tool refuses every form the Claude guard refuses.
- [ ] The `[[verify]]` command passes.
