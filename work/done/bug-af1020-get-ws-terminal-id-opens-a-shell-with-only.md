+++
id = "bug-af1020"
kind = "bug"
title = "GET /ws/terminal/{id} opens a shell with only the read scope and no RBAC check"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware", "roko-serve/terminal"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/middleware.rs::required_scope_for", "crates/roko-serve/src/terminal.rs::ws_terminal"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add", "bug-88aead"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn terminal_websocket_requires_terminal_write' crates/roko-serve/src/ && cargo test -p roko-serve --lib terminal_websocket_requires_terminal_write"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1c7442b58. GET /ws/terminal requires terminal:write plus an agent:spawn RBAC row. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
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

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-af1020` at `fcdfec7bf`; cargo verification deferred to the batch check. `route_permissions::opens_interactive_session` names the routes whose GET opens a shell (`/ws/terminal`). For those, `required_scope_for` returns the table scope (`terminal:write`), `require_scope` checks it, and `required_permission_for` requires `agent:spawn` through a new `/ws/terminal` RBAC row. Other GETs are unchanged. Test `routes::tests::terminal_websocket_requires_terminal_write`: a read key gets 403 `insufficient_scope`, a `terminal:write` key clears both layers, and no shell starts. The portal's session cookie carries `admin`, so it still opens terminals. Left as found: `check_cookie_same_origin` skips GETs, so a WebSocket upgrade with a cookie is not Origin-checked; the cookie is `SameSite=Strict`, which keeps cross-site pages from sending it.
