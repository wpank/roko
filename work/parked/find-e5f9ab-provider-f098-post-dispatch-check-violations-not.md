+++
id = "find-e5f9ab"
kind = "finding"
title = "[provider F098] post_dispatch_check violations not automatically blocking"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F098"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F098"
anchors = ["crates/roko-agent/src/safety/", "post_dispatch_check"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Safety `post_dispatch_check` violations are recorded but do not automatically block subsequent dispatches from the same provider/agent. A provider that produces safety violations remains available for dispatch.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F098`

How to verify: Confirm in crates/roko-agent/src/safety/ whether still true: `post_dispatch_check` violations not automatically blocking
