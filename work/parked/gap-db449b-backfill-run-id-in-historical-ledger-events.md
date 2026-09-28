+++
id = "gap-db449b"
kind = "gap"
title = "Backfill run_id in Historical Ledger Events"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/393-run-id-backfill-ledger-events.md#393 — Backfill run_id in Historical Ledger Events"
discovered_from = "audit:tmp/backlog/archive/393-run-id-backfill-ledger-events.md#393 — Backfill run_id in Historical Ledger Events"
anchors = ["crates/roko-cli/src/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
41,935 task_started entries missing run_id. Dev-audit found 41,935 historical task_started entries in efficiency/episode logs that are missing the run_id field. New events include run_id, but historical data cannot be correlated to specific runs.

Imported without verification from:
- `tmp/backlog/archive/393-run-id-backfill-ledger-events.md#393 — Backfill run_id in Historical Ledger Events`

How to verify: Check whether the gap described in tmp/backlog/archive/393-run-id-backfill-ledger-events.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
