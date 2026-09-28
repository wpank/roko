+++
id = "bug-c30f28"
kind = "bug"
title = "[provider F159] Gemini cache read token pricing defaults to wrong rate"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F159"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F159"
anchors = ["crates/roko-agent/src/gemini/native.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`GeminiNativeAgent` sets `cost_cache_read_per_m: None`. The `fill_cost_from_pricing` helper defaults to `input_per_m * 0.1`. The actual Gemini 2.5 cache-read rate may differ from 10% of input price.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F159`
- `tmp/archive/provider-audit/19-caching.md`

How to verify: Confirm in crates/roko-agent/src/gemini/native.rs whether still true: Gemini cache read token pricing defaults to wrong rate
