+++
id = "bug-58dfbc"
kind = "bug"
title = "DOCS-06 A2: `roko explain` has 12 stale references to deleted orchestrate.rs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/explain"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A2. `roko explain` has 12 stale references to deleted `orchestrate.rs`"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A2. `roko explain` has 12 stale references to deleted `orchestrate.rs`"
anchors = ["roko explain", "orchestrate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
12 explain topics point at a file that no longer exists, producing broken output (decision: FIX). 09-03: 77 stale refs fixed across 38 files, yet 09-15 still lists 12.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A2. `roko explain` has 12 stale references to deleted `orchestrate.rs``
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Potential Dead Code — VERIFY THEN CLEAN`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: grep explain sources for orchestrate.rs. / grep explain topics for orchestrate.rs.

Merged 2 mined candidates: m4-032, m5-082.
