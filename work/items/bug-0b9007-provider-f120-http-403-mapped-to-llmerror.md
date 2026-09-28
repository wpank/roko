+++
id = "bug-0b9007"
kind = "bug"
title = "[provider F120] HTTP 403 mapped to LlmError::Network not AuthFailure"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/openai_compat_backend"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F120"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F120"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "LlmError::Network", "AuthFailure"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`classify_http_error` maps 403 to `LlmError::Network` in the generic path. The correct mapping is `ProviderError::AuthFailure`. This prevents the auth failure retry suppression from applying.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F120`
- `tmp/archive/provider-audit/02-openai-compat.md`

How to verify: P3-1 centralization listed anthropic_api/cursor_acp/cerebras, not openai_compat; check 403 mapping there. Confirm in crates/roko-agent/src/openai_compat_backend.rs whether still true: HTTP 403 mapped to `LlmError::Network` not `AuthFailure`
