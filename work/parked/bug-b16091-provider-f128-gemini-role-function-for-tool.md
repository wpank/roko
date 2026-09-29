+++
id = "bug-b16091"
kind = "bug"
title = "[provider F128] Gemini role: \"function\" for tool results is deprecated API format"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F128"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F128"
anchors = ["crates/roko-agent/src/translate/gemini.rs", "role: \"function\""]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The Gemini translator uses `role: "function"` for tool result messages. This was the old format; the current Gemini API uses `role: "user"` with `functionResponse` parts.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F128`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/translate/gemini.rs whether still true: Gemini `role: "function"` for tool results is deprecated API format
