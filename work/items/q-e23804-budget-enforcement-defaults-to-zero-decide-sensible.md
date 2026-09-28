+++
id = "q-e23804"
kind = "question"
title = "Budget enforcement defaults to zero (decide sensible defaults or document)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config-budget"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.5 Config Issues"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.5 Config Issues"
anchors = ["roko_core::config budget"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Roadmap §3.5: budget enforcement uses zero defaults; set sensible non-zero defaults or document zero as 'unlimited/disabled'.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.5 Config Issues`

How to verify: Check RokoConfig::default() budget fields and how zero is interpreted.
