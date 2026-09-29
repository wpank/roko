+++
id = "bug-6f1685"
kind = "bug"
title = "[provider F069] Failures never update LinUCB A/b matrices; only confidence stats updated"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/model_router"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F069"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F069"
anchors = ["crates/roko-learn/src/model_router.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When a routing outcome is a failure, the code path updates the empirical confidence stats (Stage 2) but does not call the LinUCB parameter update (`A <- A + x*x^T`, `b <- b + reward*x`). The bandit does not learn from failures in Stage 3.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F069`
- `tmp/archive/provider-audit/08-cascade-router.md`

How to verify: Confirm in crates/roko-learn/src/model_router.rs whether still true: Failures never update LinUCB A/b matrices; only confidence stats updated
