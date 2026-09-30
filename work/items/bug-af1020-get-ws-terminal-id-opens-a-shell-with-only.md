+++
id = "bug-af1020"
kind = "bug"
title = "GET /ws/terminal/{id} opens a shell with only the read scope and no RBAC check"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware", "roko-serve/terminal"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/middleware.rs::required_scope_for", "crates/roko-serve/src/terminal.rs::ws_terminal"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add", "bug-88aead"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn terminal_websocket_requires_terminal_write' crates/roko-serve/src/ && cargo test -p roko-serve --lib terminal_websocket_requires_terminal_write"
+++

## Problem

The scope table in `crates/roko-serve/src/routes/middleware.rs` maps `/ws/terminal` to `terminal:write` (around :1136), but `required_scope_for` returns `"read"` for every GET, HEAD and OPTIONS request before it looks at the table (:1284-1287). A WebSocket upgrade is a GET, so `GET /ws/terminal/{id}` (`terminal.rs::ws_terminal`, :1210) opens an interactive shell for any token with the `read` scope. wk-serve-sec also found no RBAC (permission-row) check on the route.

## Why it matters

Release blocker, p1: a read-only token becomes a shell on the host.

## Where

`required_scope_for` and the scope table in `middleware.rs`, and the terminal routes in `terminal.rs`.

## Plan

1. Classify WebSocket routes by their table entry, not by the HTTP method. At least `/ws/terminal` must require `terminal:write`, even on GET.
2. Run the RBAC permission check on the terminal routes.
3. Add `terminal_websocket_requires_terminal_write`: a read-scoped token gets 403 on the upgrade.

## Done when

- [ ] Opening a terminal needs `terminal:write` and passes RBAC.
- [ ] The `[[verify]]` command passes.
