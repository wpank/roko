+++
id = "bug-24232b"
kind = "bug"
title = "Fix 250ms async path keyboard latency"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.9 Fix 250ms async path keyboard latency"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.9 Fix 250ms async path keyboard latency"
anchors = ["crates/roko-cli/src/tui/app.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The async run path hardcodes a 250ms poll timeout, creating up to 250ms keyboard latency during `plan run`. Change to the same adaptive tick (16ms active / 50ms idle) used by the sync path.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.9 Fix 250ms async path keyboard latency`

Warning: every file this item cites is gone (`crates/roko-cli/src/tui/app.rs`) — likely obsolete or moved.

How to verify: Source: UX audit report 16 (TUI performance); TUI parity MX.5. Check `crates/roko-cli/src/tui/app.rs` (async `run()` path) for: The async run path hardcodes a 250ms poll timeout, creating up to 250ms keyboard latency during `plan run`. Change to the same adaptive tick (16ms active /…
