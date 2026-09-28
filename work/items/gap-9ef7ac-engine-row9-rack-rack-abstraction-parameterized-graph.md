+++
id = "gap-9ef7ac"
kind = "gap"
title = "[engine row9-rack] Rack abstraction (parameterized Graph interface) deferred"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/types"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
discovered_from = "audit:tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
anchors = ["crates/roko-graph/src/types.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Deferred ledger row 9: create work only after two production callers require the same parameterized Graph interface; no current symbols.

Imported without verification from:
- `tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table`
- `tmp/archive/engine-audit/13-spec-v3-vision.md`

How to verify: Trigger-gated; check whether two production callers now build the same parameterized Graph.
