+++
id = "find-19dbb1"
kind = "finding"
title = "[provider F072] CPU-bound/blocking sync handlers not interruptible by timeout"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F072"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F072"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Tool handlers implemented as synchronous blocking functions are not interruptible by `tokio::time::timeout`. A CPU-bound handler that takes longer than the timeout will continue running even after the deadline fires.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F072`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: CPU-bound/blocking sync handlers not interruptible by timeout
