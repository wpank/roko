+++
id = "q-89756f"
kind = "question"
title = "G0b's privy_enabled config key: add it, or amend S11 to match the shipped privy_app_id gate?"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate-13c follow-up reports 2026-10-04 (PK85 gap-fcb44c, task 9334)"
discovered_from = "gap-fcb44c (closed; own done-note already flagged this at task 9334)"
anchors = ["crates/roko-core/src/config/serve.rs::ServeAuthConfig", "crates/roko-serve/src/routes/middleware.rs::try_privy_jwt"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

S11 G0b names a `ServeAuthConfig.privy_enabled` config key (default `false`) that does not
exist in code, so `fly.showcase.toml` cannot set it and omits any Privy configuration entirely.
`crates/roko-core/src/config/serve.rs::ServeAuthConfig` has `privy_app_id: Option<String>`,
`privy_workspace_id: Option<String>` and `privy_allowed_roles: Vec<String>` — no `privy_enabled`
field anywhere (confirmed: zero matches in `serve.rs`, `loader.rs`, or `fly.showcase.toml`).
`tmp/cybernetic-harness/specs/S11-fly-deploy-security.md:133` (G0b) specifies the key by name
and its example config block (`S11...md:248`) shows `privy_enabled = false # NEW (G0b)`.

But the *security property* G0b exists for is already shipped, via a different mechanism:
`bug-7eef96` (done)'s evidence says "Privy JWT auth is off unless `serve.auth.privy_app_id` is
set explicitly," and rejects (not downgrades) every Privy JWT unless an allow-list
(`privy_workspace_id`/`privy_allowed_roles`) is configured — matching G0b's "reject, never
read" requirement (D18) via `privy_app_id.is_some()` as the de facto enable gate, instead of a
dedicated boolean. One part of G0b is still genuinely missing either way: "Startup refuses a
non-loopback bind with Privy enabled and no allow-list." No such check exists — a grep for
`showcase_refuses_privy_without_allowlist` (G0b's own named test) and for any
loopback-bind-plus-Privy check returns nothing anywhere in `crates/`.

## Why it matters

Goal: release, S11 fly-deploy security (G0b, required). Two different things are tangled here:
whether the spec and code agree on a name (cosmetic, but it blocks `fly.showcase.toml` from
expressing intent explicitly and blocks anyone grepping for `privy_enabled` from finding the
real gate), and whether the startup refusal on a public bind with Privy mis-configured exists
(a real, unimplemented safety check — a public showcase deploy that somehow ends up with
`privy_app_id` set and no allow-list has nothing stopping it from starting).

## Where

- `crates/roko-core/src/config/serve.rs::ServeAuthConfig` (has `privy_app_id`, not
  `privy_enabled`).
- `crates/roko-serve/src/lib.rs::build_app_state`, `crates/roko-serve/src/routes/middleware.rs::try_privy_jwt`
  (the shipped off-by-default/reject-not-downgrade behavior, bug-7eef96).
- `fly.showcase.toml` (omits all Privy keys; currently safe by the `privy_app_id`-unset default,
  but doesn't match the spec's example block).
- `tmp/cybernetic-harness/specs/S11-fly-deploy-security.md:50,71,132-133,248,338` (G0a/G0b, the
  `privy_enabled` references).

## Plan (decision needed)

- **Option A — add `privy_enabled: bool` (default `false`)** to `ServeAuthConfig`, gate
  `build_app_state`/`try_privy_jwt` on it explicitly instead of inferring from
  `privy_app_id.is_some()`, and set it explicitly in `fly.showcase.toml`. Matches the spec
  text exactly; adds one more field to keep in sync with `privy_app_id`.
- **Option B — amend S11's G0b text** to describe the mechanism actually shipped
  (`privy_app_id.is_some()` as the gate, reject-not-downgrade without an allow-list) instead of
  a separate `privy_enabled` key, and drop the `privy_enabled = false` line from its example
  config block. No code change; `fly.showcase.toml` stays as is (Privy already off by omission).
- **Either way:** add the startup check G0b also names — refuse to bind non-loopback when Privy
  is effectively enabled (`privy_app_id` set, under either option's naming) with no allow-list
  configured — and the test it names, `showcase_refuses_privy_without_allowlist`.

## Done when

- Will (or whoever picks this up) picks Option A or B, and the startup-refusal check and its
  named test exist regardless of which option is chosen.

## Notes

- 2026-10-04 (gate-13c follow-up, PK85 gap-fcb44c, task 9334): confirmed at main HEAD
  `908f7ec40`. Matches `gap-fcb44c`'s own done-note verbatim ("9334: implemented at
  `c40515248` (cargo verification deferred; G0b's `privy_enabled` key does not exist, so the
  config omits it)"). Filed as `kind = "question"` for the naming decision; the startup-refusal
  sub-gap is real and unimplemented under either answer, so it's recorded in the Plan regardless
  of which option Will picks.
