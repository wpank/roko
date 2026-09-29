+++
id = "find-e5573b"
kind = "finding"
title = "[engine row5-error-algebra] Unified typed error algebra for Graph engine (redesign deferred)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/error"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
discovered_from = "audit:tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
anchors = ["crates/roko-graph/src/error.rs", "GraphError", "FailureStrategy"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deferred ledger row 5: current GraphError/FailureStrategy retained after #268; a unified error algebra redesign remains future product work.

Imported without verification from:
- `tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table`
- `tmp/archive/engine-audit/02-graph-engine-gaps.md`

How to verify: Low priority; confirm GraphError still coexists with separate gate/dispatch/controller error types.
