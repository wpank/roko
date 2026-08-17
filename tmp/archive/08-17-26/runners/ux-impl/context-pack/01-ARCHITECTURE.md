# Architecture (current state, 2026-05-01)

## 5-Layer model

```
Layer 4 (UI):           kauri/nunchi-dashboard (sibling repo, TS)
                        demo/demo-app          (in-repo, Vite + TS)
                        crates/roko-cli/src/tui (ratatui)

Layer 3 (orchestration): crates/roko-serve     (~85 routes, axum)
                        crates/roko-cli         (orchestrator, agent_serve, ...)

Layer 2 (per-agent):     crates/roko-agent-server (HTTP per agent)

Layer 2.5 (aggregator):  crates/roko-serve/src/routes/aggregator.rs
                          (mirage-compatible /api/* surface)

Layer 1 (sim/substrate): apps/mirage-rs        (EVM fork + JSON-RPC)
                                               (still has bolted-on
                                                http_api/ + chain/ —
                                                being deleted in Wave M)

Layer 0 (chain):         contracts/src/*.sol
                        crates/roko-chain
```

Dependencies flow downward only.

## Crate layer reference

| Layer | Crate | Role |
|-------|-------|------|
| 0 | `roko-core` | Engram, foundation types, traits, RuntimeEvent |
| 1 | `roko-fs`, `roko-primitives`, `roko-neuro`, `roko-dreams`, `roko-daimon`, `roko-index` | Support modules |
| 2 | `roko-agent`, `roko-gate`, `roko-compose`, `roko-learn`, `roko-orchestrator`, `roko-conductor`, `roko-chain` | Domain |
| 3 | `roko-runtime`, `roko-cli`, `roko-serve`, `roko-agent-server` | Runtime + surface |
| 3 | `roko-mcp-{code,github,scripts,slack,stdio}` | MCP servers |
| - | `apps/mirage-rs` | EVM substrate (deletes its bolted-on state in Wave M) |
| - | `apps/agent-relay`, `apps/roko-chain-watcher` | Side daemons |

## What's already shipped

- `roko-agent-server` is a real crate with builder API, all five
  feature modules (messaging, predictions, research, tasks, logs),
  bearer auth (`crates/roko-agent-server/src/auth/bearer.rs`), and
  Agent Card registration including (opt-in) on-chain
  `updateAgentCardUri` (`crates/roko-agent-server/src/registration.rs:177-191`).
- The aggregator at `crates/roko-serve/src/routes/aggregator.rs`
  serves `/api/agents`, `/api/agents/topology`, `/api/agents/{id}/{stats,skills,heartbeat,trace}`,
  `/api/predictions/sessions`, `/api/predictions/sessions/{id}`,
  `/api/predictions/claims`, `/api/predictions/calibration/{agent_id}`,
  `/api/knowledge/{entries,edges,search,kinds}`,
  `/api/tasks`, `/api/tasks/stats`, `/api/tasks/{id}`, `/api/ws`.
- `mirage-rs` Cargo features: `default = ["binary", "chain"]`. `chain`
  implies `dashboard-api`. Pure-EVM mirage already builds with
  `--no-default-features --features binary`.
- The local demo at `demo/demo-app/` already talks to roko-serve at
  `:6677`.
- `crates/roko-cli/src/snapshot_migrate.rs` exists with v0→v1→v2.
- `crates/roko-cli/src/tui/{fs_watch,git_watch,jsonl_tailer,jsonl_cursor}.rs`
  — incremental tailer infrastructure exists but is only used by
  efficiency + c-factor tailers as of 2026-05-01.
- Gate pipeline: `run_gate_rung` (in `crates/roko-cli/src/orchestrate.rs:17656`)
  dispatches all 7 canonical rungs through `roko_gate::rung_dispatch::run_rung`.

## What's NOT yet shipped (this runner's scope)

| Concern | Plan | Wave |
|---------|------|------|
| `apps/mirage-rs/{http_api,chain,roko_bridge}/` deletion | 01 | M |
| `/api/pheromones/*` routes on aggregator | 02 | AG |
| Knowledge endpoints reading from `InsightBoard` chain | 02 | AG |
| `nunchi-dashboard` URL split (REST vs JSON-RPC) | 03 | DB (manual) |
| Chain enumeration of agents via 8004 | 04 | CH |
| Capability bitmask bit 15 + `"roko"` domain tag filtering | 04 | CH |
| Demo bootstrap of 5 chain-registered passports | 04 | CH |
| Per-agent WS subscribe in TUI Agents tab | 05 | TU |
| Episode/signals/event-log/task-output incremental tail | 05 | TU |
| Per-MCP-crate audit + bucket | 06 | MC |
| `default-members` for shipped slice | 07 | FG |
| Codex/Cursor/Gemini/Perplexity/Ollama parity tests | 08 | BP |
| Stale doc + terminology sweep | 09 | DC (manual) |
| SystemPromptBuilder snapshot tests | 10 | HY |
| Per-gate timeline render | 10 | HY |
| TUI-parity runner hardening | 11 | RH (manual) |

