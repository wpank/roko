+++
id = "gap-31a619"
kind = "gap"
title = "Structured JSON Output for Reviewer Agents"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/197-structured-reviewer-json.md#197 — Structured JSON Output for Reviewer Agents"
discovered_from = "audit:tmp/backlog/archive/197-structured-reviewer-json.md#197 — Structured JSON Output for Reviewer Agents"
anchors = ["crates/roko-cli/src/dispatch/mod.rs", "event_loop.rs", "roko-compose/src/schemas.rs", "crates/roko-compose/src/schemas.rs", "crates/roko-compose/src/lib.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-cli/src/runner/event_loop.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
reviewer agent output is unstructured text that the gate handler must heuristically parse; structured JSON would make gate verdicts deterministic. When runner-v2 dispatches a reviewer agent (e.g. for code review gates), the agent returns unstructured text. The gate handler in `event_loop.rs` must…

Imported without verification from:
- `tmp/backlog/archive/197-structured-reviewer-json.md#197 — Structured JSON Output for Reviewer Agents`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-compose/src/schemas.rs`, `roko-compose/src/schemas.rs`.

How to verify: Check: Reviewer agents receive a JSON schema constraint when dispatched; Agent responses for reviews are valid JSON matching the schema (when provider supports it); Gate handler parses structured review output without heuristic text matching [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
