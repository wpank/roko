+++
id = "gap-6fd380"
kind = "gap"
title = "[provider F155] InferenceMeta lacks thinking_level field — thinking-aware routing dead at gateway level"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gateway/types"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F155"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F155"
anchors = ["crates/roko-gateway/src/types.rs", "InferenceMeta", "thinking_level"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`RoutingContext.thinking_level` is always `None` in the gateway dispatch path because `InferenceMeta` has no `thinking_level` field to populate from.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F155`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-gateway/src/types.rs whether still true: `InferenceMeta` lacks `thinking_level` field — thinking-aware routing dead at gateway level
