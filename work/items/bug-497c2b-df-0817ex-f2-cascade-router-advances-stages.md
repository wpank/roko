+++
id = "bug-497c2b"
kind = "bug"
title = "DF-0817ex F2: Cascade router advances stages on a global counter, not per-model readiness"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/cascade-router"]
created = 2026-08-17
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#F. Learning / Knowledge Subsystems (7 findings)"
discovered_from = "audit:tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#F. Learning / Knowledge Subsystems (7 findings)"
anchors = ["cascade_router.rs confidence_scores"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Router moved to the confidence stage from a global observation count while only 3 of many models had confidence data.

Imported without verification from:
- `tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#F. Learning / Knowledge Subsystems (7 findings)`

How to verify: Read stage-transition logic.
