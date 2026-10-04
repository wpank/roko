+++
id = "gap-511268"
kind = "gap"
title = "ServerEvent::AgentSpawned has no provider field; serve's bridge drops it in both directions"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-core"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-15 follow-up reports 2026-10-04 (bug-2e5429)"
discovered_from = "bug-2e5429 (closed; own Progress note names this as a separate gap)"
anchors = ["crates/roko-serve/src/events.rs::ServerEvent", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_spawned_provider_survives_the_serve_bridge' crates/roko-serve/ && cargo test -p roko-serve agent_spawned_provider_survives_the_serve_bridge"
+++

## Problem

`roko-serve`'s `ServerEvent::AgentSpawned` has no `provider` field, so serve's bridge drops the
provider label in both directions and SSE/WS clients never see which provider an agent runs on.
`ServerEvent::AgentSpawned` (`crates/roko-serve/src/events.rs:123-128`) declares only `agent_id,
role, model` — no `provider`. The kernel's own `DashboardEvent::AgentSpawned`
(`crates/roko-core/src/dashboard_snapshot.rs:141-157`) has a real `provider: String` field with
a doc comment describing exactly this use ("Provider label (e.g. `"claude-cli"`,
`"codex-cli"`); empty when the emitter does not know it."), and `bug-2e5429` (closed) just fixed
the TUI/dashboard side to populate it correctly from the resolved `ProviderDispatchResolver`
label instead of a lossy `AgentBackend` round-trip.

The bridge between the two event types drops it on both conversions:
- `ServerEvent::AgentSpawned -> DashboardEvent::AgentSpawned`
  (`crates/roko-serve/src/lib.rs:1755-1766`) hardcodes `provider: String::new()`.
- `DashboardEvent::AgentSpawned -> ServerEvent::AgentSpawned`
  (`crates/roko-serve/src/lib.rs:2236-2244`) destructures with `{agent_id, role, model, ..}`
  (the `..` discards `provider` along with `plan_id`/`task_id`/`attempt`), and constructs a
  `ServerEvent::AgentSpawned` literal that has no `provider:` field to set even if it tried.

## Why it matters

Goal: visibility, same goal as `bug-2e5429`. `bug-2e5429`'s fix makes the provider label correct
at the TUI/dashboard layer, but any consumer reaching an agent-spawn event through `roko-serve`'s
HTTP control plane (SSE or WS) — the demo app, an external dashboard, any REST/SSE client —
still can't tell which provider (and specifically, per `bug-2e5429`, whether a Claude-family
model ran via the CLI subprocess or the direct Anthropic API) an agent is running on. The fix
that just landed is invisible outside the TUI process.

## Where

- `crates/roko-serve/src/events.rs::ServerEvent::AgentSpawned` (needs a `provider` field).
- `crates/roko-serve/src/lib.rs:1755-1766` (`ServerEvent` → `DashboardEvent`, drops it inbound).
- `crates/roko-serve/src/lib.rs:2236-2244` (`DashboardEvent` → `ServerEvent`, drops it outbound).
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent::AgentSpawned` (the field to carry
  through; read-only reference).

## Current state

Confirmed by reading both bridge functions and both event structs: the field exists on one side
only, and both conversion directions actively discard it rather than merely defaulting it.

## Plan

1. Add `provider: String` (or `Option<String>`) to `ServerEvent::AgentSpawned`.
2. In the `ServerEvent -> DashboardEvent` conversion, carry the incoming `provider` through
   instead of hardcoding `String::new()`.
3. In the `DashboardEvent -> ServerEvent` conversion, read `provider` out of the destructure
   (drop it from the `..`) and set it on the constructed `ServerEvent::AgentSpawned`.
4. Regression test: an `AgentSpawned` round-trip through both conversions preserves a non-empty
   provider label.

## Done when

- A provider label set on either event type survives a round trip through both bridge
  conversions.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-15 follow-up, bug-2e5429, main HEAD `7d82b944a`): `bug-2e5429`'s own Progress
  note names this exact follow-up: "Serve's `ServerEvent::AgentSpawned` drops the provider field
  altogether: a separate gap." Confirmed by reading `events.rs` and both `lib.rs` bridge
  functions directly.

## Progress

- gap-511268: implemented at b1260c64f. `ServerEvent::AgentSpawned` has an optional `provider` (left out of the wire when unknown), and both bridge conversions carry it; docs/v3/26-HTTP-API.md's event catalog and the demo app's `DashboardEvent` type list it. The verify's grep passes; cargo verification is deferred to the batch gate (`agent_spawned_provider_survives_the_serve_bridge`).
