+++
id = "gap-3870d9"
kind = "gap"
title = "DF-0925 P2-1: Agent turns are unbounded for non-express tasks"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns"
anchors = ["graph_task_dispatch.rs:1913", "EXPRESS_MAX_TURNS", "is_express_task"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
max_turns is None unless tier is mechanical/trivial; no config key caps turns for other tiers (one task ran 312s with 305K cache-read tokens).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-1. Unbounded agent turns`

How to verify: Check max_turns derivation and config keys.
