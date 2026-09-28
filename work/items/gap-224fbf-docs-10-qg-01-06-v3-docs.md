+++
id = "gap-224fbf"
kind = "gap"
title = "DOCS-10 QG-01..06: v3 docs pre-flight quality gates"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["docs/v3"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/10-V3-DEPTH-TODOS.md#Pre-Flight: Quality Gates (Do Before Starting Any Chapter)"
discovered_from = "audit:tmp/docs-audit/10-V3-DEPTH-TODOS.md#Pre-Flight: Quality Gates (Do Before Starting Any Chapter)"
anchors = ["docs/v3/", "docs/v3/depth/", "docs/v3/TEMPLATE.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unchecked pre-flight: docs/v3 structure with 39 depth subdirs, chapter and depth templates, canonical gist citation extraction, saved cargo doc output. docs/v3 has since been partly landed (vitepress dist present), so completion is unknown.

Imported without verification from:
- `tmp/docs-audit/10-V3-DEPTH-TODOS.md#Pre-Flight: Quality Gates (Do Before Starting Any Chapter)`
- `tmp/docs-audit/04-V3-STRUCTURE.md#Directory Structure`

Some cited files are gone: `docs/v3/TEMPLATE.md`.

How to verify: ls docs/v3 and docs/v3/depth; count depth subdirs vs 39.
