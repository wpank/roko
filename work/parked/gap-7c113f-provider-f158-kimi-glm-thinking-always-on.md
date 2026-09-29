+++
id = "gap-7c113f"
kind = "gap"
title = "[provider F158] Kimi/GLM thinking always-on for supports_thinking = true models; no per-request opt-out"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F158"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F158"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs", "supports_thinking = true"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`inject_kimi_params` and `inject_glm_params` add thinking body parameters unconditionally for models where `supports_thinking = true`. There is no per-request mechanism to suppress thinking for a model that normally supports it.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F158`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-agent/src/provider/openai_compat.rs whether still true: Kimi/GLM thinking always-on for `supports_thinking = true` models; no per-request opt-out
