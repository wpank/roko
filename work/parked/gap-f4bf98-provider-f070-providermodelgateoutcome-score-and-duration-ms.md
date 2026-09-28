+++
id = "gap-f4bf98"
kind = "gap"
title = "[provider F070] ProviderModelGateOutcome.score and duration_ms always None"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/provider_outcome"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F070"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F070"
anchors = ["crates/roko-learn/src/provider_outcome.rs", "ProviderModelGateOutcome.score", "duration_ms", "None"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The outcome recorder sets `score: None` and `duration_ms: None` for all gate outcomes. These fields are defined in the struct but never populated, making them useless for analysis.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F070`

Warning: every file this item cites is gone (`crates/roko-learn/src/provider_outcome.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-learn/src/provider_outcome.rs whether still true: `ProviderModelGateOutcome.score` and `duration_ms` always `None`
