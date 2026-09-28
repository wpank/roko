+++
id = "find-19a5e3"
kind = "finding"
title = "[provider F180] Two parallel ACP pipeline implementations coexist"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F180"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F180"
anchors = ["crates/roko-acp/src/bridge_events.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
A `ROKO_ACP_LEGACY` environment variable gates between two parallel ACP pipeline implementations. Both exist in production code. The legacy path is enabled by default when the variable is set.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F180`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-acp/src/bridge_events.rs whether still true: Two parallel ACP pipeline implementations coexist
