+++
id = "gap-703eb0"
kind = "gap"
title = "[engine row6-migration-cli] Graph-to-Graph checkpoint migration CLI (deferred until first breaking schema change)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/hot"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
discovered_from = "audit:tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
anchors = ["crates/roko-graph/src/hot.rs", "HOT_CHECKPOINT_SCHEMA_VERSION", "crates/roko-graph/src/fingerprint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Deferred ledger row 6: trigger on the first breaking graph schema change not handled by #251 versioned extension migration.

Imported without verification from:
- `tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table`
- `tmp/archive/engine-audit/16-convergence-plan.md`

How to verify: Check whether HOT_CHECKPOINT_SCHEMA_VERSION or graph checkpoint schema has been bumped since 2026-09-05 without a migration path.
