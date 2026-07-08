# Agent Stack Summary — Audited Scope

For agents working on `tmp/docs-parity/02`.

This pack is for auditing and tightening docs around the current agent stack. It is not permission to widen batch `02` into open-ended runtime work.

## Core Split

`roko-agent` owns the live agent/runtime surfaces: provider adapters, translators, tool-loop execution, dispatcher/safety, MCP support, pools, and advanced agent primitives.

`roko-core` owns shared config and model-selection surfaces that other crates can depend on without the full runtime.

`roko-cli` owns the user-facing entrypoints:

- `run.rs` is the clearest single-run reference path.
- `orchestrate.rs` is the larger plan-execution path and needs path-by-path verification.
- `main.rs` still contains specialty and research entrypoints.

## Quick Facts

- 19 checked-in `impl Agent for ...` blocks under `crates/roko-agent/src/`
- 28 `AgentRole` variants
- 6 `ProviderKind` variants and 6 registered adapters
- shared built-in tool registry in `roko-std`, currently `TOOL_COUNT = 16`
- per-agent HTTP sidecar in `roko-agent-server`
- `PlanRunner` owns `ProcessSupervisor`
- `CascadeRouter` is live
- duplicate `AgentEvent` enums remain in `roko-agent` and `roko-learn`

## Main Batch-02 Question

What is demonstrably wired today, what is only partial, and what should be handed off instead of being half-implemented here?

## Clearly Wired Today

- `Agent` trait plus many concrete agent implementations across Claude CLI/API, Codex, Cursor, OpenAI, Ollama, Gemini, and Perplexity families
- provider construction through `create_agent_for_model()` and CLI-scoped spawning through `spawn_agent_scoped()` / `spawn_agent_with_layer()`
- `ToolLoop`, `ToolLoopAgent`, `ToolDispatcher`, and `SafetyLayer`
- MCP config discovery, registry, handler resolution, and tool bridge plumbing
- `AgentPool`, `MultiAgentPool`, `CompositeAgent`, `MorphableAgent`, and `MetacognitiveMonitor`
- `roko-agent-server` messaging and relay registration surfaces
- `CascadeRouter`, `LinUCBRouter`, Pareto filtering, and anomaly detection

## Partial Or Mixed Wiring

- `run.rs` is the cleanest dispatcher/tool-loop reference path; `orchestrate.rs` needs path-specific wording rather than blanket claims
- the shared `create_tool_loop_backend()` helper is narrower than the total tool-capable provider coverage
- `ChatResponse` and `ResponseMetadata` still have duplicate definitions
- `temperament` is live as a string on `AgentIdentity`, not as a shared typed runtime contract
- research or specialty entrypoints still bypass the main scoped spawn path in places

## Defer By Default

- learning-policy redesign
- verification-policy semantics
- executor/restart redesign
- domain/plugin scaffolding
- plugin SPI tiers 4-5
- research reasoning systems
- shared memory / archive systems

## Working Stance

- Prefer file-backed evidence over architectural guesses.
- Describe a surface as `wired` only if a runtime call path is visible.
- If the next step is bigger than a single-agent 90-minute pass, record the seam and defer it.
