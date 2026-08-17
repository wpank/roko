# File Inventory — Reference for Batches

Most batches name absolute paths. This file is the dictionary so an agent
doesn't have to grep around to find what each path is for. Update only if
files move.

## Crates

| Crate | Path | Role |
|---|---|---|
| roko-core | `crates/roko-core/` | Kernel: `Signal`, 6 traits, config, tools, errors |
| roko-cli | `crates/roko-cli/` | CLI binary: subcommands, TUI, orchestrator |
| roko-agent | `crates/roko-agent/` | LLM backends, dispatch, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | Per-agent HTTP sidecar (~13 routes) |
| roko-serve | `crates/roko-serve/` | HTTP control plane (~85 routes on :6677) |
| roko-orchestrator | `crates/roko-orchestrator/` | Plan DAG, parallel executor |
| roko-gate | `crates/roko-gate/` | 11 gates, 7-rung pipeline |
| roko-compose | `crates/roko-compose/` | Prompt assembly, 9 templates |
| roko-learn | `crates/roko-learn/` | Episodes, cascade router, experiments, efficiency |
| roko-neuro | `crates/roko-neuro/` | Durable knowledge store, distillation |
| roko-fs | `crates/roko-fs/` | File storage (JSONL substrate) |
| roko-std | `crates/roko-std/` | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | `ProcessSupervisor`, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | HDC vectors, tier routing |
| roko-dreams | `crates/roko-dreams/` | Offline consolidation (hypnagogia, imagination) |
| roko-daimon | `crates/roko-daimon/` | Affect engine, somatic markers |
| roko-conductor | `crates/roko-conductor/` | 10 watchers, circuit breaker |
| roko-acp | `crates/roko-acp/` | ACP protocol (editor integration) |
| roko-chain | `crates/roko-chain/` | Chain witness primitives (Phase 2+) |
| roko-mcp-code | `crates/roko-mcp-code/` | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | Parser + graph + HDC indexing |

## Three execution engines (the convergence target)

These all serve overlapping purposes. The convergence direction is
**runner v2 + WorkflowEngine** as the live path; `orchestrate.rs` is
frozen behind `legacy-orchestrate`.

| Engine | Path | LOC | Status |
|---|---|---|---|
| `orchestrate.rs` | `crates/roko-cli/src/orchestrate.rs` | ~21,653 | Frozen; default-off; bug fixes only |
| Runner v2 | `crates/roko-cli/src/runner/` | ~2,181 | Default-on but partially wired |
| `WorkflowEngine` | `crates/roko-runtime/src/workflow_engine.rs` | ~1,500 | Used by ACP and `roko prompt` single-turn |

## Dispatch paths (the unification target)

| Path | Entry point | Use |
|---|---|---|
| `ModelCallService` | `crates/roko-agent/src/model_call_service.rs` | Single canonical dispatcher |
| `ServiceFactory::build` | `crates/roko-orchestrator/src/service_factory.rs` | Wraps ModelCallService for runner v2 |
| `dispatch_direct` | `crates/roko-cli/src/dispatch_direct.rs` | DEPRECATED; chat happy-path; route to ChatAgentSession |
| ACP raw subprocess | `crates/roko-acp/src/session.rs` | Replaced by Dispatcher in `R3_F01` |
| Serve routes | `crates/roko-serve/src/routes/{inference,gateway,...}.rs` | Local route-construct of `reqwest::Client`; migrate to ModelCallService |

## Learning artifacts (durable state)

| Artifact | Path | Producer | Consumer |
|---|---|---|---|
| Episodes JSONL | `.roko/episodes.jsonl` | `EpisodeSink` | `roko learn episodes`, distillation |
| Routing observations | `.roko/learn/cascade-router.json` | `RoutingObservationSink` | `CascadeRouter::load` on startup |
| Gate thresholds | `.roko/learn/gate-thresholds.json` | `AdaptiveThresholds` | `GateService` on startup |
| Experiments | `.roko/learn/experiments.json` | bench runs | `roko learn experiments` |
| Efficiency events | `.roko/learn/efficiency.jsonl` | runner v2 / orchestrate.rs | summarisation |
| Knowledge candidates | `.roko/learn/knowledge-candidates.jsonl` | `KnowledgeIngestionSink` | `roko_neuro::admission` (after T0-6 fix) |
| Signals | `.roko/signals.jsonl` | `SignalLogger` | currently empty / orphan |

## Config files

| Path | Owner | Hot-reload? |
|---|---|---|
| `roko.toml` | repo root | partial (some sections) |
| `~/.config/roko/config.toml` | user | no |
| `roko.shared.toml` | builder workspaces | no |

## Known dead-code zones (DEBT_* targets)

- `crates/roko-learn/src/resonant_patterns.rs` — orphan, not in lib.rs
- `crates/roko-learn/src/signal_metabolism.rs` — orphan
- `crates/roko-learn/src/shapley.rs` — orphan
- `crates/roko-learn/src/kalman.rs` — orphan
- 14 other learn modules in lib.rs with zero external callers
- 7 phantom config sections (`OneirographyConfig`, `DemurrageConfig`, etc.)
- 6 phantom conductor fields (`auto_advance_batch`, `pre_plan`, ...)
- `ConductorObservationSink`, `DreamTriggerSink` — write-only sinks

## Common file paths cited by tasks

These appear in many batches; treat them as well-known:

- `crates/roko-cli/src/orchestrate.rs:14554..16613` — `dispatch_agent_with`
- `crates/roko-cli/src/runner/event_loop.rs` — runner v2 event loop
- `crates/roko-cli/src/main.rs:2225..2260` — accessibility env vars (still UB)
- `crates/roko-cli/src/commands/config_cmd.rs:200..215` — unreachable arms
- `crates/roko-serve/src/routes/mod.rs:100..170` — auth/public router merge
- `crates/roko-serve/src/routes/config.rs:36..58` — TOML/JSON config endpoints
- `crates/roko-gate/src/rung_dispatch.rs:132..239` — stub verdict zone
- `crates/roko-core/src/config/{schema,validation,provenance}.rs` — config split
