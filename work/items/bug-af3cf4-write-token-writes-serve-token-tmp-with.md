+++
id = "bug-af3cf4"
kind = "bug"
title = "write_token writes serve.token.tmp with default permissions before chmodding it to 0600"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/endpoint"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/endpoint.rs::write_token"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn write_token_never_leaves_a_readable_temp_file' crates/roko-serve/src/ && cargo test -p roko-serve --lib write_token_never_leaves_a_readable_temp_file"
+++

## Problem

`endpoint.rs::write_token` (:113) writes the launch token to a sibling temp file with `std::fs::write(&tmp, token)` (:121). That creates the file with the process's default permissions, often 0644, and only then sets 0600 (:127) and renames it. Until then, other local users can read the token.

## Why it matters

Release blockers: the token authenticates to serve. The window is short, but it's a real exposure on shared machines. p2.

## Where

`write_token`.

## Plan

1. Create the temp file with mode 0600 from the start (`OpenOptions` with `.mode(0o600)` and `create_new(true)`), write, fsync, then rename.
2. Add `write_token_never_leaves_a_readable_temp_file`.

## Done when

- [ ] The token file is never readable by others, at any point.
- [ ] The `[[verify]]` command passes.
