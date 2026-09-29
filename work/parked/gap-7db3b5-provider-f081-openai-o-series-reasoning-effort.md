+++
id = "gap-7db3b5"
kind = "gap"
title = "[provider F081] OpenAI o-series reasoning_effort parameter not wired"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F081"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F081"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs", "reasoning_effort"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
No `reasoning_effort: "low"|"medium"|"high"` parameter is injected for OpenAI o-series models. Roko uses `use_max_completion_tokens` for o-series but does not control reasoning effort level.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F081`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-agent/src/provider/openai_compat.rs whether still true: OpenAI o-series `reasoning_effort` parameter not wired
