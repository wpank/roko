+++
id = "bug-883803"
kind = "bug"
title = "DF-0925 P2-2: cheap_agent() picks the alphabetically-first model, not the cheapest"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model"
anchors = ["graph_task_dispatch.rs:890 cheap_agent", "schema.rs:1021", "crates/roko-cli/src/graph_task_dispatch.rs::select_cheap_model_key", "crates/roko-cli/src/graph_task_dispatch.rs::cheap_agent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "triage check 2026-09-28"
evidence = "Fixed in 725f21e05: cheap_agent (graph_task_dispatch.rs:1296) now calls select_cheap_model_key (:128-175), which prefers routing.fast_task_model when it names a dispatchable model and otherwise takes the min by cost_input_per_m then cost_output_per_m (built-in pricing fallback, unpriced last), excluding embedding, tool-less and disabled-provider models. It no longer takes the first alphabetical slug. Unit tests are at :6338-6349. (Static check against 3d0ee4d02; tests not re-run.)"
+++
It takes the first of available_model_slugs_for_cascade (sorted alphabetically), resolving to claude-sonnet-4-6; should filter by cost_input_per_m or use routing.fast_task_model.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-2. `cheap_agent()` picks the most expensive model`

How to verify: Read cheap_agent selection logic.

Fixed in 725f21e05 (checked 2026-09-28).
