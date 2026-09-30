+++
id = "bug-88aead"
kind = "bug"
title = "The terminal id is joined unvalidated into .roko/workspaces/{id}/terminal.state, so an encoded / escapes the workspace"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-serve/terminal"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/terminal.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add", "bug-af1020"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn terminal_ids_cannot_escape_the_workspaces_dir' crates/roko-serve/src/ && cargo test -p roko-serve --lib terminal_ids_cannot_escape_the_workspaces_dir"
+++

## Problem

`crates/roko-serve/src/terminal.rs` builds the terminal state path as `self.workdir.join(".roko/workspaces").join(id)` and then `terminal.state` (:540-573). `id` is the path parameter of the terminal routes, decoded and never validated. A percent-encoded `/` (`%2F`) with `..` segments makes the path point outside `.roko/workspaces`, so terminal state is read or written wherever the id points.

## Why it matters

Release blocker, p1: a path traversal on a route that also opens a shell (bug-af1020).

## Where

The id handling in `terminal.rs` (:116, :540-573).

## Plan

1. Validate the id against the workspace-id format (for example `[A-Za-z0-9_-]{1,64}`), reject anything else with 400, and check that the joined path stays under `.roko/workspaces`.
2. Add `terminal_ids_cannot_escape_the_workspaces_dir`.

## Done when

- [ ] No terminal id can reach a path outside `.roko/workspaces/`.
- [ ] The `[[verify]]` command passes.

## Notes

- Fix it together with bug-af1020: both are in the terminal routes.
