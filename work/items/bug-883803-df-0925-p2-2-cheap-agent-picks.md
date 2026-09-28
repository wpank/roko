+++
id = "bug-883803"
kind = "bug"
title = "DF-0925 P2-2: cheap_agent() picks the alphabetically-first model, not the cheapest"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model"
anchors = ["graph_task_dispatch.rs:890 cheap_agent", "schema.rs:1021"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
It takes the first of available_model_slugs_for_cascade (sorted alphabetically), resolving to claude-sonnet-4-6; should filter by cost_input_per_m or use routing.fast_task_model.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model`

How to verify: Read cheap_agent selection logic.
