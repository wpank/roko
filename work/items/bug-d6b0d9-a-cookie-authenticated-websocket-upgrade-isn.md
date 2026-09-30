+++
id = "bug-d6b0d9"
kind = "bug"
title = "A cookie-authenticated WebSocket upgrade isn't Origin-checked, because check_cookie_same_origin skips GETs"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report)"
anchors = ["crates/roko-serve/src/routes/middleware.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-af1020", "bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cookie_websocket_upgrades_are_origin_checked' crates/roko-serve/src/ && cargo test -p roko-serve --lib cookie_websocket_upgrades_are_origin_checked"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 6da901d82. Cookie-authenticated WebSocket upgrades are Origin-checked like mutations. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
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

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-d6b0d9` at `39f71b550`; cargo verification deferred to the batch check. `check_cookie_same_origin` now Origin-checks a GET that asks to upgrade to WebSocket (`is_websocket_upgrade`: `Upgrade: websocket`, any case, anywhere in a protocol list), the same way it checks mutations: a different or `"null"` Origin gets 403, while the same origin or no Origin passes. Test: `middleware::tests::cookie_websocket_upgrades_are_origin_checked`. Side effect: a dev frontend on another port (Vite :5173, Next :3000) that proxies a WebSocket with a session cookie is now refused, as its cookie mutations already are (gap-eb4a65). The same-origin portal and API-key clients are unaffected.
