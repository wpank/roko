+++
id = "bug-88aead"
kind = "bug"
title = "The terminal id is joined unvalidated into .roko/workspaces/{id}/terminal.state, so an encoded / escapes the workspace"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-serve/terminal"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/terminal.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add", "bug-af1020"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn terminal_ids_cannot_escape_the_workspaces_dir' crates/roko-serve/src/ && cargo test -p roko-serve --lib terminal_ids_cannot_escape_the_workspaces_dir"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1c7442b58. Terminal ids must be 1-128 of [A-Za-z0-9_-] at every entry point; state files stay under .roko/workspaces/. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
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
- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-af1020` at `51fb45395`; cargo verification deferred to the batch check. A session id must be 1-128 ASCII letters, digits, `-` or `_` (`terminal::is_valid_session_id`), which covers every id the portal and the REST create route produce. `/ws/terminal/{id}`, `DELETE /api/terminal/sessions/{id}` and `POST .../{id}/input` answer any other id with 400. The WebSocket handler takes the upgrade as a `Result`, so it checks the id before the upgrade. In `SessionManager`, `attach_session` and `create_session_with_id` refuse invalid ids, and one `state_file_path` helper validates the id and checks the joined path stays under `.roko/workspaces/`. Test: `terminal::tests::terminal_ids_cannot_escape_the_workspaces_dir`.
