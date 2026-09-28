+++
id = "find-4ba227"
kind = "finding"
title = "[provider F050] ACP injects experiment context as ephemeral text, not canonical section replacement"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F050"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F050"
anchors = ["crates/roko-acp/src/bridge_events.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The ACP pipeline injects prompt experiment context as freeform text rather than using the canonical section-replacement mechanism. On crash and resume, the experiment context may be double-counted. Experiment attribution is weaker (no blake3 hash of the exact prompt pair).

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F050`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: CLAUDE.md still lists ACP/serve experiment injection as lacking canonical-section parity. Confirm in crates/roko-acp/src/bridge_events.rs whether still true: ACP injects experiment context as ephemeral text, not canonical section replacement
