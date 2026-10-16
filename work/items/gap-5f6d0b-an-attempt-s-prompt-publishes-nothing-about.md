+++
id = "gap-5f6d0b"
kind = "gap"
title = "An attempt's prompt publishes nothing about the lessons, playbooks and episodes it carried or left out, so no host can show a kept lesson being used"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v6/findings/learning-research.md §4 item 7; design/spec-v3.md §10 R3 (learning-research, 2026-10-16)"
discovered_from = "hermes-roko-v6 round: make learning visible on the hub"
anchors = ["crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_planned_attempt", "crates/roko-cli/src/dispatch/prompt_builder.rs::PromptDiagnostics", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_attempt_publishes_what_its_prompt_carried' crates/roko-cli && cargo test -p roko-cli --lib an_attempt_publishes_what_its_prompt_carried"
+++

## Problem

Nothing on the hub says which stored lessons, playbooks and earlier episodes an attempt's prompt used, or which it
retrieved and left out. `PromptDiagnostics` (`dispatch/prompt_builder.rs`) holds every retrieved item with whether it
reached the prompt and why not (`withheld_arm`, `bandit_excluded`, `token_budget`, …), and `exposures.jsonl` keeps it
per run, but no event carries it. So a host can show "kept a new lesson" (`knowledge_written`) and never "used 2
lessons" or "the audit's control arm withheld the lesson": the causal link from a kept lesson to its use, the core of
"it gets better the more you use it", is invisible live (learning-research §4 item 7, spec-v3 R3).

## Why it matters

Goal `hermes`: the Hermes plugin's Learning view and the route sheet (spec-v3 P5/P6, D6/D7) need it; it is the
evidence a viewer sees for the learning loop. In learning-research's showcase the section bandit dropped
`domain_context` for a whole chain (3.2) and nobody could see why the lesson did not reach the prompt.

## Where

- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_planned_attempt`: where an attempt's route row is
  published (`route_decided`) and its exposures written.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::PromptDiagnostics`: `items`, `included_sections`,
  `estimated_tokens`.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`: the hub's events.

## Plan

1. `DashboardEvent::PromptAssembled` (`prompt_assembled`): `plan_id`, `task_id`, `attempt` (from 1),
   `knowledge_ids`, `playbook_ids`, `episode_ids` (what the prompt carried), `left_out` (`[{kind, id, reason}]` for
   retrieved knowledge entries, playbooks, episodes and sections that did not reach it), `included_sections`,
   `dropped_sections`, `estimated_tokens`.
2. Publish it from `record_planned_attempt`, after `route_decided`.
3. serve: name it in `/api/events` filters; `/health` feature `prompt_events`; docs/v3/26-HTTP-API.md.

## Done when

The `[[verify]]` test passes: a dispatched attempt whose prompt carries a stored lesson publishes `prompt_assembled`
naming it, after its `route_decided`.
