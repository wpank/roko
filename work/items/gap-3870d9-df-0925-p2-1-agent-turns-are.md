+++
id = "gap-3870d9"
kind = "gap"
title = "DF-0925 P2-1: Agent turns are unbounded for non-express tasks"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns"
anchors = ["graph_task_dispatch.rs:1913", "EXPRESS_MAX_TURNS", "is_express_task", "crates/roko-cli/src/graph_task_dispatch.rs::task_turn_limit", "crates/roko-core/src/config/gates.rs::max_turns_for_tier"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "triage check 2026-09-28"
evidence = "Fixed in 725f21e05: task_turn_limit (graph_task_dispatch.rs:679-686) gives every task its tier's [pipeline.<tier>] max_turns via PipelineConfig::max_turns_for_tier (roko-core/src/config/gates.rs:543). The defaults are mechanical 40, focused 60, integrative 90, architectural 120 (gates.rs:340-343), and unknown tiers use the focused band. Express tasks are lowered to EXPRESS_MAX_TURNS=5 (:654). Both dispatch requests now pass max_turns: Some(max_turns) (graph_task_dispatch.rs:3432, :3864) instead of None. (Static check against 3d0ee4d02; tests not re-run.)"
+++
max_turns is None unless tier is mechanical/trivial; no config key caps turns for other tiers (one task ran 312s with 305K cache-read tokens).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns`

How to verify: Check max_turns derivation and config keys.

Fixed in 725f21e05 (checked 2026-09-28).
