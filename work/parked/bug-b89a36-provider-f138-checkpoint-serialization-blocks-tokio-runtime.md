+++
id = "bug-b89a36"
kind = "bug"
title = "[provider F138] Checkpoint serialization blocks Tokio runtime thread"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F138"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F138"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Tool loop checkpoint writes use blocking `std::fs::File` operations on the Tokio async runtime without wrapping in `spawn_blocking`. This blocks the Tokio thread pool during checkpoint writes.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F138`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: Checkpoint serialization blocks Tokio runtime thread
