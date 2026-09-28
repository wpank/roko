+++
id = "bug-2c9fa8"
kind = "bug"
title = "[graph verify] 6 pre-existing roko-serve test failures recorded at 2026-09-05"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/tests"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/graph-audit/08-completion-status.md#verification-results"
discovered_from = "audit:tmp/archive/graph-audit/08-completion-status.md#verification-results"
anchors = ["crates/roko-serve/tests/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Completion verification recorded `cargo test -p roko-serve`: 805 passed, 6 failed (all 6 pre-existing); the failing tests were not named or fixed.

Imported without verification from:
- `tmp/archive/graph-audit/08-completion-status.md#verification-results`

How to verify: Run or inspect latest CI for `cargo test -p roko-serve` failures.
