+++
id = "find-5e6837"
kind = "finding"
title = "[provider F133] Context overflow fallback selection is order-dependent"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F133"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F133"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When the primary model hits context overflow, the fallback selection iterates the candidate list and picks the first model with a smaller context window. The ordering of the candidate list determines which fallback is chosen.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F133`
- `tmp/archive/provider-audit/08-cascade-router.md`

How to verify: Confirm in crates/roko-learn/src/cascade_router.rs whether still true: Context overflow fallback selection is order-dependent
