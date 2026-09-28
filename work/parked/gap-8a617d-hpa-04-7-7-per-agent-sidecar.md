+++
id = "gap-8a617d"
kind = "gap"
title = "HPA-04 §7.7: Per-agent sidecar URL discovery has no stable contract"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent-server"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.7. Per-agent sidecar URL discovery is manual"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.7. Per-agent sidecar URL discovery is manual"
anchors = ["crates/roko-cli/src/tui/ws_client.rs", "sidecar_url"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Sidecars bind random ports; clients learn sidecar_url from GET /api/agents/:id, which is not guaranteed populated, and must replicate ws_client.rs fallback logic.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.7. Per-agent sidecar URL discovery is manual`

How to verify: Check agents route contract for sidecar_url.
