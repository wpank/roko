+++
id = "find-614753"
kind = "finding"
title = "[provider F089] Prompt-control language detection exempt for local/builtin tools"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/tool_immune"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F089"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F089"
anchors = ["crates/roko-agent/src/tool_immune.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`detect_tool_result_anomaly` only applies prompt-control language checking to `ToolSource::Mcp`, `ToolSource::Plugin`, `ToolSource::External`, and `ToolSource::Retrieval`. Local/builtin tools (bash, read_file, grep, glob) are exempt. A prompt injection via file content or bash output will not be...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F089`
- `tmp/archive/provider-audit/20-safety-screening.md`

How to verify: Confirm in crates/roko-agent/src/tool_immune.rs whether still true: Prompt-control language detection exempt for local/builtin tools
