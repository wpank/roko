+++
id = "bug-dddba3"
kind = "bug"
title = "[provider F057] Gemini SAFETY finish reason mapped to FinishReason::Error(\"safety\") not ContentFilter"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F057"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F057"
anchors = ["crates/roko-agent/src/gemini/", "SAFETY", "FinishReason::Error(\"safety\")", "ContentFilter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
When Gemini returns `finishReason: SAFETY`, the adapter maps this to `FinishReason::Error("safety")`. The correct mapping is `FinishReason::ContentFilter` or a dedicated content policy error. This prevents the retry layer from recognizing content policy violations.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F057`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/gemini/ whether still true: Gemini `SAFETY` finish reason mapped to `FinishReason::Error("safety")` not `ContentFilter`
