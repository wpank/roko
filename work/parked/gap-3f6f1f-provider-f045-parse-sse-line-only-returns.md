+++
id = "gap-3f6f1f"
kind = "gap"
title = "[provider F045] parse_sse_line only returns first tool call from multi-tool SSE chunk"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/streaming"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F045"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F045"
anchors = ["crates/roko-agent/src/streaming.rs", "parse_sse_line"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When an SSE chunk contains `tool_calls` with multiple items in the array, `parse_sse_line` iterates the array but returns immediately after the first item. Subsequent tool calls in the same chunk are silently discarded.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F045`
- `tmp/archive/provider-audit/02-openai-compat.md`

How to verify: Roadmap P0-7 fixed empty-string key accumulation; multi-tool parse_sse_line return not explicitly addressed. Confirm in crates/roko-agent/src/streaming.rs whether still true: `parse_sse_line` only returns first tool call from multi-tool SSE chunk
