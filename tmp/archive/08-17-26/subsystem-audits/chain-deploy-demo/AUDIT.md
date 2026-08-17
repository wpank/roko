# Chain, Deployment & Demo Subsystem Audit

On-chain agent registry, collusion detection, Railway/Docker deployment, tournament benchmarks — Phase 2+ infrastructure that's mostly built but dormant, plus active deployment tooling.

## The Problem

roko-chain (20K LOC) has production-ready blockchain abstractions (ChainClient/ChainWallet traits, Alloy backend, soulbound passports, collusion detection) but only a handful of things are used at runtime: basic chain reads via serve routes, alloy client initialization in orchestrate.rs, non-blocking agent registration in agents.rs, and demo scenarios. The rest (reputation registry, marketplace, KORAI token, x402 state channels, identity economy, chain gates) is built and dormant. Deployment tooling (Railway, Fly.io, Docker, daemon) is fully wired and active.

---

## 1. roko-chain (20,095 LOC, 30 files)

### Core Abstractions

**ChainClient trait** (read-only):
```rust
#[async_trait]
trait ChainClient: Send + Sync {
    async fn block_number(&self) -> ChainResult<BlockNumber>;
    async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader>;
    async fn get_receipt(&self, tx: &TxHash) -> ChainResult<Option<Receipt>>;
    async fn get_logs(&self, from, to, addresses: &[String], topics: &[String]) -> ChainResult<Vec<LogEntry>>;
    async fn get_storage_at(&self, address: &str, slot: &str, block: Option<BlockNumber>) -> ChainResult<Vec<u8>>;
    async fn eth_call(&self, request: &TxRequest, block: Option<BlockNumber>) -> ChainResult<CallResult>;
    async fn get_balance(&self, address: &str, block: Option<BlockNumber>) -> ChainResult<u128>;
    async fn chain_id(&self) -> ChainResult<u64>;
    fn name(&self) -> &str;
}
```

**ChainWallet trait** (sign + submit):
```rust
#[async_trait]
trait ChainWallet: Send + Sync {
    async fn address(&self) -> ChainResult<String>;
    async fn balance(&self, block: Option<BlockNumber>) -> ChainResult<u128>;
    async fn nonce(&self) -> ChainResult<u64>;
    async fn sign_and_submit(&self, tx: TxRequest) -> ChainResult<TxHash>;
    async fn wait_for_receipt(&self, tx: &TxHash, timeout_ms: u64) -> ChainResult<Receipt>;
    fn name(&self) -> &str;
}
```

**Alloy backend:** `AlloyChainClient::http(rpc_url)` + `AlloyChainWallet::from_hex_key()`. Feature-gated `[alloy-backend]`.

**Mock doubles:** `MockChainClient` + `MockChainWallet` with `paired_mocks()` for integration tests.

### Module Inventory

| Module | LOC | Purpose | Status |
|---|---|---|---|
| phase2.rs | 2,312 | Phase 2 stubs (80+ placeholder types) | Orphaned |
| identity_economy_identity.rs | 2,154 | On-chain identity proofs | Dormant |
| identity_economy_markets.rs | 1,428 | Identity-gated markets | Dormant |
| isfr.rs | 1,277 | Incentive-Stable Fee Routing | Dormant |
| reputation_registry.rs | 1,179 | 7-domain EMA scoring (CHAIN-03) | Dormant |
| marketplace.rs | 1,090 | Spore job marketplace + escrow (CHAIN-04) | Dormant |
| gate/mev_gate.rs | 1,005 | MEV detection (sandwich, frontrun) | Dormant |
| x402.rs | 958 | HTTP 402 micropayments + state channels (CHAIN-08) | Dormant |
| agent_registry.rs | 785 | Soulbound ERC-721 passports (CHAIN-02) | Partially wired |
| tools.rs | 764 | 10 DeFi chain tools for agents | Dormant |
| mock.rs | 738 | MockChainClient/Wallet + paired_mocks | Active (tests) |
| gate/wallet_gate.rs | 579 | Wallet balance/nonce checks | Dormant |
| korai_token.rs | 657 | KORAI token with lazy demurrage (CHAIN-01) | Dormant |
| futures_market.rs | 590 | Futures market for DeFi routing | Dormant |
| triage.rs | 510 | EventEnrichment + MidasRScorer | Dormant |
| trace_rank.rs | 508 | PageRank reputation over payment edges | Dormant |
| heartbeat_ext.rs | 475 | ChainHeartbeatExtension, PolicyCage | Dormant |
| validation_registry.rs | 456 | GateScore, ValidationRecord (CHAIN-05) | Dormant |
| gate/tx_sim_gate.rs | 445 | Transaction simulation gate | Dormant |
| collusion.rs | 379 | Collusion ring detection (clique analysis) | Dormant |
| nelson_siegel.rs | 307 | Nelson-Siegel yield curve model (P2-09) | Dormant |
| observer.rs | 345 | BlockObserver, log event enrichment | Dormant |
| alloy_impl.rs | 344 | Alloy JSON-RPC backend | Active |
| witness.rs | 305 | Chain witness engine | Dormant |
| types.rs | 232 | Core types | Active |
| lib.rs | 121 | Crate root + pub re-exports | Active |
| client.rs | 71 | ChainClient trait | Active |
| wallet.rs | 36 | ChainWallet trait | Active |
| gate/mod.rs | 45 | Gate module re-exports | Active |

