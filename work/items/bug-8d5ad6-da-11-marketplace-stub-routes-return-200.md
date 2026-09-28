+++
id = "bug-8d5ad6"
kind = "bug"
title = "DA-11: Marketplace stub routes return 200/201 instead of 501"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/dev-audit/11-implementation-status.md#New risks surfaced by the audits that affect dev-audit scope"
discovered_from = "audit:tmp/dev-audit/11-implementation-status.md#New risks surfaced by the audits that affect dev-audit scope"
anchors = ["crates/roko-serve/src/routes/", "crates/roko-serve/src/routes/marketplace.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
CLI audit critical finding #6: marketplace stubs return success codes, so evidence scoring counts them green; the chain deprecation plan instead proposes removing marketplace/DeFi stub routes entirely.

Imported without verification from:
- `tmp/dev-audit/11-implementation-status.md#New risks surfaced by the audits that affect dev-audit scope`
- `tmp/docs-audit/09-CHAIN-DEPRECATION-PLAN.md#Summary of Decisions`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit (`tmp/cli-audit/`, 34 files)`
- `tmp/cli-audit/SUMMARY.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: curl marketplace routes on a local serve and check status codes. / curl marketplace routes on a local roko serve.

Merged 2 mined candidates: m4-013, m5-143.
