+++
id = "find-a1284b"
kind = "finding"
title = "Default Privy JWKS URL returns 404 and degrades health"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/jwks.rs::PRIVY_JWKS_URL", "crates/roko-serve/src/state.rs:1145", "crates/roko-serve/src/lib.rs:1148", "crates/roko-serve/src/routes/status/health.rs:86"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'privy_app_id = Some(crate::jwks::NUNCHI_PRIVY_APP_ID' crates/roko-serve/src/lib.rs && ! grep -q 'jwks_configured && jwks_health.fail_closed' crates/roko-serve/src/routes/status/health.rs"
+++

serve defaults to `PRIVY_JWKS_URL = "https://auth.privy.io/.well-known/jwks.json"` (`jwks.rs:22`). A local audit found this endpoint returns 404 (Privy serves per-app JWKS URLs) and the failed fetch makes the health endpoint return 503 while Privy auth is on by default; not re-checked here (needs network).
Fix: make Privy opt-in, use the per-app JWKS URL when configured, and keep JWKS fetch failures out of liveness health.

Check on 2026-09-28 was inconclusive: Code path confirmed: with no [serve.auth].jwks_providers the cache uses the generic PRIVY_JWKS_URL (crates/roko-serve/src/state.rs:1040-1044; jwks.rs:22), build_app_state always sets privy_app_id = NUNCHI_PRIVY_APP_ID (crates/roko-serve/src/lib.rs:1044-1046) and primes JWKS (lib.rs:393), an empty key set gives fail_closed = true (jwks.rs cache_health, key_count == 0), and health returns 503 when privy_app_id is set and fail_closed (crates/roko-serve/src/routes/status/health.rs:37-57). Whether that URL actually returns 404 needs a network probe (not done), so the verdict depends on it.

Re-checked 2026-09-29: still true. GET https://auth.privy.io/.well-known/jwks.json returns HTTP 404 and the code path is unchanged (crates/roko-serve/src/jwks.rs:22 PRIVY_JWKS_URL; health 503 decision at crates/roko-serve/src/routes/status/health.rs:86).

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-928add` at `6a36045aa`; cargo verification deferred to the batch check. The static `[[verify]]` passes. Re-checked at BASE `8a88c6267`: the opt-in half was done in `3fd7bfc76` (no default `privy_app_id`, JWKS primed only when one is set), but two parts remained. First, with no `jwks_providers` the cache still used the generic `https://auth.privy.io/.well-known/jwks.json`. Probed 2026-09-30, that URL returns 404, while `https://auth.privy.io/api/v1/apps/<app-id>/jwks.json` returns 200 with ES256 keys. `jwks::jwks_providers_for` now uses the operator's providers, else Privy's per-app endpoint, else none while Privy auth is off; `PRIVY_JWKS_URL` was removed. Second, `/api/health` returned 503 on `jwks_configured && fail_closed`; missing or stale keys now return `degraded` with HTTP 200 (`routes::status::tests::health_stays_live_when_configured_jwks_is_fail_closed`, which replaces the old 503 test). `try_privy_jwt` and the allow-list (gap-47e0de) are untouched.
