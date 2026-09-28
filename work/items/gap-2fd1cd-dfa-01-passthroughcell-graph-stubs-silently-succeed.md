+++
id = "gap-2fd1cd"
kind = "gap"
title = "DFA-01: PassthroughCell graph stubs silently succeed"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/cells"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
anchors = ["PassthroughCell", "is_stub"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Engine audit found 7 PassthroughCell stubs that pass input unchanged without the [STUB] warning path (cognitive-loop ran 7 no-op cells), which can mask missing Cell implementations during plan execution.

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`
- `tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#C. Graph Schema (5 findings)`

How to verify: grep PassthroughCell registrations and whether they report is_stub().
