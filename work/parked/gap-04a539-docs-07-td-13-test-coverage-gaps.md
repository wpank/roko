+++
id = "gap-04a539"
kind = "gap"
title = "DOCS-07 TD-13: Test coverage gaps (roko-execution has no integration tests)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-execution"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-13: Test Coverage Gaps"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-13: Test Coverage Gaps"
anchors = ["crates/roko-execution/tests/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
roko-execution has only inline unit tests and no tests/ directory; several crates lack cross-crate integration suites.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-13: Test Coverage Gaps`

Warning: every file this item cites is gone (`crates/roko-execution/tests/`) — likely obsolete or moved.

How to verify: ls crates/roko-execution/tests.
