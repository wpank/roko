+++
id = "bug-f4e133"
kind = "bug"
title = "The agent command guard lets recursive rm through under sudo, -R, subshells and sh -c"
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
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-guard's report on bug-7de5df)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py", "crates/roko-agent/src/claude_cli_agent.rs::build_settings_json"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-7de5df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_denies_recursive_rm_in_any_form' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_denies_recursive_rm_in_any_form"
+++

## Problem

bug-7de5df rewrote the git half of the agent command guard. It now splits chains quote-aware, strips wrappers, and follows `sh -c`, `eval`, `$()` and backquotes. The recursive-rm rules are unchanged.

wk-guard ran these through the exact hook commands on 2026-09-29, and all of them pass:

- `sudo rm -rf x`
- `rm -R x`
- `(rm -rf x)`
- `bash -c "rm -rf x"`

## Why it matters

The guard is the only barrier between an agent and destructive commands; there is no OS sandbox (gap-8f8544, on hold). Epic spec-ba7bea.

## Where

`crates/roko-agent/src/claude_cli_guard.py`, which `claude_cli_agent.rs` embeds with `include_str!`.

## Current state

The git rules use the new parser, but the rm rules still match raw text.

## Plan

1. Run the rm rules through the same normalisation as the git rules: wrappers, subshells, `sh -c`, `eval` and substitutions. Every form the rules deny directly must also be denied when wrapped.
2. Treat `-R`, `-r` and `--recursive` alike, and fail closed on parse errors.
3. Add `settings_hook_denies_recursive_rm_in_any_form`, covering each form above.

## Done when

- [ ] Every form listed above is denied.
- [ ] Commands the rules allowed before are still allowed.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `407ce30d5` through the exact hook command: `sudo rm -rf x`, `rm -R x`, `(rm -rf x)`, `bash -c "rm -rf x"`, `rm --recursive x`, `rm -f -r x`, `/bin/rm -rf x` and `rm -rf x` on a second line all exited 0.
- `check_rm` now runs on the parsed commands, like the git rules: `-r`, `-R` or `--recursive` (or a prefix such as `--rec`) anywhere before `--` denies, and so does an rm option built from a variable or the positional parameters. A wrapper now checks every later word that names a checked program, so `sudo -u git rm -rf x` is denied. `rm -rf` inside a quoted string, such as a commit message, is no longer a false positive.
- Not covered (report): a command string handed to a wrapper (`watch 'rm -rf x'`, `flock l -c 'rm -rf x'`), `find -exec rm -rf {} +`, `find -delete` and `busybox rm`.
- Implemented on `work/bug-f4e133` at `976496b27`; cargo verification deferred to the batch check.
