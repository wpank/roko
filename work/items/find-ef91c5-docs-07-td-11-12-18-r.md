+++
id = "find-ef91c5"
kind = "finding"
title = "DOCS-07 TD-11,12,18 / R-03: Crate structure (roko-core/roko-agent size, provider feature gates, conductor fold-in)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-12: roko-agent Size (107,778 LOC, 182 files)"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-12: roko-agent Size (107,778 LOC, 182 files)"
anchors = ["crates/roko-agent/Cargo.toml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-core (~93K LOC) and roko-agent (~108K LOC, 12 providers) are very large; proposals: feature-gate each provider, consider folding loosely coupled roko-conductor. Low priority.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-12: roko-agent Size (107,778 LOC, 182 files)`
- `tmp/docs-audit/07-TECH-DEBT.md#R-03: Provider Feature Gates`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-18: `roko-conductor` Loose Coupling`

How to verify: Check roko-agent features for per-provider gates.
