+++
id = "gap-4119fb"
kind = "gap"
title = "PK83 Showcase and deploy: Browser sessions carry a scope, absolute and idle TTLs, and a passphrase generation (+5 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
rank = 83
size = "L"
subsystem = ["roko-serve/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK83"
anchors = ["crates/roko-core/src/config/loader.rs", "crates/roko-core/src/config/serve.rs", "crates/roko-serve/src/auth_audit.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/routes/auth_session.rs", "crates/roko-serve/src/routes/middleware.rs", "crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/state.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-9ecd37"], blocks = [], related = ["find-c8527b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rotating_the_passphrase_hash_revokes_sessions' crates/roko-serve/src/ && grep -rqw 'fn session_scope_comes_from_the_session_record' crates/roko-serve/src/ && cargo test -p roko-serve --lib rotating_the_passphrase_hash_revokes_sessions && cargo test -p roko-serve --lib session_scope_comes_from_the_session_record"

[[verify]]
command = "grep -rqw 'fn passphrase_login_sets_a_showcase_session' crates/roko-serve/src/ && grep -rqw 'fn passphrase_login_requires_csrf_and_origin' crates/roko-serve/src/ && cargo test -p roko-serve --lib passphrase_login_"

[[verify]]
command = "test -f crates/roko-serve/src/routes/showcase/bundles.rs && grep -rqw 'fn showcase_bundle_checksum_mismatch_is_409' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_bundle_"

[[verify]]
command = "grep -rqw 'fn login_lockout_sixth_failure_from_one_ip_is_429' crates/roko-serve/src/ && cargo test -p roko-serve --lib login_lockout_"

[[verify]]
command = "grep -rqw 'fn showcase_scope_session_gets_403_on_config_and_secrets' crates/roko-serve/src/ && grep -rqw 'fn showcase_scope_rejects_privy_and_agent_tokens' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_scope_"

[[verify]]
command = "grep -q 'public_routes' crates/roko-core/src/config/serve.rs && grep -rqw 'fn showcase_router_mounts_no_public_extras' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_router_mounts_no_public_extras"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK83, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9322 | M | p2 | Browser sessions carry a scope, absolute and idle TTLs, and a passphrase generation | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9322-sessions-carry-scope-ttls-and-a-generation.md` |
| 2 | 9323 | M | p2 | Passphrase login on `/api/auth/session`: Argon2id behind a semaphore, the CSRF header, an exact Origin, and a session probe | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9323-passphrase-login-with-csrf-origin-and-probe.md` |
| 3 | 9324 | M | p3 | Serve the showcase read routes from verified bundles, with a checksum loader and an admin reload (R1-serve) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9324-serve-showcase-read-routes-with-checksum-loader.md` |
| 4 | 9325 | M | p2 | Login lockout: per-IP and global failure limits with `Retry-After`, `Fly-Client-IP` trust, and an admin unlock | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9325-login-lockout-per-ip-and-global-with-unlock.md` |
| 5 | 9326 | M | p2 | Showcase sessions reach only the showcase routes, and showcase mode refuses every other credential | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9326-showcase-sessions-reach-only-the-showcase-routes.md` |
| 6 | 9327 | M | p2 | `serve.public_routes`: make the unauthenticated routes an allowlist, so showcase mode mounts only health and ready (G1) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9327-serve-public-routes-allowlist-g1.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-core/src/config/loader.rs`, `crates/roko-core/src/config/serve.rs`, `crates/roko-serve/src/auth_audit.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/auth_session.rs`, `crates/roko-serve/src/routes/middleware.rs`, `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/routes/showcase/admin.rs`, `crates/roko-serve/src/routes/showcase/bundles.rs`, `crates/roko-serve/src/routes/showcase/mod.rs`, `crates/roko-serve/src/routes/showcase/views.rs`, `crates/roko-serve/src/showcase/auth.rs`, `crates/roko-serve/src/showcase/lockout.rs`, `crates/roko-serve/src/showcase/mod.rs`, `crates/roko-serve/src/showcase/scope.rs`, `crates/roko-serve/src/state.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK82 (gap-9ecd37).
- Existing work items this package covers or touches: find-c8527b. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
