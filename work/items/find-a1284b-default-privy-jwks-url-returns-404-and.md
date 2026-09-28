+++
id = "find-a1284b"
kind = "finding"
title = "Default Privy JWKS URL returns 404 and degrades health"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/jwks.rs::PRIVY_JWKS_URL", "crates/roko-serve/src/state.rs:1040", "crates/roko-serve/src/lib.rs:1044"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

serve defaults to `PRIVY_JWKS_URL = "https://auth.privy.io/.well-known/jwks.json"` (`jwks.rs:22`). A local audit found this endpoint returns 404 (Privy serves per-app JWKS URLs) and the failed fetch makes the health endpoint return 503 while Privy auth is on by default; not re-checked here (needs network).
Fix: make Privy opt-in, use the per-app JWKS URL when configured, and keep JWKS fetch failures out of liveness health.

Check on 2026-09-28 was inconclusive: Code path confirmed: with no [serve.auth].jwks_providers the cache uses the generic PRIVY_JWKS_URL (crates/roko-serve/src/state.rs:1040-1044; jwks.rs:22), build_app_state always sets privy_app_id = NUNCHI_PRIVY_APP_ID (crates/roko-serve/src/lib.rs:1044-1046) and primes JWKS (lib.rs:393), an empty key set gives fail_closed = true (jwks.rs cache_health, key_count == 0), and health returns 503 when privy_app_id is set and fail_closed (crates/roko-serve/src/routes/status/health.rs:37-57). Whether that URL actually returns 404 needs a network probe (not done), so the verdict depends on it.
