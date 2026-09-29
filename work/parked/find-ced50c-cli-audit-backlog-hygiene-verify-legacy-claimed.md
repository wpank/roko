+++
id = "find-ced50c"
kind = "finding"
title = "[cli-audit backlog hygiene] Verify legacy claimed-complete packets and archive superseded ones"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["backlog"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/FINDINGS-COVERAGE-MATRIX.md#Verification-only / archive queue"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/FINDINGS-COVERAGE-MATRIX.md#Verification-only / archive queue"
anchors = ["tmp/backlog/ packets #49 #50 #51 #65 #77 #79 #100 #113 #146 #222 #132 #147"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Verify-only queue: #49/#50/#51/#65/#77/#79/#100/#113/#146/#222 claimed-complete behavior unverified; superseded scopes (#115/#128/#147/#192 old, #132) not archived; zero-caller/stale-ref rescans pending.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/FINDINGS-COVERAGE-MATRIX.md#Verification-only / archive queue`
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 5 — retirement and residue`

How to verify: For each packet, spot-check its acceptance symbols in current source.
