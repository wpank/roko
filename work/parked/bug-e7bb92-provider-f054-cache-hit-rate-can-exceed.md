+++
id = "bug-e7bb92"
kind = "bug"
title = "[provider F054] cache_hit_rate can exceed 1.0 (not clamped)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/efficiency"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F054"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F054"
anchors = ["crates/roko-learn/src/efficiency.rs", "cache_hit_rate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`EfficiencyEvent.cache_hit_rate()` computes `cache_read_tokens / input_tokens`. If `cache_read_tokens > input_tokens` (possible due to token counting inconsistencies), the ratio exceeds 1.0. No clamp is applied.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F054`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Confirm in crates/roko-learn/src/efficiency.rs whether still true: `cache_hit_rate` can exceed 1.0 (not clamped)
