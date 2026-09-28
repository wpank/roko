+++
id = "gap-7ac14d"
kind = "gap"
title = "DOCS-05/10 Phase 1: v3 core chapters (00-INDEX, 01-SIGNAL, 02-CELL, 03-GRAPH, 29-HEARTBEAT, 30-CONDUCTOR, 35-ARCH, REFERENCES)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["docs/v3"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/05-V3-CHAPTER-CHECKLIST.md#Phase 1: Core Primitives (8 chapters, ~50 depth files)"
discovered_from = "audit:tmp/docs-audit/05-V3-CHAPTER-CHECKLIST.md#Phase 1: Core Primitives (8 chapters, ~50 depth files)"
anchors = ["docs/v3/00-INDEX.md", "docs/v3/01-SIGNAL.md", "docs/v3/depth/00-architecture/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
8 chapters and ~50 depth files unchecked (30 for 00-architecture), incl. the Signal/Engram direction fix and verifying 'everything is a Graph of Cells'; ~37K v1 lines to preserve.

Imported without verification from:
- `tmp/docs-audit/05-V3-CHAPTER-CHECKLIST.md#Phase 1: Core Primitives (8 chapters, ~50 depth files)`
- `tmp/docs-audit/10-V3-DEPTH-TODOS.md#Chapter 00: INDEX + Architecture Foundations`
- `tmp/docs-audit/10-V3-DEPTH-TODOS.md#Chapter 03: Graph Engine`

How to verify: Compare existing docs/v3 chapter/depth files and line counts against the checklist.
