+++
id = "gap-82e418"
kind = "gap"
title = "[plan-audit T1-09] Agent contracts missing for 20 of 28 roles (deny-all)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-09: Write agent contracts for remaining roles"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-09: Write agent contracts for remaining roles"
anchors = ["crates/roko-agent/src/safety/contracts/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Only 8 YAML contracts exist (impl, arch, audit, auto-fix, researcher, reviewer, scribe, strategist); Conductor, Refactorer, MergeResolver, IntegrationTester, etc. get deny-all tool access and cannot be dispatched.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-09: Write agent contracts for remaining roles`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: ls contracts dir; compare against role enum.
