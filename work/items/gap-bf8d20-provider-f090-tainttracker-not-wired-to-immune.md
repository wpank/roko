+++
id = "gap-bf8d20"
kind = "gap"
title = "[provider F090] TaintTracker not wired to immune boundary decisions"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/immune_boundary"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F090"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F090"
anchors = ["crates/roko-agent/src/immune_boundary.rs", "crates/roko-agent/src/safety/taint_propagation.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
When the immune boundary quarantines a provider output or tool result, it sets `immune_denied` tags on the denied signal but does not call `TaintTracker.observe_signal()` or `TaintTracker.mark_tainted()`. Downstream signals derived from quarantined content will not carry the inherited taint.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F090`
- `tmp/archive/provider-audit/20-safety-screening.md`

How to verify: E34 claims trust-origin IFC; check TaintTracker use in immune boundary decisions. Confirm in crates/roko-agent/src/immune_boundary.rs, crates/roko-agent/src/safety/taint_propagation.rs whether still true: TaintTracker not wired to immune boundary decisions
