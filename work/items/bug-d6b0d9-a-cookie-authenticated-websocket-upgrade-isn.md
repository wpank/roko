+++
id = "bug-d6b0d9"
kind = "bug"
title = "A cookie-authenticated WebSocket upgrade isn't Origin-checked, because check_cookie_same_origin skips GETs"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report)"
anchors = ["crates/roko-serve/src/routes/middleware.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-af1020", "bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cookie_websocket_upgrades_are_origin_checked' crates/roko-serve/src/ && cargo test -p roko-serve --lib cookie_websocket_upgrades_are_origin_checked"
+++

## Problem

`check_cookie_same_origin` (`crates/roko-serve/src/routes/middleware.rs:635`) returns early for GET, HEAD and OPTIONS (:636). A WebSocket upgrade is a GET, so a cookie-authenticated upgrade from another origin isn't Origin-checked, which is cross-site WebSocket hijacking. The session cookie's `SameSite=Strict` prevents it today.

## Why it matters

Release: the protection rests on one cookie attribute. If that attribute is relaxed (for example to `Lax` for OAuth flows), the terminal and event WebSockets become hijackable. p3.

## Where

`check_cookie_same_origin`.

## Plan

1. Check the `Origin` header on cookie-authenticated WebSocket upgrades (GET with `Upgrade: websocket`).
2. Add `cookie_websocket_upgrades_are_origin_checked`.

## Done when

- [ ] A cross-origin cookie upgrade is refused whatever the cookie's SameSite setting.
- [ ] The `[[verify]]` command passes.
