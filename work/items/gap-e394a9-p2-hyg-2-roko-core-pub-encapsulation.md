+++
id = "gap-e394a9"
kind = "gap"
title = "P2-HYG-2: roko-core pub encapsulation (~5,510 pub items)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["workspace"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-HYG-2 (Subsystem: Code Hygiene)"
discovered_from = "audit:tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-HYG-2 (Subsystem: Code Hygiene)"
anchors = ["crates/roko-core/src/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[deferred] roko-core pub encapsulation (~5,510 pub items). P2-HYG-1: 8,342 total (7,623 excl. test dirs). Ongoing incremental work. P2-HYG-2: Needs systematic audit across roko-core.

Imported without verification from:
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-HYG-2 (Subsystem: Code Hygiene)`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P2 items: 8/10 done`
- `tmp/archive/status-quo-audit-2026-09-21/07-tech-debt.md#TD-05: `roko-core` has no encapsulation (Severity: Medium)`

How to verify: Confirm against code: P2-HYG-1: 8,342 total (7,623 excl. test dirs). Ongoing incremental work. P2-HYG-2: Needs systematic audit across roko-core. / Count pub items in roko-core; check for pub(crate) adoption.

Merged 2 mined candidates: m1-177, m3-094.
