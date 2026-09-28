+++
id = "bug-e8ddb7"
kind = "bug"
title = "Enrichment artifacts empty"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#15)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#15)"
anchors = ["skip_enrichment"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Enrichment artifacts come out empty (#15); marked moot while `skip_enrichment` is used but never resolved. Related: mori-diffs J-1 enrichment artifacts schema/receipt.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#15)`

How to verify: Run an enrichment-enabled plan and inspect enrichment artifacts; check receipt/schema population.
