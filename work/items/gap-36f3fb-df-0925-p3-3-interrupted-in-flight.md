+++
id = "gap-36f3fb"
kind = "gap"
title = "DF-0925 P3-3: Interrupted in-flight task side effects are unrecorded"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/checkpoint"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned"
anchors = ["activities.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
A task that wrote files but had not reached the Ok arm leaves on-disk changes with no activity record, so workspace and checkpoint diverge on interrupt.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-3. In-flight work is orphaned`

How to verify: Interrupt mid-task and compare git diff with checkpoint.
