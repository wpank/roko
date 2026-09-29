+++
id = "gap-72adff"
kind = "gap"
title = "DOCS-07 TD-09: StateHub baseline/overlay rebasing leaves SSE/WS consumers stale"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/statehub"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-09: StateHub Baseline/Overlay Rebasing"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-09: StateHub Baseline/Overlay Rebasing"
anchors = ["StateHub", "crates/roko-cli/src/state_hub_ipc.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When the baseline snapshot changes, SSE/WS consumers may keep stale overlay state, making dashboards inconsistent (also a CLAUDE.md partial: overlay/SSE cursor atomicity). P1-SH-1 done 09-20; P0-SH-1 final cleanup still open.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-09: StateHub Baseline/Overlay Rebasing`
- `tmp/dogfood/2026-09-20-final-session.md#P0 (Critical)`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

How to verify: Trace baseline replacement path and overlay rebase in StateHub.
