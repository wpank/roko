+++
id = "gap-dce6d3"
kind = "gap"
title = "[provider F060] Gemini tool_config (function calling mode) never populated"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/gemini"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F060"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F060"
anchors = ["crates/roko-agent/src/gemini/types.rs", "tool_config"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`GenerateContentRequest.tool_config` controls Gemini's function calling mode (AUTO/ANY/NONE). This field is never set in any Gemini request construction path. Gemini always uses its default function calling mode.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F060`
- `tmp/archive/provider-audit/05-gemini-api.md`

How to verify: Confirm in crates/roko-agent/src/gemini/types.rs whether still true: Gemini `tool_config` (function calling mode) never populated
