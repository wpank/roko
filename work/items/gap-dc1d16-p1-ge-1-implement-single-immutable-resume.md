+++
id = "gap-dc1d16"
kind = "gap"
title = "Implement single immutable resume generation"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-graph"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)"
discovered_from = "audit:tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)"
anchors = ["crates/roko-graph/src/snapshot.rs", "crates/roko-cli/src/graph_checkpoint.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
links = { depends_on = [], blocks = [], related = ["gap-9c82d3"], supersedes = [], duplicate_of = "" }
+++
Implement single immutable resume generation. Resume after crash should reconstruct exact pre-crash derived state. Requires deterministic replay from checkpoint. Depends on P0-GE-1.

Imported without verification from:
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)`
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt`

How to verify: Confirm against code: Resume after crash should reconstruct exact pre-crash derived state. Requires deterministic replay from checkpoint. Depends on P0-GE-1. Its blocker P0-GE-1 is marked done 2026-09-20, so it may now be unblocked. / Check whether resume selects exactly one immutable generation and rebuilds derived state deterministically.

Merged 2 mined candidates: m1-171, m2-172.

Verified 2026-09-28: there is no generation concept in crates/roko-graph/src/snapshot.rs, crates/roko-cli/src/graph_checkpoint.rs (which has uncommitted concurrent edits) or graph_execution/. Resume combines the graph checkpoint, Activity log, cost state and extensions without selecting one immutable generation, and CLAUDE.md still lists single-generation resume as partial. P0-GE-1's in-process harness exists (crates/roko-graph/tests/crash_resume_equivalence.rs). This overlaps the second half of gap-9c82d3.
