+++
id = "find-87c357"
kind = "finding"
title = "[provider F127] Cascade router context vector sparse — 3 of 32 dimensions used"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/model_router"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F127"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F127"
anchors = ["crates/roko-learn/src/model_router.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The LinUCB context vector has 32 dimensions but only 3 are meaningfully populated in live dispatch: role hash (dim 0), latency (dim 1), and a bias constant (dim 31). The remaining 29 dimensions are zero or hardcoded constants.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F127`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Confirm in crates/roko-learn/src/model_router.rs whether still true: Cascade router context vector sparse — 3 of 32 dimensions used
