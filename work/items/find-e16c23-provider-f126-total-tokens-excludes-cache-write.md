+++
id = "find-e16c23"
kind = "finding"
title = "[provider F126] total_tokens() excludes cache_write and reasoning tokens"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/chat_types"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F126"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F126"
anchors = ["crates/roko-core/src/chat_types.rs", "total_tokens()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`Usage::total_tokens()` sums input + output + cache_read. Cache write tokens and reasoning tokens are excluded. Total token counts reported in dashboards and telemetry undercount actual tokens processed.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F126`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Confirm in crates/roko-core/src/chat_types.rs whether still true: `total_tokens()` excludes cache_write and reasoning tokens
