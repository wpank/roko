+++
id = "bug-44320f"
kind = "bug"
title = "required_scope_for matches prefixes without segment boundaries, so POST /api/relay-tokens gets the /relay scope"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/middleware.rs::required_scope_for"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scope_prefixes_match_whole_path_segments' crates/roko-serve/src/ && cargo test -p roko-serve --lib scope_prefixes_match_whole_path_segments"
+++

## Problem

`required_scope_for` (`routes/middleware.rs:1284`) picks a mutating route's scope with `path.starts_with(entry.prefix)`, with no segment boundary. A prefix entry for `/relay` (or `/api/relay`) therefore also matches `/api/relay-tokens`, and `POST /api/relay-tokens` gets the relay scope (`agent:write`) instead of its own.

## Why it matters

Release blockers: a scope table that matches by string prefix can grant or require the wrong scope for any route whose name extends another's. p2.

## Where

The prefix matching in `required_scope_for`.

## Plan

1. Match a prefix only at a segment boundary: the path equals the prefix, or continues with `/`.
2. Give `/api/relay-tokens` its own entry.
3. Add `scope_prefixes_match_whole_path_segments`.

## Done when

- [ ] Every route gets the scope of its own table entry.
- [ ] The `[[verify]]` command passes.
