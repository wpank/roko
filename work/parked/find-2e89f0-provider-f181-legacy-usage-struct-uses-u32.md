+++
id = "find-2e89f0"
kind = "finding"
title = "[provider F181] Legacy Usage struct uses u32 for token counts and f32 for cost"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/chat_types"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F181"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F181"
anchors = ["crates/roko-core/src/chat_types.rs", "Usage", "u32", "f32"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The legacy `Usage` struct (not the `TokenUsage` or `UsageObservation` types) uses `u32` for token counts (overflow at ~4B tokens) and `f32` for cost (precision loss after ~$16M). High-volume or high-cost runs may encounter silent overflow or precision loss.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F181`

How to verify: Confirm in crates/roko-core/src/chat_types.rs whether still true: Legacy `Usage` struct uses `u32` for token counts and `f32` for cost
