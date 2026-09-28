+++
id = "gap-7a3527"
kind = "gap"
title = "Dead config keys that give false confidence"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-core/config"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["task_parser.rs:44 skip_enrichment", "config/gates.rs:147", "AdaptiveThresholds::from_gates_config", "config/agent.rs:73", "crates/roko-cli/src/task_parser.rs:45", "crates/roko-core/src/config/gates.rs:155", "crates/roko-core/src/config/agent.rs:81", "crates/roko-gate/src/adaptive_threshold.rs:322", "crates/roko-cli/src/graph_task_dispatch.rs::graph_engine_inert_settings"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
No read sites for skip_enrichment, gates.domain_gates, learning.replan_max_per_plan/replan_gate_attempts, agent.data_llm; adaptive gate keys only via uncalled AdaptiveThresholds::from_gates_config; most [routing] and [gates] keys are display-only or Runner-v2 only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`

How to verify: grep each key for read sites; wire or delete.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed in 725f21e05: [meta] skip_enrichment is now read by Graph dispatch (plan_skips_enrichment, crates/roko-cli/src/graph_task_dispatch.rs:1311-1330, used at :3061-3074). An uncommitted change also adds it to the plan fingerprint (crates/roko-graph/src/fingerprint.rs:24). Still dead: gates.domain_gates, learning.replan_max_per_plan, learning.replan_gate_attempts and agent.data_llm have no production reader. The same commit only lists them, together with the legacy-only [gates] keys and display-only [routing] keys, in graph_engine_inert_settings (graph_task_dispatch.rs:764-900), which warns once at dispatch (:957, :1161) and in `config doctor` (config_cmd.rs:254). AdaptiveThresholds::from_gates_config (crates/roko-gate/src/adaptive_threshold.rs:322) is still called only from its own tests module (:946 onward). Remaining work: wire or delete these keys; so far they are only warned about.
