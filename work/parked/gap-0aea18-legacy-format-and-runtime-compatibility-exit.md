+++
id = "gap-0aea18"
kind = "gap"
title = "Legacy Format and Runtime Compatibility Exit"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["workspace"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/336-legacy-format-compatibility-exit.md#336 — Legacy Format and Runtime Compatibility Exit"
discovered_from = "audit:tmp/backlog/archive/336-legacy-format-compatibility-exit.md#336 — Legacy Format and Runtime Compatibility Exit"
anchors = [".roko/state/executor.json", "state_snapshot.rs", ".roko/state/state-snapshot.json", "roko-acp/src/bridge_events.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[blocked] Blocked on caller/format evidence — The audit found 20 legacy-format/path references. They are functional compatibility readers/runtime fallbacks, not immediate correctness bugs, but there is no single exit ledger proving which can be removed and which are still required. The 15 formal…

Imported without verification from:
- `tmp/backlog/archive/336-legacy-format-compatibility-exit.md#336 — Legacy Format and Runtime Compatibility Exit`

Warning: every file this item cites is gone (`.roko/state/executor.json`, `.roko/state/state-snapshot.json`, `roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Check: Legacy snapshot and ACP removals have format-specific evidence and rollback-safe migration tests; plugin V1 remains an explicitly supported/tested reader.; Current writers emit only canonical snapshot/plugin schema and the non-legacy ACP… [evidence: own status: Blocked on caller/format evidence]
