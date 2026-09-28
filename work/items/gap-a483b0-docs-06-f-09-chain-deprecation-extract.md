+++
id = "gap-a483b0"
kind = "gap"
title = "DOCS-06 F / 09: Chain deprecation (extract 8 algorithms, remove blockchain code, routes, tools, deps)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-chain"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#Category F: Chain/Economy — DECISION MADE"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#Category F: Chain/Economy — DECISION MADE"
anchors = ["crates/roko-chain/", "phase2.rs", "mirage-rs", "chain feature flag"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
7 unchecked: extract reputation/arena/passport/TraceRank/collusion/knowledge/validation/Sybil modules; drop chain from default build; delete ~21 chain files incl. phase2.rs stubs; update serve routes; move relay config; update CLAUDE.md/GAPS; exclude mirage-rs.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#Category F: Chain/Economy — DECISION MADE`
- `tmp/docs-audit/09-CHAIN-DEPRECATION-PLAN.md#Phase 1: Extract Good Algorithms (~8 modules)`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-16: phase2.rs Simplified Types`
- `docs/v3/39-ROADMAP.md#4. Chain Deprecation`
- `tmp/docs-audit/09-CHAIN-DEPRECATION-PLAN.md`

How to verify: Check whether roko-chain still exists/default-built and which modules moved. / Check whether any Phase 1 extraction has begun (e.g. reputation in roko-core).

Merged 2 mined candidates: m4-051, m5-061.
