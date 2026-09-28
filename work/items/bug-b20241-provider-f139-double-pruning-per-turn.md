+++
id = "bug-b20241"
kind = "bug"
title = "[provider F139] Double pruning per turn"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F139"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F139"
anchors = ["crates/roko-agent/src/tool_loop/prune.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The context pruning logic fires twice per turn in some code paths, performing redundant truncation work.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F139`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/prune.rs whether still true: Double pruning per turn
