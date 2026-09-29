+++
id = "bug-0bc728"
kind = "bug"
title = "The command guard misses command strings passed to wrappers, find -exec and -delete, and busybox rm"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
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

## Notes

- Premise confirmed at `df47167f0` through the exact hook command: `watch 'rm -rf x'`, `flock /tmp/l -c 'rm -rf x'`, `find . -exec rm -rf {} +`, `find . -delete` and `busybox rm -rf x` exited 0, and `sudo -u git whoami` exited 2.
- The wrapper scan is `check_wrapped` now: every argument of `watch` and `sg`, and the value of `-c`, `--command`, `-S` and `--split-string` (flock, su, runuser, script, env), is checked as a command line; su, runuser, script and sg are wrappers. `find -delete` is denied, the command a `find -exec`/`-execdir`/`-ok`/`-okdir` runs is checked, and any `rm` in it is denied however it is reached (`sudo rm`, `sh -c 'rm "$1"'`). busybox and toybox are read as their applet. The value of a user or group option (`-u`, `-g`, `-U`, `--user`, `--group`) is never taken for the wrapped program.
- Not covered (report): `find ... | xargs rm` (non-recursive rm of a whole tree), `fd -x rm`, and wrappers not in the list that take a command string (`ssh host 'cmd'`, `parallel`). `sudo -s 'cmd'` passes a single escaped word, so it is not a bypass.
- Implemented on `work/bug-62e7e6` at `d0111b562`; cargo verification deferred to the batch check.
