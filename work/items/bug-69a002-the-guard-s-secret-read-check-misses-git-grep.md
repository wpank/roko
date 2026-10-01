+++
id = "bug-69a002"
kind = "bug"
title = "The guard's secret-read check misses git grep, ag/ack, reads through find or xargs and brace globs, and judges a search after cd from the wrong directory"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/claude_cli_guard", "roko-std/sandbox"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py", "crates/roko-std/src/tool/builtin/sandbox.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-e9660f", "bug-41bea4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn settings_hook_refuses_every_search_that_reaches_a_secret' crates/roko-agent/src/ && cargo test -p roko-agent --lib settings_hook_refuses_every_search_that_reaches_a_secret"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 0b4860627. The guard judges git grep, ag/ack, reads of find/fd/xargs lists, brace expansion and cd targets. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
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

## Notes

- Premise held at 4cf2e329b: `git grep`, `ag`, `ack`, `find . -exec cat {} +`, `ls | xargs cat` and `cat roko.{toml,lock}` all passed over a secret-holding `roko.toml`, and `cd src && grep -r x ..` was judged from the call's directory.
- Changes: `git grep` (after `-C`, over its paths and pathspecs), `ag` and `ack` count as recursive searches with their own option tables and filters. A read (cat, head, grep, cp and the like) of a list the guard cannot see is judged by where the list comes from: find's starting points or fd's paths, else the call's directory for `xargs` and `parallel`, unless a find name test or an fd pattern or `-e` leaves `roko.toml` out. Brace expansions are expanded before a word is judged. A command after a literal `cd`/`pushd` is judged where it runs, and a word is resolved against each `cd` target as well; `cd -` and `cd $X` stay unknown.
- Not done: roko-std's `refuse_key_file_in_command` (the `bash` tool) has none of these checks, nor bug-41bea4's; porting them to Rust is left for a follow-up item.
- The guard's scratch suites, old and new (reach cases and a mirror of `settings_hook_refuses_every_search_that_reaches_a_secret`), pass under python 3.12 and 3.9, and a fuzz of odd commands finds no crash. Implemented on `work/bug-7830f5` at `2a211b515`; cargo verification deferred to the batch check.
