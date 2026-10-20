+++
id = "gap-7576c7"
kind = "gap"
title = "Knowledge a Graph run writes (admitted lessons, reinforcements, promotions) publishes no event"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/runtime_feedback"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
last_verified_rev = "ea7d6e7bb"
source = "roko-loops worker for the hermes-plugin-roko live UI, 2026-10-14 (roko main 216f88c84)"
anchors = ["crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink", "crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn verified_knowledge_publishes_what_it_wrote' crates/roko-cli/ && cargo test -p roko-cli verified_knowledge_publishes_what_it_wrote"

[closed]
at = 2026-10-14
at_ts = "2026-10-14T09:55:27Z"
commit = "ea7d6e7bb"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-14T08:26:48Z"
model = "claude-opus-5-5"
forced = false
evidence = "ea7d6e7bb: VerifiedKnowledgeSink::with_publisher publishes knowledge_written (admission, entry_id, reinforced, promoted, lesson) when an ingestion changed something; the plan runner passes the run's hub through build_graph_feedback_context/facade (verified_knowledge.rs, plan_runner.rs). Verify: cargo test -p roko-cli verified_knowledge_publishes_what_it_wrote passes. Checked at the top of the stacked branches (all six roko-loops items): cargo test -p roko-cli --lib passed except the date-keyed active_self_model_picks_start_rung_and_logs_propensity (fails on 2026-10-14 without these changes too), learning_wiring_census passed, clippy -D warnings clean on roko-core, roko-cli and roko-serve; a zero-cost roko serve smoke (fake Claude CLI) showed the event in the run's index."
+++

## Problem

When a Graph attempt passes its verify steps, the verified-knowledge sink admits its stated lesson into the knowledge
store (or confirms a stored one), reinforces the entries its prompt surfaced, and promotes tiers. None of this is
published; the only trace is `.roko/neuro/knowledge-lifecycle.jsonl`.

## Why it matters

This is loop L3 writing what a run taught it. A host following the run cannot show "roko learned: …" as it happens
(goal `visibility`).

## Where

- `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink::on_event`: the ingestion and its
  `KnowledgeLifecycleRecord`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade` (and
  `build_graph_feedback_context`): where the sink is built; the run's hub is resolved in the same function.

## Current state

`DashboardEvent::KnowledgeEntriesUpdated` carries browse entries for the TUI's knowledge tab and has no Graph caller;
it does not say what one attempt wrote.

## Plan

1. Add `DashboardEvent::KnowledgeWritten {plan_id, task_id, admission, entry_id, reinforced, promoted, lesson}`.
2. Give the sink an optional hub sender; after an ingestion that admitted, confirmed, reinforced or promoted anything,
   publish the event (`lesson` cut to 160 characters).
3. Build the run's sink with the run's hub.

## Done when

`[[verify]]` passes: a verified attempt's ingestion publishes what it wrote.
