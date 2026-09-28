+++
id = "find-a1284b"
kind = "finding"
title = "Default Privy JWKS URL returns 404 and degrades health"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/jwks.rs::PRIVY_JWKS_URL"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

serve defaults to `PRIVY_JWKS_URL = "https://auth.privy.io/.well-known/jwks.json"` (`jwks.rs:22`). A local audit found this endpoint returns 404 (Privy serves per-app JWKS URLs) and the failed fetch makes the health endpoint return 503 while Privy auth is on by default; not re-checked here (needs network).
Fix: make Privy opt-in, use the per-app JWKS URL when configured, and keep JWKS fetch failures out of liveness health.