### Agent Registry (Soulbound ERC-721)

- Non-transferable NFT per agent with capabilities + tier
- **10 capability bits:** inference, data-transform, fine-tune, RAG, multi-agent, trading, security, analytics, knowledge, strategy
- **4 tiers:** EDGE (no stake), WORKER (5K KORAI), SOVEREIGN (25K KORAI), PROTOCOL (100K KORAI, governance-approved)
- **Ventriloquist defense:** system prompt hash committed; 24h timelock on updates; >3 changes in 30 days → reputation penalty

### Collusion Ring Detection

- Build assignment graph (A→B = A assigned job to B)
- Mutual ratio: `min(A→B, B→A) / max(A→B, B→A)`; threshold 0.5
- DFS clique detection for fully connected subgraphs ≥3
- Penalty: feedback weight dilution (-50% for 30 days)

### What's Actually Used at Runtime

| Component | Where | Status |
|---|---|---|
| AlloyChainClient init | orchestrate.rs:3985 | Wired (from `chain.rpc_url` config) |
| AlloyChainWallet init | orchestrate.rs:4003 | Wired (from `chain.wallet_key` config) |
| serve chain routes (3) | `GET /api/chain/{agents,bounties,status}` | Wired |
| on-chain agent registration | agents.rs (non-blocking dual-write on create/update) | Wired |
| Demo scenarios | roko-demo | Active |
| Mock doubles | Tests | Active |

**Everything else (16,000+ LOC):** Built, exported, never called at runtime.

---

## 2. Deployment System

### Deploy Backends

| Backend | File | LOC | Status |
|---|---|---|---|
| Railway API | `roko-serve/src/deploy/railway_api.rs` | 923 | Wired |
| Railway CLI | `roko-serve/src/deploy/railway_cli.rs` | 160 | Wired (fallback) |
| Manual Docker | `roko-serve/src/deploy/manual.rs` | 119 | Wired |
| Fly.io | `roko-cli/src/commands/server.rs` (`cmd_deploy_fly`) | ~60 | Wired |
| Sigstore Verifier | `roko-cli/src/deployment.rs` | 114 | Active (release verification) |

**Railway API** — Full integration: create/deploy/update/delete services, env vars, region selection, status polling, logs streaming.

**Fly.io** — Generates `fly.toml` and calls `flyctl deploy --remote-only`.

**Manual Docker** — Generates Dockerfile + docker-compose + instructions for self-hosted.

### Daemon System (daemon.rs, 2,015 LOC)

**IPC commands** (Unix domain socket):

| Command | Purpose |
|---|---|
| Status | Health + uptime + agent count + memory |
| Stop | Graceful shutdown + state backup |
| Restart | Stop → reload config |
| Reload | Config/templates/subscriptions without restart |
| ListSubscriptions | All monitored repos |
| PauseSubscription | Pause one repo |
| ResumeSubscription | Resume one repo |

**Daemon lifecycle:**
1. HTTP server on configured port
2. Socket listener at `{workdir}/.roko/daemon.sock`
3. Subscription scheduler (polls repos, allocates agents, tracks spend)
4. File watcher for .roko/ config changes
5. Platform-specific install (macOS LaunchAgent, Linux systemd)

### Worker System

**Entry:** `roko worker`

**Flow:**
1. Read `ROKO_TEMPLATE_JSON` env (Base64-encoded AgentTemplate)
2. Load `ROKO_CONTROL_PLANE_URL` + `ROKO_DEPLOYMENT_ID`
3. Bind to `PORT` env (Railway injects)
4. Run thin HTTP server
5. Cloud execution: clone → branch (`impl/{slug}`) → commit → push → open PR

**cloud.rs (558 LOC):** Ephemeral git workflow (clone → branch → commit → push → open PR) with GitHub MCP server for code operations (`roko-cli/src/worker/cloud.rs`).

---

## 3. roko-demo (5,860 LOC, 21 files)

### Scenarios run via separate `roko-demo` binary; `roko demo` and `roko bench demo` subcommands exist but are unrelated prep/benchmarking utilities.

The 5 demo scenarios (up, tournament, benchmark c-factor) are only accessible via:

```bash
cargo run -p roko-demo -- up yield-routing
cargo run -p roko-demo -- tournament --rounds 5
cargo run -p roko-demo -- benchmark c-factor
```

The `roko` CLI has a `roko demo setup|warm` for workspace prep and `roko bench demo` for internal benchmarks, but these do NOT invoke roko-demo scenarios.

### 5 Scenarios

| Scenario | Purpose |
|---|---|
| job_board | Job marketplace with posters + workers |
| consortium | Multi-agent collective decision-making |
| defi_routing | Token swap/liquidity routing |
| flywheel | Reputation-based incentive loop |
| yield_routing | DeFi yield optimization (tournament-enabled) |

### Scenario Trait

