+++
id = "gap-5ec5d5"
kind = "gap"
title = "[provider F051] Cascade/health-aware routing opt-in via env var for ACP"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F051"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F051"
anchors = ["crates/roko-acp/src/bridge_events.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Health-aware cascade routing for ACP is opt-in via `ROKO_ACP_CASCADE_SELECT=1` rather than always-on. Without this env var, ACP sessions bypass health filtering and cascade routing.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F051`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-acp/src/bridge_events.rs whether still true: Cascade/health-aware routing opt-in via env var for ACP
