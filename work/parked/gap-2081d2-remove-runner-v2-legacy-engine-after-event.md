+++
id = "gap-2081d2"
kind = "gap"
title = "Remove Runner-v2 legacy engine after event_loop.rs decomposition (backlog #20/#54)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#2.4 Runner-v2 Removal"
discovered_from = "audit:docs/v3/39-ROADMAP.md#2.4 Runner-v2 Removal"
anchors = ["crates/roko-cli/src/runner/event_loop.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Runner-v2 kept as --engine legacy for one release after #260/#276; removal decided. First extract gate dispatch, persist, snapshot writer, merge and branch cleanup from the ~23K-line event_loop.rs and close Graph parity gaps (#54).

Imported without verification from:
- `docs/v3/39-ROADMAP.md#2.4 Runner-v2 Removal`
- `docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#5. Engine Audit`

Warning: every file this item cites is gone (`crates/roko-cli/src/runner/event_loop.rs`) — likely obsolete or moved.

How to verify: wc -l event_loop.rs; list --engine legacy call sites; check release-cycle deadline.
