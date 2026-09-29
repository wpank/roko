+++
id = "bug-ff7aa0"
kind = "bug"
title = "[status-quo dogfood] `status` signal counts show 0 for Graph activity"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/status"]
created = 2026-09-10
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
anchors = [".roko/engrams.jsonl", ".roko/state/graph/ activities"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Graph engine activity signals are written to .roko/state/graph/ activity logs, not the signal log, so `roko status` reports 'signal counts (0 total)'.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)`

Some cited files are gone: `.roko/engrams.jsonl`.

How to verify: Compare status signal counts to Graph activity log entries after a run.
