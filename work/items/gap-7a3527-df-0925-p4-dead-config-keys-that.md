+++
id = "gap-7a3527"
kind = "gap"
title = "DF-0925 P4: Dead config keys that give false confidence"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["task_parser.rs:44 skip_enrichment", "config/gates.rs:147", "AdaptiveThresholds::from_gates_config", "config/agent.rs:73"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No read sites for skip_enrichment, gates.domain_gates, learning.replan_max_per_plan/replan_gate_attempts, agent.data_llm; adaptive gate keys only via uncalled AdaptiveThresholds::from_gates_config; most [routing] and [gates] keys are display-only or Runner-v2 only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`

How to verify: grep each key for read sites; wire or delete.
