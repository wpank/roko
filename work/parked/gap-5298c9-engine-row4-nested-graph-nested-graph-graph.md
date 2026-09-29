+++
id = "gap-5298c9"
kind = "gap"
title = "[engine row4-nested-graph] Nested Graph / Graph-as-Cell: deeper trace/cancel/budget nesting for authored graphs"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/engine"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
discovered_from = "audit:tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
anchors = ["crates/roko-graph/src/engine.rs", "GraphEngine::execute", "crates/roko-graph/src/cells/corrigibility.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deferred ledger row 4: #268 landed Cell contract and corrigibility/immune sub-Graphs show nested execution, but deeper trace/cancel/budget nesting for authored graphs remains future product work.

Imported without verification from:
- `tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table`
- `tmp/archive/engine-audit/02-graph-engine-gaps.md`

How to verify: Check whether an authored graph can embed a sub-Graph node with propagated trace, cancellation, and budget.
