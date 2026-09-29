+++
id = "bug-0bc728"
kind = "bug"
title = "The command guard misses command strings passed to wrappers, find -exec and -delete, and busybox rm"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:55, wk-guard2's report on bug-f4e133, bug-66f5a1, bug-63327d and find-570af2)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-f4e133", "bug-7de5df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_denies_destructive_commands_behind_wrappers' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_denies_destructive_commands_behind_wrappers"
+++

## Problem

After bug-f4e133, the guard parses `rm` through sudo, subshells, `sh -c`, `eval` and `$()`. It still misses command strings handed to other wrappers (`watch 'rm -rf x'`, `flock -c 'rm -rf x'`), `find -exec rm -rf {} +` and `find -delete`, and `busybox rm`. `sudo -u git <cmd>` is a known false positive (wk-guard2, 2026-09-29).

## Why it matters

These are the obvious next bypasses of the only destructive-command barrier. Epic spec-ba7bea.

## Where

`crates/roko-agent/src/claude_cli_guard.py`.

## Current state

The forms above pass.

## Plan

1. Treat `watch`, `flock -c`, `timeout`, `nice`, `nohup` and `xargs` as wrappers whose argument is a command, and parse it.
2. Deny `find` with `-delete`, or with `-exec`/`-execdir` running `rm`.
3. Treat `busybox <applet>` as `<applet>`.
4. Fix the `sudo -u git` false positive.
5. Add the test the verify names.

## Done when

- [ ] Each form above is denied and the false positive is gone.
- [ ] The `[[verify]]` command passes.
