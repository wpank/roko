+++
id = "bug-280407"
kind = "bug"
title = "HPA-04 §7.6/R7: WebSocket Coalesce/ResumeRequired back-pressure modes silently fall back"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes/ws"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.6. WebSocket back-pressure modes unimplemented"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.6. WebSocket back-pressure modes unimplemented"
anchors = ["crates/roko-serve/src/routes/ws.rs", "BackPressureMode"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
ws.rs declares BackPressureMode AtMostOnce/Coalesce/ResumeRequired but only AtMostOnce is implemented; the others log a warning and fall back, so slow clients drop events with no recovery short of reconnect.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.6. WebSocket back-pressure modes unimplemented`
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#R7 (Low Priority): Implement WebSocket `Coalesce` back-pressure mode`

How to verify: Read the BackPressureMode match arms.
