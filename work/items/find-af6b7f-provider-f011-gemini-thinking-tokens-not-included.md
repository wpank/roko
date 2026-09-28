+++
id = "find-af6b7f"
kind = "finding"
title = "[provider F011] Gemini thinking tokens not included in UsageObservation.output_tokens"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011"
anchors = ["crates/roko-agent/src/gemini/types.rs:245", "crates/roko-agent/src/gemini/native.rs::gemini_observation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'thoughtsTokenCount|thoughts_token_count' crates/roko-agent/src/gemini/types.rs"
+++
Gemini 2.5+ models return `thinkingTokenCount` in usage metadata. This field is not extracted or added to `UsageObservation.output_tokens`. Thinking token costs are therefore not reflected in cost accounting for Gemini models that perform extended thinking.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F011`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/gemini/native.rs whether still true: Gemini thinking tokens not included in `UsageObservation.output_tokens`

Verified 2026-09-28: still true, and worse than reported. `UsageMetadata` (crates/roko-agent/src/gemini/types.rs:245) deserializes `thinking_token_count` as camelCase `thinkingTokenCount`, but the Gemini API reports `thoughtsTokenCount`, so the field is never populated from live responses. Only fixtures use the made-up name (types.rs:345, native.rs:882, tests/gemini_integration.rs:488). `gemini_observation` (native.rs:560-584) sets `output_tokens = candidates_token_count` and `reasoning_tokens: None`, so thinking tokens are missing from cost accounting.
