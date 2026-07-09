# Workspace Topology — Key Crates + Responsibilities

Reference map of the 28-crate workspace. Use this before touching an unfamiliar
crate so your edit lands in the right tree.

## Core kernel

| Crate | Role | Key types |
|-------|------|-----------|
| `roko-core`        | Signal + 6 trait kernel (Substrate, Scorer, Gate, Router, Composer, Policy). Types, config, tools, errors, obs. | `Signal`, `Engram`, `StateHub`, `DashboardSnapshot`, `SharedStateHub`, `Metric*` |
| `roko-primitives`  | HDC vectors + tier routing. Wired in orchestrate/neuro/learn; HDC fingerprint-per-episode pending (UX25). | `HdcVector`, `Tier` |
| `roko-std`         | Defaults, 19 builtin tools, mock dispatcher. Stable. | `StdSubstrate`, `MockDispatcher` |
| `roko-fs`          | FileSubstrate (JSONL), GC, layout. Stable. | `FileSubstrate` |

## Runtime / orchestration

| Crate | Role | Key files |
|-------|------|-----------|
| `roko-runtime`     | ProcessSupervisor, event bus, cancellation, metrics. | `process.rs`, `event_bus.rs`, `cancel.rs`, `metrics.rs`, `resource.rs` |
| `roko-orchestrator`| Plan DAG, parallel executor, merge queue, safety. | `plan_runner.rs` (may live under `roko-cli/src/orchestrate.rs`) |
| `roko-conductor`   | 10 watchers, circuit breaker, diagnosis. Diagnosis output currently invisible to operator (UX16). | `diagnosis.rs`, `watcher/*.rs` |
| `roko-gate`        | 11 gates, 7-rung pipeline, adaptive thresholds. 4 of 7 rungs currently unwired (UX23). | `lib.rs`, `adaptive_threshold.rs`, `payload.rs`, `artifact_store.rs` |
| `roko-compose`     | Prompt assembly, 9 templates, 6-layer system-prompt builder. Enrichment module exists but has no call-sites (UX28). | `system_prompt_builder.rs`, `templates/`, `enrichment/` |
| `roko-learn`       | Episodes, playbooks, bandits, model routing, experiments, efficiency, cascade router. Fully wired; playbook query missing (UX24). | `episode_logger.rs`, `playbook.rs`, `cascade_router.rs`, `model_router.rs`, `model_experiment.rs` |

## Agent / dispatch

| Crate | Role | Key files |
|-------|------|-----------|
| `roko-agent`       | Dispatcher + 8+ backends (Claude CLI, Claude API, Codex, Cursor, OpenAI, Gemini, Perplexity, Ollama), pools, MCP, tool loop, safety. | `dispatcher/mod.rs`, `tool_loop/mod.rs`, `claude_agent.rs`, `codex_agent.rs`, `cursor_agent.rs`, `safety/mod.rs`, `safety/contract.rs` |
| `roko-agent-server`| Per-agent HTTP sidecar: `/message` (real LLM dispatch T9), `/stream` WS, `/predictions`, `/research`, `/tasks`, `/health`, `/stats`. | `lib.rs`, `state.rs`, `features/messaging.rs`, `features/stream.rs` |

## HTTP control plane

| Crate | Role | Key routes |
|-------|------|-----------|
| `roko-serve`       | ~85 REST routes + SSE + WebSocket on :6677. No request validation / OpenAPI yet (UX39). | `routes/*.rs` (status, learning, deployments, templates, middleware, experiments, agents) |

## User-facing

| Crate | Role | Key files |
|-------|------|-----------|
| `roko-cli`         | CLI binary + ratatui TUI. Main entry-point for everything. TUI polling is the user-flagged bug surface (UX05-UX11). | `main.rs`, `prd.rs`, `orchestrate.rs`, `tui/app.rs`, `tui/dashboard.rs`, `tui/state.rs`, `tui/views/*.rs` |

## MCP integrations

| Crate | Status | LOC |
|-------|--------|-----|
| `roko-mcp-code`    | Wired (PR #13) | ~ |
| `roko-mcp-github`  | Build status unknown (UX29) | 2 643 |
| `roko-mcp-slack`   | Build status unknown (UX29) | 920 |
| `roko-mcp-scripts` | Build status unknown (UX29) | 767 |
| `roko-mcp-stdio`   | Build status unknown (UX29) | 246 |

## Language / index support

| Crate | Role |
|-------|------|
| `roko-index`         | Parser + graph + HDC indexing |
| `roko-lang-rust`     | Rust language support |
| `roko-lang-typescript` | TS language support |
| `roko-lang-go`       | Go language support |

## Phase 2+ (feature-gate target for UX29)

| Crate | Status |
|-------|--------|
| `roko-neuro`    | Wired (durable knowledge store + distillation) |
| `roko-dreams`   | Phase 2+ (compiles + tests but no call-sites) |
| `roko-daimon`   | Phase 2+ |
| `roko-chain`    | Phase 2+ |
| `roko-plugin`   | Fate TBD (not in CLAUDE.md key-crates; UX28/29 may drop or document) |

## Absolute paths

| What | Path |
|------|------|
| Workspace root | `/Users/will/dev/nunchi/roko/roko/` |
| All crates     | `/Users/will/dev/nunchi/roko/roko/crates/` |
| CLI source     | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/` |
| Orchestrator   | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (~15 K LOC, the central nervous system) |
| Agent dispatch | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` |
| Safety layer   | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/` |
| SystemPromptBuilder | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` |
| Role templates | `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/` |
| `.roko/` data  | `/Users/will/dev/nunchi/roko/roko/.roko/` (state/ prd/ learn/ episodes.jsonl signals.jsonl task-outputs/) |
| Implementation plans | `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/` (UX46 refreshes markers) |
| UX follow-up catalog | `/Users/will/dev/nunchi/roko/roko/tmp/ux-followup/` |

## Crate dependency hints

- `roko-cli` imports everything downstream of `roko-core`
- `roko-agent-server` imports `roko-agent` (dispatch) + `roko-core` (obs)
- `roko-serve` imports `roko-core` + `roko-learn` + friends
- Backends in `roko-agent/src/*_agent.rs` implement `LlmBackend` from the
  same crate's `provider/mod.rs`

## What not to touch

- `crates/bardo-backup/` — does not exist in the workspace; only
  `/Users/will/dev/nunchi/roko/bardo-backup/` which is **read-only**
- `.roko/` runtime files — the runner / orchestrator own these
- `target/` + `.cargo-target/` — build outputs; the runner wipes them
- `/Users/will/dev/uniswap/bardo/` — the old Mori monorepo, reference only
