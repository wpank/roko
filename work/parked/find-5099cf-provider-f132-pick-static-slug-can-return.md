+++
id = "find-5099cf"
kind = "finding"
title = "[provider F132] pick_static_slug can return unconfigured model slugs"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F132"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F132"
anchors = ["crates/roko-learn/src/cascade_router.rs", "pick_static_slug"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The static routing table may contain model slugs that are not in the active `roko.toml` configuration. Selecting such a slug results in an agent creation error.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F132`
- `tmp/archive/provider-audit/08-cascade-router.md`

How to verify: Confirm in crates/roko-learn/src/cascade_router.rs whether still true: `pick_static_slug` can return unconfigured model slugs
