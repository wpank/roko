+++
id = "find-82fd49"
kind = "finding"
title = "[provider F134] send_turn_streaming deprecated but still used in retry path"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F134"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F134"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs", "send_turn_streaming"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`send_turn_streaming_with_retry` calls the deprecated `send_turn_streaming` (channel-based API). The newer `stream_turn` is not used in this retry path.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F134`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Roadmap P3-4/AR-1 (streaming unification, TODO(082)) deferred. Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: `send_turn_streaming` deprecated but still used in retry path
