+++
id = "find-3b5196"
kind = "finding"
title = "[status-quo TD-01/P1-F] roko-cli crate-level clippy allow blanket largely remains"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/07-tech-debt.md#TD-01: Crate-level lint suppression blanket (Severity: High)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/07-tech-debt.md#TD-01: Crate-level lint suppression blanket (Severity: High)"
anchors = ["crates/roko-cli/src/lib.rs lines 1-90", "crates/roko-cli/src/main.rs lines 10-38"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
roko-cli lib.rs suppressed 84 clippy lint categories (plus dead_code/missing_docs/unused_*) and main.rs a 15-lint block, labeled 'temporary'. P1-F removed only 5 suppressions; clippy::unwrap_used suppression still hides unwraps.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/07-tech-debt.md#TD-01: Crate-level lint suppression blanket (Severity: High)`
- `tmp/archive/status-quo-audit-2026-09-21/08-recommendations.md#4. Remove the 84-lint suppression blanket from roko-cli`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P1 items: 5/7 done`

How to verify: grep -c 'clippy::' crates/roko-cli/src/lib.rs crates/roko-cli/src/main.rs