## Current file layout (key paths)

```
apps/mirage-rs/src/
  Cargo.toml                            # default = ["binary", "chain"]
  lib.rs                                # pub mod chain (gated); pub mod http_api (gated)
  main.rs                               # CLI flags --enable-hdc, --enable-knowledge, --enable-stigmergy
  rpc.rs                                # JSON-RPC + chain_* methods (the latter to be removed)
  fork.rs replay.rs scenario.rs persist.rs provider.rs precompiles/
  http_api/                             # to be DELETED (Wave M)
    mod.rs agent.rs knowledge.rs pheromone.rs prediction.rs skills.rs task.rs topology.rs ws.rs isfr.rs
  chain/                                # to be DELETED (Wave M)
  roko_bridge/                          # to be DELETED (Wave M)

crates/roko-serve/src/
  state.rs                              # AppState, DiscoveredAgent
  lib.rs                                # serve(...) startup
  openapi.rs                            # utoipa OpenAPI doc
  error.rs                              # ApiError
  routes/
    aggregator.rs                       # ~1500 LOC, the mirage-compat surface
    agents.rs config.rs deployments.rs heartbeats.rs
    learning/ status/ ...

crates/roko-agent-server/src/
  lib.rs                                # AgentServer + AgentServerBuilder
  state.rs                              # AgentState
  registration.rs                       # AgentCard, updateAgentCardUri (alloy)
  features/
    health.rs messaging.rs predictions.rs research.rs tasks.rs logs.rs relay_client.rs
  auth/bearer.rs

crates/roko-chain/src/
  lib.rs                                # exports
  identity_economy_identity.rs          # write-side identity ops
  agent_registry.rs                     # legacy roko AgentRegistry
  marketplace.rs                        # template for new readers (e.g. InsightBoardReader)
  validation_registry.rs reputation_registry.rs
  client.rs alloy_impl.rs mock.rs wallet.rs types.rs

crates/roko-cli/src/tui/
  app.rs dashboard.rs state.rs input.rs
  fs_watch.rs git_watch.rs jsonl_tailer.rs jsonl_cursor.rs verdicts.rs
  views/agents_view.rs views/dashboard_view.rs ...

crates/roko-agent/src/
  tool_loop/mod.rs
  dispatcher/mod.rs                     # multi-backend dispatch
  multi_pool.rs
  claude_agent.rs claude_cli_agent.rs codex_agent.rs cursor_agent.rs
  openai_agent.rs openai_compat_backend.rs ollama_agent.rs ollama_backend.rs
  gemini/native.rs perplexity/{chat,deep_research,tool_loop,search}.rs
  exec.rs
  safety/mod.rs

contracts/src/
  IdentityRegistry.sol AgentRegistry.sol BountyMarket.sol
  InsightBoard.sol ReputationRegistry.sol ValidationRegistry.sol
  WorkerRegistry.sol ConsortiumValidator.sol FeeDistributor.sol
```

## Cross-crate flows you'll touch

### Aggregator request lifecycle

```
HTTP request → roko-serve middleware (auth, tracing)
  → routes/aggregator.rs handler
  → state.cached_value/json (LRU TTL cache)
  → KnowledgeSource::Chain { reader }    (after Wave AG)
  →   reader.list_insights() via roko-chain
  →   AgentCardFetcher.fetch(uri)
  → return JSON (shape preserved from mirage-rs golden fixture)
```

### Agent registration lifecycle

```
roko-cli `agent serve` → AgentServer::builder().build().serve()
  → registration.publish_card(state, addr)
  →   publisher.publish(card)            # IPFS / data-URI / relay
  → registration.update_identity_registry(card_uri)
  →   wallet.sign_and_submit(updateAgentCard tx)   # opt-in, on-chain
  → on_start hook                                  # operator callback
  → heartbeat_loop (background, POST /api/heartbeats to roko-serve)
```

### Aggregator agent discovery (after Wave CH)

```
known_agents(state)
  → state.identity_reader.enumerate_passports(filter)
  →   PassportFilter { require_capability_bits: BIT_ROKO_COMPATIBLE }
  → fetcher.fetch(card.agent_card_uri) for each
  → filter where domain_tags contains "roko"
  → merge with state.process_supervisor.discovered_agents()
  → cache (TTL 30s, invalidated on AgentCardUpdated event)
```
