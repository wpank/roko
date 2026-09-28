+++
id = "find-b5cbd9"
kind = "finding"
title = "S11: Mutex Poisoning & Unwrap Risks"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S11. Mutex Poisoning & Unwrap Risks"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S11. Mutex Poisoning & Unwrap Risks"
anchors = ["dispatcher/mod.rs:779,787", "model_call_service.rs:689-728", "orchestrate.rs:15268", "routes/feeds.rs:127", "roko-agent/src/lib.rs:22", "std::sync::Mutex", "Option::unwrap()", "parking_lot::Mutex"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Systemic audit finding (2026-04-28) with 4 open checklist fixes: S11.1-2 Switch to `parking_lot::Mutex` or handle; S11.3 Replace with `if let Some(ref client) = se; S11.4 Replace `.expect()` with `.ok_or()` + erro; S11.5 Remove crate-level lint suppression, fix i

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S11. Mutex Poisoning & Unwrap Risks`

How to verify: Check each open sub-item (S11.1-2, S11.3, S11.4, S11.5). S11.5 likely closed by P2-HYG-3 (blanket allows removed); S11.1-4 overlap P2-HYG-1 (.unwrap cleanup, deferred).
