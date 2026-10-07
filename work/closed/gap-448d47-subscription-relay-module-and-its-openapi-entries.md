+++
id = "gap-448d47"
kind = "gap"
title = "subscription_relay module and its OpenAPI entries still compile/register in the default (non-relay) build"
status = "superseded"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "f74890b4b"
source = "wave-6 follow-up reports 2026-10-03 (PK79 gap-425d9e)"
discovered_from = "gap-425d9e"
anchors = ["crates/roko-serve/src/lib.rs::subscription_relay", "crates/roko-serve/src/openapi.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["spec-c89168"], supersedes = [], duplicate_of = "gap-e7a3d4" }

[[verify]]
command = "grep -rqw 'fn default_build_has_no_relay_routes_in_openapi' crates/roko-serve/ && cargo test -p roko-serve default_build_has_no_relay_routes_in_openapi"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:33:25Z"
forced = false
evidence = "Duplicate of gap-e7a3d4, filed independently and concurrently for the same PK79/gap-425d9e report. gap-e7a3d4 has the more complete anchors (AppState field, the unconditional lib.rs wiring block, relay.rs as the correct reference pattern) and is kept."
+++

## Problem

`crates/roko-serve/src/lib.rs`'s `subscription_relay` module and its OpenAPI entries still compile in the default
build, even though the actual relay functionality is feature-gated:

- `mod subscription_relay;` (line 75) carries only `#[cfg_attr(not(feature = "relay"), allow(dead_code))]` — this
  suppresses the dead-code warning when the `relay` feature is off, but does NOT stop the module from being
  compiled; it has no `#[cfg(feature = "relay")]` on the `mod` declaration itself.
- `crates/roko-serve/src/openapi.rs` imports `subscription_relay` types unconditionally (line 28) and registers
  relay paths/schemas in the default build's OpenAPI document with no feature gate: `relay_subscription_status`
  (line 144, 836-844, path `/subscriptions/relay/status`), `issue_relay_token_handler`, `revoke_relay_token`,
  `relay_health` (lines 452-454).

So a default build (no `--features relay`) still carries the relay module's code and advertises relay routes in
its own OpenAPI spec, even though those routes are refused/dead per `lib.rs:484-490`'s runtime message ("`[relay]
url` ignored: rebuild roko with `--features relay` to register...").

## Why it matters

Goal `tooling`/binary hygiene: dead code that still compiles bloats the default build and its OpenAPI surface
with routes that will 404 or do nothing, misleading anyone reading the generated API spec about what a default
`roko serve` actually exposes. The pattern precedent (`spec-c89168`, open) already flags this exact class of
problem for the `chain`/Alloy split ("Non-default features go untested... Alloy-gated code is only `cargo check`ed")
— this is the same shape of gap for a different feature.

## Where

- `crates/roko-serve/src/lib.rs:74-75` (`mod subscription_relay;`'s gating).
- `crates/roko-serve/src/openapi.rs:28,144,452-454,836-844` (unconditional relay OpenAPI registration).
- `crates/roko-serve/Cargo.toml` — the `relay` feature definition (confirm what it currently gates).

## Current state

The module and its OpenAPI entries compile and register regardless of the `relay` feature; only specific
function bodies inside `lib.rs` (e.g. lines 477-496) are behind `#[cfg(feature = "relay")]`.

## Plan

1. Put `mod subscription_relay;` itself behind `#[cfg(feature = "relay")]` in `lib.rs` (removing the
   `allow(dead_code)` workaround, which won't be needed once the module is truly absent from non-relay builds).
2. Gate `openapi.rs`'s relay imports and path/schema registrations the same way, so the default build's OpenAPI
   document has no relay routes at all.
3. Confirm the default build still compiles clean (no dangling references to the now-gated module from code that
   isn't itself feature-gated).

## Done when

- A default build (no `--features relay`) does not compile `subscription_relay` or register any relay path in
  the OpenAPI document.
- The `[[verify]]` command passes.

## Notes

- Related: `spec-c89168` (open) — the same "non-default feature goes untested/under-gated" pattern for
  `chain`/Alloy; not a duplicate (different feature, different crate scope) but worth coordinating with if the
  same person picks up both.
