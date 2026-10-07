+++
id = "dec-55532d"
kind = "decision"
title = "Confirm nine worker choices from waves 3-5 (budget raise transport, failover retries, 24h auth quarantine, frozen allow-list, CLI billing, cold-start knowledge, domain packs, secret scrubbing)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "coordinator, gates 4c-5b (2026-10-03)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

Workers in waves 4 and 5 made choices that departed from a task's text or need Will's confirmation. All of them are
merged on main; each can be reverted or adjusted if Will disagrees.

## Why it matters

These change operator-visible behaviour or what counts as learned state.

## Where

See each point.

## Current state

1. **PK08 (2118), against decision 2117:** `roko plan budget raise` sends the raise over the plan-control socket,
   like pause and cancel since 1209, so it gets the run's answer. A `raise_budget` in control.json still works.
2. **PK02 (1116):** `provider_denied` (not retried) covers isolation_state_unavailable, invalid_agent_identity and
   auth failures; `agent_isolated` stays retryable because 1107/1109 give each attempt its own agent id.
3. **PK02 (1120):** an open circuit with nothing usable above still probes the routed model, as before.
4. **PK02 (1115):** an auth failure quarantines the provider for 24 h in provider-health.json, across runs, and no
   command clears it yet (gap-d90a93, p1).
5. **PK11 (2223):** the frozen-learning proof treats gate-failures.jsonl (read by `roko diagnose` and the plan's own
   revision) and provider-health.json (the circuit breaker) as non-learned state, so a frozen run may still write
   them.
6. **PK13 (2115):** `[providers.x] billing = "subscription" | "metered"` exists, but roko.toml sets none, so CLI
   attempts record `billed_usd` as unknown. Should the Claude and Codex CLI providers be marked `subscription`?
7. **PK33 (fix in gate 5a):** in cold start (under 50 observations) stored knowledge advice alone now ranks the
   router's candidates, so knowledge can move a new workspace's model choice.
8. **PK74 (9120):** a task of a non-code domain with no `[gates.packs]` entry no longer runs `[[gates.rungs]]`; a
   malformed rung now fails config load.
9. **PK05 (1212, wave 3):** provider CLIs and MCP servers lose every secret-looking variable; Claude Code or Gemini on
   Bedrock/Vertex need `[agent] env_passthrough` (for example `["AWS_*"]`).

## Plan

Will confirms or adjusts each point; adjustments become items.

## Done when

Each point has Will's answer recorded here.

## Notes

- Raised by the coordinator after gate 5b, 2026-10-03.
