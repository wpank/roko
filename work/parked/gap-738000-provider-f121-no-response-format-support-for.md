+++
id = "gap-738000"
kind = "gap"
title = "[provider F121] No response_format support for OpenAI JSON mode"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/openai_compat_backend"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F121"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F121"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "response_format"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`TurnConfig` has no `response_format` field. Structured output / JSON mode for OpenAI (`response_format: { "type": "json_object" }`) cannot be requested.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F121`
- `tmp/archive/provider-audit/02-openai-compat.md`

How to verify: Confirm in crates/roko-agent/src/openai_compat_backend.rs whether still true: No `response_format` support for OpenAI JSON mode
