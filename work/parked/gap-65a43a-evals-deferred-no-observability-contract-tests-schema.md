+++
id = "gap-65a43a"
kind = "gap"
title = "[evals deferred] No observability contract tests (schema checks for structured logs)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-runtime/events"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap"
anchors = ["RuntimeEventEnvelope", "episodes.jsonl/efficiency.jsonl schemas"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deferred: structured log / JSONL schemas have no contract tests; requires a schema registry decision.

Imported without verification from:
- `tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap`
- `tmp/archive/evals-audit/16-endpoint-data-flow.md`

Warning: every file this item cites is gone (`episodes.jsonl/efficiency.jsonl`) — likely obsolete or moved.

How to verify: Look for schema/golden tests covering JSONL event formats.
