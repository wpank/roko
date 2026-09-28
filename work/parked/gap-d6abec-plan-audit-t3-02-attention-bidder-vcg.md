+++
id = "gap-d6abec"
kind = "gap"
title = "[plan-audit T3-02] Attention bidder VCG auction not the main composition path"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/auction"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["crates/roko-compose/src/auction.rs", "AttentionBidder"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
VCG attention bidder auction is built in roko-compose but not wired to the main composition pipeline; crash durability gap and no provider cost attribution.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: Check whether dispatch prompt assembly runs the auction.
