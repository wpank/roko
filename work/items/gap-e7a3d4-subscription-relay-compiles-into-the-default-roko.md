+++
id = "gap-e7a3d4"
kind = "gap"
title = "subscription_relay compiles into the default roko-serve build; it isn't behind the relay feature"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-serve"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK79 gap-425d9e)"
discovered_from = "gap-425d9e"
anchors = ["crates/roko-serve/src/lib.rs", "crates/roko-serve/src/state.rs::AppState", "crates/roko-serve/src/relay.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_build_has_no_subscription_relay' crates/roko-serve/ && cargo test -p roko-serve default_build_has_no_subscription_relay"
+++

## Problem

`crates/roko-serve/Cargo.toml:38` already defines `relay = []` (and `crates/roko-cli/Cargo.toml:35`:
`relay = ["roko-serve/relay"]`), and `crates/roko-serve/src/relay.rs`'s contents are already
correctly gated item-by-item with `#[cfg(feature = "relay")]` (13+ occurrences). But
`subscription_relay` — a *different* module (`lib.rs:75`: `mod subscription_relay;`, no cfg at
all) — is not behind this feature, or any feature:
- `AppState.subscription_relay: Arc<SubscriptionRelayRuntime>` (`state.rs:737`, constructed
  unconditionally at `state.rs:1206-1207,1430`) is a plain, always-present field.
- `lib.rs` wires a large block of subscription-relay client logic unconditionally (roughly
  2969-3230: cursor reconciliation, reconnect, `ServeRelayConnectionStatus` handling) — none of it
  sits inside a `#[cfg(feature = "relay")]` block (the file's `relay`-cfg'd lines are only at
  477, 496, 2910, 3248-3385, well outside that range).
So `subscription_relay` and its state/OpenAPI/status surface compile into every `roko-serve` build,
default included, regardless of whether `relay` is enabled.

## Why it matters

Release hygiene (matches the `chain`/`hdc` precedent already in `roko-cli/Cargo.toml`, which
explicitly calls out "parked out of the default build"): code for a feature nobody turned on should
not ship in the default binary's surface area (routes, OpenAPI schema, `AppState` fields) — it
widens the attack surface and the public API surface for something that isn't meant to be live yet.

## Where

- `crates/roko-serve/src/lib.rs:75` (`mod subscription_relay;`) and its unconditional wiring block
  (~2969-3230).
- `crates/roko-serve/src/state.rs:737,1206-1207,1430` (`AppState.subscription_relay`).
- `crates/roko-serve/src/openapi.rs` and `crates/roko-serve/src/routes/subscriptions.rs` (the
  status/OpenAPI entries team-lead named — neither has any `cfg(feature = "relay")`).
- Reference pattern already correct in the same crate: `crates/roko-serve/src/relay.rs`'s
  per-item `#[cfg(feature = "relay")]` gating.

## Current state

Unfixed. The `relay` feature exists and gates one module (`relay.rs`) correctly; it does not gate
`subscription_relay` at all.

## Plan

1. Gate `mod subscription_relay;` and its `AppState` field behind `#[cfg(feature = "relay")]`,
   following `relay.rs`'s own pattern (or gate the whole module declaration plus every call site,
   whichever keeps `AppState` simplest — `subscription_relay`'s field likely needs
   `#[cfg(feature = "relay")]` plus an `Option`-free default elsewhere, or the field itself removed
   under `#[cfg(not(feature = "relay"))]`).
2. Gate the unconditional wiring block in `lib.rs` (~2969-3230) the same way.
3. Gate `subscriptions.rs`'s and `openapi.rs`'s relay-specific routes/schema entries.
4. Confirm `cargo build -p roko-serve` (no features) still compiles clean with `subscription_relay`
   fully absent, and `cargo build -p roko-serve --features relay` still works with it present.

## Done when

- `subscription_relay` and its OpenAPI/status/route surface are absent from a default
  (no-features) `roko-serve` build and present only under `--features relay`.
- The `[[verify]]` command passes.

## Notes

- I did not run cargo; this is read from source only. Confirm the exact boundary of what needs
  `#[cfg]` by trying a no-features build once the gates are added.

## Progress

- gap-e7a3d4: implemented at ee25bc7e5. `subscription_relay`, `AppState.subscription_relay` (field, construction, and the `anyhow::Context` import only it used) and `dispatch_relay_subscription` are `#[cfg(feature = "relay")]`; without it `GET /api/subscriptions/relay/status` is parked (501, `required_feature: relay`) like the `/relay` proxy, and the OpenAPI document holds no relay schema (they join only through the relay build's status path). The route snapshot is unchanged (`--check-snapshot` passes); showcase mode's root unmount concerns the `/relay` proxy and is unchanged. The verify's grep passes; cargo verification (the default build and `--features relay`) is deferred to the batch gate.
