+++
id = "gap-a48e95"
kind = "gap"
title = "HPA-04 §7.5/R8, HPA-07 §1.2: CORS policy undocumented/untested and browser EventSource cannot authenticate"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/auth"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.5. No CORS configuration documented / tested"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.5. No CORS configuration documented / tested"
anchors = ["crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/sse.rs", "ServeAuthConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
CORS layer exists but its config and cross-origin behavior are undocumented/untested; EventSource cannot send Authorization headers, so SSE/WS need ?token= (or cookie) auth for browser clients.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.5. No CORS configuration documented / tested`
- `tmp/hermes-product-audit/07-roko-ux-roadmap.md#1.2 CORS and auth plumbing for browser origin`
- `tmp/hermes-product-audit/05-roko-demoability.md#4e. CORS Configuration for Demo Deployment`

How to verify: Check [serve] cors config keys and token query-param support in sse.rs/ws.rs.
