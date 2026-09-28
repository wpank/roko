+++
id = "find-b12176"
kind = "finding"
title = "[provider F076] Serve/HTTP routes use DaimonPolicy::default() instead of live affect state"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F076"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F076"
anchors = ["crates/roko-serve/src/routes/providers.rs", "crates/roko-serve/src/routes/gateway.rs", "DaimonPolicy::default()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`providers.rs` and `gateway.rs` in roko-serve construct `RoutingContext` with `DaimonPolicy::default()` (neutral state). The live affect state loaded by runner-v2 is not used for HTTP-triggered routing decisions.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F076`
- `tmp/archive/provider-audit/16-daimon-affect.md`

How to verify: Confirm in crates/roko-serve/src/routes/providers.rs, crates/roko-serve/src/routes/gateway.rs whether still true: Serve/HTTP routes use `DaimonPolicy::default()` instead of live affect state