```rust
#[async_trait]
trait Scenario: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_fixtures(&self, _registry: &mut FixtureRegistry) {}
    async fn spine(
        &self,
        ctx: Arc<ChainCtx>,
        manifest: &ScenarioManifest,
        runtime: Arc<ScenarioRuntime>,
    ) -> anyhow::Result<()>;
}
```

**Pattern:** Scripted backbone (deterministic flow) + LLM leaves (agent decisions via `LlmProvider`).

### Tournament System

- `prepare_tournament()` → `run_tournament(rounds)` → `TournamentReport`
- Per-round: calls `yield_routing::run_round()`, records `RoundOutcome` (winner, output_eth, confidence)
- Aggregate: wins per agent, avg confidence, total ETH output, learning curve
- Only `yield_routing` scenario is tournament-enabled; other scenarios have no tournament support

### Benchmark

- C-factor measurement: cold run vs warm run improvement
- `improvement_pct = (warm - cold) / cold * 100`

### LLM Backend

- `StubLlm` — deterministic bounded-random for CI
- `ClaudeApiProvider` — Anthropic API via `ANTHROPIC_API_KEY` env (`--llm-backend claude`)
- `OllamaProvider` — local Ollama via `OLLAMA_URL` env (`--llm-backend ollama`)
- `MultiProvider` — round-robin over multiple backends (`--llm-backend multi`)

**Available backends:** `stub` (default), `claude`, `ollama`, `multi`. No `openai` backend.

### Event Streaming

- WebSocket server for live scenario visualization
- TUI for terminal dashboard (`roko-demo tui`)
- `--events [none|ndjson|ws|both]` (no `file` mode; `ndjson` writes newline-delimited JSON to stdout)

---

## 4. PRD Lifecycle (prd.rs, 1,259 LOC at `crates/roko-cli/src/prd.rs`)

```
Idea → Draft (agent research) → Publish → Plan (agent task decomposition) → Execute
```

| Command | Purpose |
|---|---|
| `roko prd idea "<text>"` | Capture work item |
| `roko prd draft new "<slug>"` | Create agent-enhanced draft |
| `roko prd plan <slug>` | Generate tasks.toml from PRD |
| `roko prd publish <slug>` | Trigger auto-plan (if config enabled) |
| `roko prd consolidate` | Scan for gaps + duplicates |

**Wired:** Fully — tied to `roko plan run` execution path.

---

## 5. Research Agent (research.rs, 1,069 LOC at `crates/roko-cli/src/research.rs`)

| Command | Purpose |
|---|---|
| `roko research topic "<topic>"` | Deep research via Perplexity (10-20 citations) |
| `roko research search "<query>"` | Direct web search |
| `roko research enhance-prd/plan/tasks` | Enhance documents with research |
| `roko research analyze` | Self-learning from historical runs |

**APIs:** PerplexityEmbedAgent (web search), optional Gemini grounding.

**Output:** Markdown to `.roko/research/<slug>.md` with citations and search context.

**Wired:** Fully — called via `roko research` subcommands.

---

## 6. Anti-Patterns

| Anti-Pattern | Where | Impact |
|---|---|---|
| **16K+ LOC dormant chain code** | roko-chain | Built but never called; maintenance burden |
| **Phase 2 placeholder types** | phase2.rs (2,312 LOC) | 80+ empty types for deferred features |
| **Demo scenarios not in `roko` CLI** | roko-demo | Scenarios only accessible via `cargo run -p roko-demo`; `roko demo` only does setup/warm |
| **gate/mev_gate.rs LOC concentration** | gate/mev_gate.rs (1,005 LOC) | Largest gate file, entirely dormant |

---

## 7. Summary: What's Active vs Dormant

### Active (Used at Runtime)
- ChainClient/ChainWallet traits + Alloy backend
- Serve chain routes (3 endpoints)
- On-chain agent registration (dual-write via agents.rs)
- Railway API deployment
- Fly.io deployment
- Daemon IPC + subscriptions
- Worker HTTP server + cloud execution
- PRD lifecycle
- Research agent
- Demo scenarios + tournament + benchmark (via roko-demo binary)

### Dormant (Built, Never Called)
- Reputation registry (7-domain EMA)
- Marketplace (job escrow + 3 hiring models)
- KORAI token (lazy demurrage)
- x402 state channels
- Identity economy (2 modules)
- Collusion ring detection
- Trace rank (PageRank reputation)
- ISFR (fee routing)
- Futures market
- Nelson-Siegel yield curve model
- Event triage + enrichment
- Block observer
- Chain witness engine
- Chain gates (MEV detection, tx simulation, wallet gate) — 2,074 LOC total
- Phase 2 stubs

---

## Sources

Key source files verified during this audit:

- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/wallet.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/alloy_impl.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/mock.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/collusion.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/gate/` (mev_gate.rs, tx_sim_gate.rs, wallet_gate.rs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/chain.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/deploy/railway_api.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/deploy/railway_cli.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/deploy/manual.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/daemon.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/deployment.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/server.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/worker/cloud.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/worker/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/research.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/scenarios/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/scenarios/llm.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/tournament.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/benchmark.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-demo/src/events.rs`
