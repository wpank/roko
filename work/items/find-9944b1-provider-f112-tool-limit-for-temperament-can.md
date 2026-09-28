+++
id = "find-9944b1"
kind = "finding"
title = "[provider F112] tool_limit_for_temperament can silently drop tools"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F112"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F112"
anchors = ["crates/roko-agent/src/provider/mod.rs", "tool_limit_for_temperament"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The tool count limit derived from temperament may reduce the advertised tool list below the number of tools the agent needs. No warning is emitted when tools are dropped.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F112`
- `tmp/archive/provider-audit/01-provider-adapters.md`

How to verify: Confirm in crates/roko-agent/src/provider/mod.rs whether still true: `tool_limit_for_temperament` can silently drop tools
