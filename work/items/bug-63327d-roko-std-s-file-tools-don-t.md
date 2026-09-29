+++
id = "bug-63327d"
kind = "bug"
title = "roko-std's file tools don't check key files, so a dispatch without SafetyLayer can read .roko/.env"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-guard's report on bug-a66941)"
anchors = ["crates/roko-std/src/tool/builtin/read_file.rs", "crates/roko-core/src/child_env.rs::is_key_file"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn std_file_tools_refuse_key_files' crates/roko-std/src/ && cargo test -p roko-std --lib std_file_tools_refuse_key_files"
+++

## Problem

bug-a66941 made `SafetyLayer`'s path policy refuse provider key files, through `roko_core::child_env::is_key_file` and `ToolError::KeyFileBlocked`. roko-std's builtin file tools don't call that check themselves. Any dispatch path that runs them without `SafetyLayer` can still read `.roko/.env` (wk-guard, 2026-09-29).

## Why it matters

The key-file block should hold whichever dispatcher runs the tool. Epic spec-ba7bea.

## Where

`crates/roko-std/src/tool/builtin/` (`read_file.rs` and its siblings that read, list or search files), plus `child_env::is_key_file`.

## Current state

Only `SafetyLayer` checks for key files.

## Plan

1. Check `is_key_file` in every builtin tool that reads file contents. Check both the path as given and the resolved path.
2. Return the same `KeyFileBlocked` error.
3. Add the test `std_file_tools_refuse_key_files`.

## Done when

- [ ] The builtin file tools refuse key files without `SafetyLayer`.
- [ ] The `[[verify]]` command passes.
