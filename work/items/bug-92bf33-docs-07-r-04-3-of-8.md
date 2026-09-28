+++
id = "bug-92bf33"
kind = "bug"
title = "DOCS-07 R-04: 3 of 8 graph examples use wrong condition types (P0-GS-1)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/examples"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#R-04: Graph Example Schema Migration"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#R-04: Graph Example Schema Migration"
anchors = ["examples/graphs/", "roko graph validate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Three graph example definitions use outdated condition types and do not work as shipped.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#R-04: Graph Example Schema Migration`
- `tmp/dogfood/2026-09-20-final-session.md#P0 (Critical)`

A source claims this was fixed; confirm against current code before closing.

How to verify: Run `roko graph validate` over every shipped example.
