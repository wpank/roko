+++
id = "reg-06ae9f"
kind = "regression"
title = "Graph plan runs only write knowledge seeds; live ingestion and tier progression are not invoked"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-execution", "roko-neuro/tier-progression"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#tier-progression-after-live-knowledge-ingestion----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_execution/feedback.rs:482", "crates/roko-cli/src/graph_execution/plan_runner.rs:936", "crates/roko-cli/src/runtime_feedback/knowledge.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -rq 'NeuroKnowledgeIngestor' crates/roko-cli/src/graph_execution'''
+++

GAPS.md recorded as RESOLVED (2026-08-13) that the runner's `NeuroKnowledgeIngestor` records gate-backed feedback as confirmations, persists the candidate and runs `TierProgression::evaluate_tier_progression_v2`. On the Graph plan path, the feedback facade (`crates/roko-cli/src/graph_execution/plan_runner.rs:457-478`) has episode, routing-observation, dream-consolidation and daimon sinks but no knowledge ingestor. The knowledge row only appends seeds to `.roko/learn/knowledge-seeds.jsonl` (`crates/roko-cli/src/graph_execution/feedback.rs:482-511`), which `roko-learn` reads back for projections. `NeuroKnowledgeIngestor` is used only by `commands/do_cmd.rs` and `runtime_feedback/`. As a result, plan runs no longer promote knowledge from Transient to Working.

Fix: attach the knowledge ingestor sink, or an equivalent that runs tier progression, to the Graph feedback facade. Restore the live-sink promotion test on that path.

Re-verified 2026-09-29 at d9e79e9d8: the Graph feedback facade (plan_runner.rs:936-1000) still has no knowledge-ingestion sink. The regression is now wider: 725f21e05 removed NeuroKnowledgeIngestor from commands/do_cmd.rs, so it has no production caller at all (only runtime_feedback/knowledge.rs tests construct it), and no path runs TierProgression after live ingestion.
