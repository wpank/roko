+++
id = "gap-b95246"
kind = "gap"
title = "Superseded: Agent Dispatch Consolidation"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/61-dispatch-consolidation.md#61 — Superseded: Agent Dispatch Consolidation"
discovered_from = "audit:tmp/backlog/archive/61-dispatch-consolidation.md#61 — Superseded: Agent Dispatch Consolidation"
anchors = ["crates/roko-cli/src/runner/", "crates/roko-cli/src/dispatch/", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-acp/src/runner.rs", "crates/roko-serve/src/dispatch.rs", "crates/roko-graph/src/cells/", "crates/roko-cli/src/runner/event_loop.rs:8602", "crates/roko-cli/src/dispatch/prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
architecture debt; every new cross-cutting feature must be wired N times. Roko has five independent implementations of the "pick a model, assemble a prompt, call a provider, run safety checks, record the outcome" pattern. These grew in parallel and each integrates a different subset of the…

Imported without verification from:
- `tmp/backlog/archive/61-dispatch-consolidation.md#61 — Superseded: Agent Dispatch Consolidation`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-RUN-1 (Subsystem: Runner / Dispatch)`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A `DispatchPipeline` type exists with optional stage composition for model routing, prompt enrichment, safety, episode recording, cascade feedback, playbook feedback, and affect.; `TemplateAgentDispatcher` (serve) constructs a… [evidence: own status: Superseded by #243-#247, #253, and #274, which split shared services/contracts from host adapters; CONSOLIDATED P2-RUN-1: open; 00-STATUS-SUMMARY 1. Impleme…]
