+++
id = "gap-b527dc"
kind = "gap"
title = "Recursive Safety Continuous Monitoring"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/09-recursive-safety-patterns.md#09 — Recursive Safety Continuous Monitoring"
discovered_from = "audit:tmp/backlog/archive/09-recursive-safety-patterns.md#09 — Recursive Safety Continuous Monitoring"
anchors = ["crates/roko-agent/", "crates/roko-core/", "crates/roko-serve/", "crates/roko-cli/", "crates/roko-runtime/", "crates/roko-agent/src/safety/recursive.rs", "crates/roko-agent/src/lifecycle.rs", "recursive.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[todo] CONSOLIDATED P3-SAF-1: deferred — background safety monitoring for meta-agent lineages; no impact on current operation since roko has no deployed meta-agent lineages today. Roko supports "meta-agents" — agents that can spawn other agents, creating recursive lineages. The R04 implementation…

Imported without verification from:
- `tmp/backlog/archive/09-recursive-safety-patterns.md#09 — Recursive Safety Continuous Monitoring`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-SAF-1 (Subsystem: Safety / Recursive)`

How to verify: Check: `RecursiveSafetyMonitor::scan()` correctly identifies `RateLimitViolation` when `creations_this_hour > max_creations_per_hour` for any process in the snapshot.; `scan()` correctly identifies `GlobalRateExceeded` when the sum of all… [evidence: CONSOLIDATED P3-SAF-1: deferred; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): L | 7 |]
