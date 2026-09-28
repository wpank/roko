+++
id = "gap-ad72ac"
kind = "gap"
title = "[cli-audit I3] Inject CLI and TUI should share one acknowledged executor-neutral request"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/inject"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3"
anchors = ["crates/roko-cli/src/commands/util.rs inject", "backlog #361/#202"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Integration criterion unchecked: `roko inject` and TUI inject reuse one acknowledged executor-neutral control request (#361/#202 marked done individually).

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3`

How to verify: Trace CLI inject and TUI inject code paths to a common request type + ack.
