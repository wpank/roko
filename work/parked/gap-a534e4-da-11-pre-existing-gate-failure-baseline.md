+++
id = "gap-a534e4"
kind = "gap"
title = "DA-11: Pre-existing gate-failure baseline classification unproven; BenchmarkRegressionGate always passes"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-gate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)"
discovered_from = "audit:tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)"
anchors = ["BenchmarkRegressionGate", "backlog #166", "backlog #170"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Focused baseline classification for pre-existing failures exists but was never proven against the dogfood recurrence pattern (backlogs #166, #170); the CLI audit found BenchmarkRegressionGate always passes.

Imported without verification from:
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/archive/dogfood-audit-2026-09-03/05-status-update-2026-09-01.md#4. #166/#170 — Gate Verify Preexisting Filter + Adaptive Verify Scoping`

How to verify: Read BenchmarkRegressionGate impl; check baseline-failure classification in Graph gate cells.
