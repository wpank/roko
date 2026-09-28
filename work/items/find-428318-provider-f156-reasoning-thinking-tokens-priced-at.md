+++
id = "find-428318"
kind = "finding"
title = "[provider F156] Reasoning/thinking tokens priced at standard output rate"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gateway/cost_track"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F156"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F156"
anchors = ["crates/roko-gateway/src/cost_track.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Both `reasoning_tokens` (OpenAI) and `thinking_tokens` (Anthropic) are costed at the standard `output_per_m` rate. For providers that charge a premium for reasoning tokens, cost estimates will be incorrect.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F156`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-gateway/src/cost_track.rs whether still true: Reasoning/thinking tokens priced at standard output rate
