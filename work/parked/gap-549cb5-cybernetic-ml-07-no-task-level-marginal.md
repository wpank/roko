+++
id = "gap-549cb5"
kind = "gap"
title = "[cybernetic ML-07] No task-level Marginal Value Theorem in retry decisions"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/retry"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-07: Task-Level Marginal Value Theorem"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-07: Task-Level Marginal Value Theorem"
anchors = ["MVT in context assembly (roko-compose)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
MVT exists in context assembly only; the task retry loop should stop early when marginal improvement across attempts drops below the fleet average.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-07: Task-Level Marginal Value Theorem`

How to verify: Inspect Graph retry policy for improvement-trend based early stopping.
