+++
id = "bug-77413c"
kind = "bug"
title = "roko-std's refuse_key_file_in_command has none of the guard's search checks (grep -r, rg, git grep, ag/ack, find/xargs reads, brace globs, cd)"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-std/sandbox"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bf40f3269"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/bug-7830f5 at c537d9739)"
anchors = ["crates/roko-std/src/tool/builtin/sandbox.rs", "crates/roko-std/src/tool/builtin/bash.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-41bea4", "bug-69a002", "gap-e9660f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bash_refuses_every_search_that_reaches_a_secret' crates/roko-std/src/ && cargo test -p roko-std --lib bash_refuses_every_search_that_reaches_a_secret"

[closed]
at = 2026-10-01
by = "coordinator (session 7622b882)"
evidence = "Batch 20b gate on cad1a56e1 (MAIN bf40f3269 has the same crates): check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests roko-agent 2278, roko-cli 3261, roko-core 1956, roko-learn 1207, roko-gate 690, roko-std 227 and roko-cli bin 429 all pass, including roko-std bash_refuses_every_search_that_reaches_a_secret (105-case table) and roko-agent settings_hook_refuses_every_search_that_reaches_a_secret. Merged bf40f3269 (work/bug-77413c 49acd1711 + rustfmt a2a25e606)."
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

## Notes

- Premise held at 286c5e53a: `refuse_key_file_in_command`, which roko-std's `bash` tool, roko-agent's SafetyLayer and roko-acp run, refused only words that named a key file or a secret-holding config.
- Decision: port the guard's rules to a `sandbox::reads` module rather than share code (the guard is a Python hook), and keep one rule set as a table, `sandbox/secret_read_cases.txt` (105 commands, each denied or allowed, all allowed without the secret), which both `bash_refuses_every_search_that_reaches_a_secret` and roko-agent's `settings_hook_refuses_every_search_that_reaches_a_secret` read. `parse_shell_words` now uses the module's tokenizer, and glob.rs's `segment_match` is shared.
- roko-std gains the `regex` crate, a workspace dependency already built for roko-core, for `find -regex`, fd patterns and `ag -G`; `Cargo.lock` gains that one edge, added by hand.
- Differences from the guard: the Rust check resolves no git alias (the guard asks git), and at too deep a nesting of command lines it stops rather than refusing.
- Found, in both implementations: a recursive search reads `.roko` key files. `grep -r OPENAI .` in a workspace with `.roko/.env` passes, since the search rules consider only roko config files.
- Checked without cargo: the real `sandbox.rs` and `reads.rs`, compiled standalone against the workspace's regex and serde_json builds with a stand-in for the roko-core items they use, pass the table and the existing key-file cases, and lint clean with the workspace's clippy settings; the table passes through the Python guard under python 3.12 and 3.9.
- Implemented on `work/bug-77413c` at `421346ab2`; cargo verification deferred to the batch check.
