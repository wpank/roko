+++
id = "gap-83d081"
kind = "gap"
title = "ACP spec upgrade (bridge_events v0.12 -> v0.13) open"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp"]
created = 2026-09-20
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-20-final-session.md#P1 (High)"
discovered_from = "audit:tmp/dogfood/2026-09-20-final-session.md#P1 (High)"
anchors = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/types.rs:10", "crates/roko-acp/src/bridge_events/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Listed as an open P1 release item on 2026-09-20 alongside ACP stability hardening.

Imported without verification from:
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`
- `tmp/dogfood/2026-09-19-session.md#Next Steps`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Check ACP protocol version constants.

Verified 2026-09-28 (static check against 3d0ee4d02): bridge_events.rs became the module directory crates/roko-acp/src/bridge_events/, but the spec version was not upgraded: ACP_SPEC_VERSION = "0.12.2" (crates/roko-acp/src/types.rs:10), ACP_PROTOCOL_VERSION = 1 (types.rs:7), used in the initialize response (handler.rs:19, :295). No v0.13 reference exists in roko-acp.
