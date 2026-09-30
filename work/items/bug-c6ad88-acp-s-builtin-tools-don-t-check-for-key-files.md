+++
id = "bug-c6ad88"
kind = "bug"
title = "ACP's builtin tools don't check for key files"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-acp/builtin_tools"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-acp/src/builtin_tools.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941", "bug-0d9ac4", "bug-34c16c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_builtin_tools_refuse_key_files' crates/roko-acp/src/ && cargo test -p roko-acp --lib acp_builtin_tools_refuse_key_files"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in ca834b178. ACP read/write/edit/grep/bash refuse key files through roko-std's checks. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

bug-a66941 made roko-std's file tools, the bash check, SafetyLayer and the Claude guard refuse key files (`roko_core::child_env::is_key_file`, `ToolError::KeyFileBlocked`). ACP has its own builtin tools (`crates/roko-acp/src/builtin_tools.rs`: `read_file` :93, `write_file` :114, `bash` :188). They read files with `tokio::fs::read_to_string` (:608, :646), and nothing in `crates/roko-acp/src/` calls `is_key_file` or `refuse_key_file`, at d2cc43346 or on `work/bug-ceab60`.

## Why it matters

Secrets and guard (epic spec-ba7bea): an editor session over ACP can read `.roko/.env` and the other key files that every other path refuses. bug-0d9ac4 covers ACP's bash environment, not its file reads.

## Where

`builtin_tools.rs`: the file tools and the bash tool.

## Plan

1. Route ACP's file tools through roko-std's key-file refusal (or call `is_key_file` on the resolved path), and run `refuse_key_file_in_command` on ACP bash commands.
2. Add `acp_builtin_tools_refuse_key_files`.

## Done when

- [ ] ACP's builtin tools refuse key files, like every other tool path.
- [ ] The `[[verify]]` command passes.
