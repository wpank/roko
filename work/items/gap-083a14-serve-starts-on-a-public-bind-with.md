+++
id = "gap-083a14"
kind = "gap"
title = "serve starts on a public bind with a Privy app id and no allow-list (G0b startup refusal missing)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-9:q-89756f"
anchors = ["crates/roko-serve/src/lib.rs::validate_bind_safety", "crates/roko-serve/src/lib.rs::is_loopback_addr", "crates/roko-serve/src/routes/middleware.rs::privy_allow_list_configured"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["q-89756f", "gap-fcb44c", "bug-7eef96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn non_loopback_bind_refuses_privy_without_allowlist' crates/roko-serve/src && cargo test -p roko-serve --lib non_loopback_bind_refuses_privy_without_allowlist"
+++

## Problem

S11 G0b requires that serve refuse to start on a non-loopback bind when Privy is enabled and no allow-list is
configured. Today `validate_bind_safety` (`crates/roko-serve/src/lib.rs:1012-1030`) only checks `auth.enabled` or
`acknowledge_public_risk`. A Privy app id with no allow-list only logs `PRIVY_ALLOW_LIST_HINT` from
`warn_if_auth_misconfigured` (`lib.rs:954-955`), and serve starts. Every Privy JWT is then rejected
(`try_privy_jwt`, reject-not-downgrade, bug-7eef96), so the failure is closed but quiet: the operator finds out
when logins fail. Decision 9 (q-89756f) keeps `privy_app_id` as the enable gate (no `privy_enabled` key) and
asks for this refusal.

## Why it matters

Goal release, S11 fly-deploy security (G0b). A public deploy with half-configured Privy should fail at startup
with a message, not at the first login.

## Where

- `crates/roko-serve/src/lib.rs::validate_bind_safety` (the public-bind refusal; called at startup).
- `crates/roko-serve/src/lib.rs::is_loopback_addr` (line 916).
- `crates/roko-serve/src/routes/middleware.rs::privy_allow_list_configured` and `PRIVY_ALLOW_LIST_HINT` (line 34).
- `crates/roko-serve/src/lib.rs::validate_showcase_mode` already refuses any Privy app id in showcase mode
  (`lib.rs:1000-1001`); this item covers non-showcase serve.

## Current state

No refusal for non-showcase public binds. The showcase refusal and the startup warning exist.

## Plan

1. In `validate_bind_safety`, when the bind is not loopback, `serve.auth.privy_app_id` is set and
   `privy_allow_list_configured(&serve.auth)` is false, bail with an error that quotes `PRIVY_ALLOW_LIST_HINT`.
   `acknowledge_public_risk` does not override this; the fix is to set the allow-list or unset the app id.
2. Loopback binds keep today's behaviour (warning only), so local development is unchanged.
3. Add the unit test `non_loopback_bind_refuses_privy_without_allowlist` in `lib.rs`'s tests: a public bind with
   an app id and no allow-list errors; adding `privy_workspace_id` or `privy_allowed_roles` passes; a loopback
   bind passes.
4. Amend S11 G0b (`tmp/cybernetic-harness/specs/S11-fly-deploy-security.md:133,248`): the gate is
   `privy_app_id`, there is no `privy_enabled` key, and the test is the one above.

## Done when

`grep -rqw 'fn non_loopback_bind_refuses_privy_without_allowlist' crates/roko-serve/src && cargo test -p roko-serve --lib non_loopback_bind_refuses_privy_without_allowlist`
passes.

## Notes

- From q-89756f, decided under decision 9 (T1, 2026-10-15): Option B (no `privy_enabled` key).
- Do not touch `fly.showcase.toml`: showcase mode already forbids Privy.
