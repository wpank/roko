+++
id = "find-72b752"
kind = "finding"
title = "[provider F129] Gemini thinking_level string enum may not match stable API's thinkingBudget integer"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F129"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F129"
anchors = ["crates/roko-agent/src/gemini/native.rs", "thinking_level", "thinkingBudget"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roko sends `thinkingConfig.thinkingLevel` as a string ("minimal"/"low"/"medium"/"high"/"dynamic"). Some versions of the Gemini API may expect `thinkingBudget` as an integer token budget instead. The field name and type may vary by API version.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F129`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/gemini/native.rs whether still true: Gemini `thinking_level` string enum may not match stable API's `thinkingBudget` integer
