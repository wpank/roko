+++
id = "bug-69a002"
kind = "bug"
title = "The guard's secret-read check misses git grep, ag/ack, reads through find or xargs and brace globs, and judges a search after cd from the wrong directory"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard", "roko-std/sandbox"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py", "crates/roko-std/src/tool/builtin/sandbox.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-e9660f", "bug-41bea4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_refuses_every_search_that_reaches_a_secret' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_refuses_every_search_that_reaches_a_secret"
+++

## Problem

The check that refuses agents a secret-holding `roko.toml` or key file still misses:

- `git grep`, and the `ag` and `ack` searchers;
- reads through `find … -exec cat` or `xargs cat`;
- brace globs (`cat roko.{toml,lock}`).

It also judges a search that follows a `cd` in the same command from the call's directory, not the `cd` target (reported by wk-guard2).

## Why it matters

Secrets and guard (epic spec-ba7bea): each is an easy way to read a secret the check exists to protect. bug-41bea4 covers `grep -r` and `parallel`. p3.

## Where

The content check in `claude_cli_guard.py`, and `refuse_key_file_in_command` in roko-std's `sandbox.rs`.

## Plan

1. Treat `git grep`, `ag`, `ack`, and reads fed by `find` or `xargs`, as searches. Expand brace globs before matching. Track `cd` within a command line.
2. Add `settings_hook_refuses_every_search_that_reaches_a_secret`.

## Done when

- [ ] Each form is refused when it would read a secret.
- [ ] The `[[verify]]` command passes.
