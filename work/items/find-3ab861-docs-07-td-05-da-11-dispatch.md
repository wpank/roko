+++
id = "find-3ab861"
kind = "finding"
title = "DOCS-07 TD-05 / DA-11: Dispatch, model-resolution, config-loader and doctor path proliferation"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/dispatch"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-05: Agent Dispatch Proliferation"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-05: Agent Dispatch Proliferation"
anchors = ["crates/roko-agent/src/dispatcher/mod.rs", "RuntimeServices"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
5-7 divergent agent dispatch paths with different safety/enrichment; engine audit counted 11 model-resolution functions, 9 config loaders and 4 doctor implementations.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-05: Agent Dispatch Proliferation`
- `tmp/dev-audit/11-implementation-status.md#Dev-audit items with new context from the audits`
- `tmp/dogfood/2026-09-19-session.md#Next Steps`
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Open findings confirmed or elaborated by the new audits`

How to verify: Enumerate dispatch entry points and check they share RuntimeServices safety/enrichment.
