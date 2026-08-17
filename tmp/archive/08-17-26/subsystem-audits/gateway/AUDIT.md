# Gateway Subsystem Audit

## What Is the Bardo Gateway?

`bardo-gateway` (`/Users/will/dev/uniswap/bardo/apps/bardo-gateway/`) is an **LLM inference proxy** — an Axum HTTP server that sits between application code (agents, Claude CLI, any OpenAI-SDK client) and upstream provider APIs. Agents never hold API keys; they POST to the gateway, which handles routing, caching, cost tracking, safety, and billing.

**Tagline from Cargo.toml**: "LLM inference proxy with three-layer caching, multi-provider routing, cost tracking, and USDC micropayments."

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        bardo-gateway                            │
│                    Axum on 0.0.0.0:4000                         │
│                                                                 │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ Auth     │→ │ Safety    │→ │ Cache    │→ │ Provider      │  │
│  │ (API key)│  │ (PII/Inj) │  │ (L1→L2) │  │ Router        │  │
│  └──────────┘  └───────────┘  └──────────┘  │               │  │
│                                              │ Venice (TEE)  │  │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  │ Anthropic     │  │
│  │ Budget   │  │ Cost DB   │  │ Batch    │  │ OpenAI        │  │
│  │ Enforce  │  │ (SQLite)  │  │ Queue    │  │ Bankr         │  │
│  └──────────┘  └───────────┘  └──────────┘  │ OpenRouter    │  │
│                                              └───────────────┘  │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐                     │
│  │ Stats/   │  │ Tool      │  │ Loop     │                     │
│  │ Metrics  │  │ Pruning   │  │ Detection│                     │
│  └──────────┘  └───────────┘  └──────────┘                     │
└─────────────────────────────────────────────────────────────────┘
```

**Tech stack**: Axum + Tower middleware, Reqwest (outbound HTTP), SQLite (rusqlite) for persistence, Moka for in-memory LRU, BLAKE3 for request hashing, DashMap for concurrent state, Alloy for Ethereum crypto (MPP payments).

## Routes (22 endpoints)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `POST` | `/v1/messages` | API key | Anthropic Messages API |
| `POST` | `/v1/chat/completions` | API key | OpenAI Chat Completions API |
| `POST` | `/v1/stream` | API key | Streaming endpoint |
| `POST` | `/v1/embeddings` | API key | Embeddings |
| `GET` | `/v1/costs` | API key | Per-key cost breakdown |
| `GET` | `/v1/stats` | none | Live gateway stats |
| `GET` | `/v1/health` | none | Health check |
| `GET` | `/v1/models` | none | Available models |
| `GET` | `/v1/ws/stats` | none | WebSocket stats stream |
| `GET` | `/v1/cache-economics` | none | Cache analytics |
| `GET` | `/v1/analytics/spend` | none | Spend analytics |
| `GET` | `/v1/analytics/cost-history` | none | Historical cost data |
| `GET` | `/v1/analytics/subsystems` | none | Subsystem breakdown |
| `GET` | `/metrics` | none | Prometheus metrics |
| `GET` | `/v1/optimizations` | none | Optimization stats |
| `POST` | `/v1/batch/submit` | API key | Anthropic Batch API submission |
| `POST` | `/v1/batch/flush` | API key | Force flush batch queue |
| `GET` | `/v1/batch/status` | API key | Batch queue status |
| `GET` | `/v1/batch/result/{id}` | API key | Fetch batch result |
| `POST` | `/v1/mpp/sessions` | MPP | Open micropayment session |
| `GET/DELETE` | `/v1/mpp/sessions/{id}` | MPP | Session status / close |
| `GET` | `/dashboard` | none | Static dashboard UI |

**Auto-detection**: The gateway detects whether an incoming request uses Anthropic or OpenAI wire format and responds in kind. Internally, all responses are normalized to Anthropic format.

## Providers (5)

| Provider | File | Model Prefix | Env Var | Priority |
|---|---|---|---|---|
| **Venice** | `providers/venice.rs` | `venice/*` + intercepted `claude-*` | `VENICE_API_KEY` | 1st (TEE privacy) |
| **Anthropic** | `providers/anthropic.rs` | `claude-*` | `ANTHROPIC_API_KEY` (+ `_2` thru `_10` for rotation) | 2nd |
| **OpenAI** | `providers/openai.rs` | `gpt-*`, `o1`, `o3`, `o4-*` | `OPENAI_API_KEY` | 3rd |
| **Bankr** | `providers/bankr.rs` | `bankr/*` | `BANKR_API_KEY` | 4th |
| **OpenRouter** | `providers/openrouter.rs` | catch-all | `OPENROUTER_API_KEY` | Last (fallback) |

**Key rotation**: Anthropic supports up to 10 keys. On 429, rotates immediately (no delay). On 5xx, exponential backoff with jitter (base 1s, max 5 retries).

## Three-Layer Cache

### L1: Hash Cache (`cache.rs`)
- BLAKE3 hash of normalized request body
- In-memory LRU via Moka (default 10K entries, 1h TTL)
- Regime-aware soft TTLs: Calm=2h, Normal=1h, Volatile=15min, Crisis=5min
- Cache isolation: Shared (default), PerAgent, Tiered (T2 shared, T0/T1 isolated)
- Normalization: UUID/timestamp stripping, tool sorting, JSON key ordering
- Persisted to SQLite every 30s, restored on startup

### L2: Semantic Cache (`semantic_cache.rs`)
- **SimHash** (default): 64-bit fingerprint, Hamming distance ≤3, ~50µs latency
- **Embedding** (opt-in): fastembed ONNX, cosine similarity ≥0.92, ~3-5ms latency
- Excludes tool_use responses (replaying produces invalid tool IDs)
- Persisted to SQLite every 60s

### L3: Prefix Cache (`prefix.rs`)
- Injects `cache_control: {"type": "ephemeral"}` into Anthropic system prompts
- Restructures prompts for maximum cross-request KV cache hits
- Claims 40-85% cost reduction on agent swarm workloads

### In-flight Coalescing
Duplicate requests while one is in-flight subscribe to a broadcast channel rather than spawning a second provider call (`DashMap<[u8; 32], broadcast::Sender>`).

## Cost Tracking (`cost_db.rs`, `pricing.rs`)

Every request is priced using the `PricingTable` (exact match + substring fallback). Computes:
- `actual_cost`: with cache discounts, batch discount
- `naive_cost`: what it would cost without the gateway

Response headers on every request:
```
X-Mori-Cost-Usd / X-Mori-Naive-Cost-Usd / X-Mori-Savings-Usd
X-Mori-Cache-Status / X-Mori-Provider
X-Mori-Tokens-In / X-Mori-Tokens-Out
X-Mori-Session-Cost / X-Mori-Session-Savings
```

SQLite persistence at `.mori/costs.db` — per-request events written async, per-model/session/key breakdowns. Stats survive restarts.

### Pricing Table (March 2026 rates)

| Model | Input $/M | Output $/M | Cached $/M |
|---|---|---|---|
| claude-opus-4-6 | $5.00 | $25.00 | $0.50 |
| claude-sonnet-4 | $3.00 | $15.00 | $0.30 |
| claude-haiku-4-5 | $1.00 | $5.00 | $0.10 |
| gpt-4o | $2.50 | $10.00 | $1.25 |
| gpt-4o-mini | $0.15 | $0.60 | $0.075 |
| o3 | $10.00 | $40.00 | $5.00 |
| o4-mini | $1.10 | $4.40 | $0.55 |
| venice/llama-3.3-70b | $0.59 | $0.79 | — |
| venice/deepseek-r1 | $0.55 | $2.19 | — |
| bankr/gemini-2.5-flash | $0.15 | $0.60 | $0.0375 |

## Budget Enforcement (`budget.rs`)

Four degradation tiers based on budget utilization:
- `< 70%` → Full (requested model)
- `≥ 70%` → T1Only (haiku only)
- `≥ 85%` → Economy (cheapest available)
- `≥ 95%` → T0Only (suppress inference)

Default: $100/agent. Per-session rate limit: 120 req/min sliding window.

## Safety Layer (`safety/`)

Three phases on every request before cache lookup:
1. **PII scanning** (`pii.rs`): Hard-blocks private keys, seed phrases (bip39). Soft-masks other PII with reversible placeholders.
2. **Injection detection** (`injection.rs`): Regex-based + optional ONNX classifier (behind `safety-onnx` feature).
3. **Privacy filtering** (`privacy.rs`): Config-driven privacy rules.

Results: `Clean`, `PiiBlocked`, `InjectionBlocked`, `PassWithWarning`.

## Venice Privacy Classification (`venice/security_class.rs`)

Deterministic keyword classifier (no LLM calls) assigns tiers:
- **Standard** → any provider
- **Confidential** → Venice preferred (portfolio >$1K, PII, rebalance timing)
- **Private** → Venice mandatory, 503 if unavailable (deal negotiation, MEV-sensitive, governance)

## Bankr: Self-Funding Agent Inference (`bankr/`)

- `metabolic.rs`: MetabolicLoopMonitor tracks `ratio = daily_revenue / daily_inference_cost`
- `credits.rs`: Credit balance with vault fee conversion
- `routing.rs`: Model tier selection based on credit balance + task complexity
- `verification.rs`: Cross-model verification for high-stakes actions
- `config.rs`: Three throttle bands — FullAccess (ratio ≥ 1.0), ReducedThroughput, EmergencyOnly

## MPP: Machine Payment Protocol (`crates/mpp/`, `src/mpp/`)

HTTP 402 USDC micropayment flow on Base (chain ID 8453), using ERC-3009 `transferWithAuthorization`.

**Two modes:**
1. **Charge** (per-request): 402 challenge → client signs ERC-3009 → retry with `X-Payment` header
2. **Session**: Pre-fund via one signed deposit, subsequent requests draw from balance

**Spread**: Default 20% over raw provider cost. Tiered discounts:
- None: 20% | Basic (5+ builds): 18% | Verified (25+): 15% | Trusted (100+): 12% | Sovereign (500+): 8%

## Optimization Subsystems (12)

| Module | File | What |
|---|---|---|
| Tool pruning | `tools.rs` | After 50 req, strips unused tool definitions (saves 2-5K tokens/req) |
| Tool schema compress | `tool_compress.rs` | Strips verbose descriptions from tool schemas |
| Tool result compress | `tool_result_compress.rs` | Truncates old tool_result content blocks |
| Context compression | `compress.rs` | Compresses conversation history |
| Loop detection | `loop_guard.rs` | Detects retry loops/oscillation/drift, injects guidance |
| Output budgeting | `output_budget.rs` | Auto-caps max_tokens per model |
| Convergence detection | `convergence.rs` | 3+ similar responses → injects guidance |
| Thinking cap | `thinking_cap.rs` | Caps extended thinking token budgets |
| KV affinity | `kv_affinity.rs` | Tracks warm KV cache sessions at Anthropic |
| Batch API | `batch.rs` | 50% cost reduction for non-urgent requests |
| Context profiles | `context_profile.rs` | Pipeline profiles for context optimization |
| Capabilities | `capabilities.rs` | Per-provider health + capability declarations |

## Configuration

| Setting | Default | Source |
|---|---|---|
| Port | 4000 | `--port` |
| Bind | `0.0.0.0` | `--bind` |
| API key | random UUID | `BARDO_GATEWAY_API_KEY` / `--api-key` |
| L1 cache capacity | 10,000 | `--max-cache` |
| Cache TTL | 3600s | `--ttl` |
| Max body size | 10 MiB | `--max-body-size` |
| Max concurrent | 256 | `--max-concurrent` |
| Pool idle conns/host | 64 | `--pool-max-idle` |
| Pool idle timeout | 90s | `--pool-idle-timeout` |
| MPP enabled | false | `BARDO_MPP_ENABLED` |
| MPP spread | 20% | `BARDO_MPP_SPREAD` |

## What Roko Already Has

Roko has a **dispatch layer** but NOT a **proxy**:

| Component | Bardo Gateway | Roko Equivalent |
|---|---|---|
| HTTP proxy (Anthropic/OpenAI wire) | Full transparent proxy | None — agents go through Rust runtime |
| Provider backends | 5 (Venice, Anthropic, OpenAI, Bankr, OpenRouter) | 6 protocol kinds (AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp, PerplexityApi, GeminiApi). Production roko.toml uses ClaudeCli for Anthropic and OpenAiCompat for OpenAI/Gemini/Kimi/GLM/Ollama. |
| Model routing | TierRouter (role → model) | CascadeRouter (3-stage: static → Wald CI confidence → LinUCB bandit). Transitions at 50 and 200 observations. |
| Caching | 3-layer (hash + semantic + prefix) | None at gateway level |
| Cost tracking | SQLite per-request, response headers | BudgetGuardrail + CostTable (in-memory) |
| Safety | PII scan, injection detection, privacy filter | AgentContract + ToolDispatcher safety |
| Budget enforcement | 4-tier degradation | 4-tier (Warn at configurable %, default 75% / RouteToCheaper >80% / BlockNewSessions >95% / Block >=100%) |
| Batch API | Anthropic batch queue | Route stubs in roko-serve (`/inference/batch/submit`, `/inference/batch/{id}`) but no actual batch provider integration |
| Micropayments | ERC-3009 USDC (MPP) | None |
| Analytics/metrics | Prometheus + WebSocket + SQLite | StateHub events (limited) |
| Provider health | Capabilities tracking | Two separate systems: `ProviderHealth`/`CircuitState` (disk-persisted, error-class-specific cooldowns) and `ProviderHealthTracker`/`HealthState` (in-memory, simpler) |
| Inference gateway routes | — | 5 routes in roko-serve: `POST /inference/complete`, `GET /gateway/stats`, `GET /gateway/models`, `POST /inference/batch/submit`, `GET /inference/batch/{id}` |
| Route count | 22 endpoints | ~130+ route registrations across all modules |

**Key gap**: Roko's inference gateway in roko-serve (`routes/gateway.rs`) creates `Box<dyn Agent>` instances and runs them through the full runtime. There is no lightweight HTTP proxy path. The bardo gateway is a **transparent proxy** — it forwards HTTP requests to upstream providers and returns HTTP responses, with middleware layered in between.

## Anti-Patterns in Bardo Gateway

1. **Monolithic AppState**: Single struct holds 30+ fields including all caches, providers, stats, MPP state. No trait boundaries.
2. **Provider trait too thin**: Just `fn can_handle(&self, model: &str) -> bool` + `fn forward(&self, req) -> Response`. No structured capability negotiation.
3. **Bardo/Golem naming entangled**: `X-Golem-Id`, `X-Mori-Cost-Usd` headers, `golem.toml` config. Needs clean namespace.
4. **Venice-specific classification hardcoded**: `security_class.rs` has DeFi-specific keyword rules baked in. Not generalizable.
5. **Pricing table stale**: Hardcoded rates in Rust source. Should be config-driven or auto-updated from provider APIs.
6. **No multi-tenant isolation**: Single API key namespace. No org/team/project boundaries.
7. **No provider auto-discovery**: Adding a provider requires code changes.
8. **SQLite single-writer bottleneck**: All cost writes funnel through one SQLite connection.

## Key File Locations (Bardo)

| File | Path |
|---|---|
| Main entry | `apps/bardo-gateway/src/main.rs` |
| Lib + start_server | `apps/bardo-gateway/src/lib.rs` |
| AppState | `apps/bardo-gateway/src/state.rs` |
| Provider trait + impls | `apps/bardo-gateway/src/providers/` |
| Route handlers | `apps/bardo-gateway/src/handler.rs` |
| Auth middleware | `apps/bardo-gateway/src/auth.rs` |
| L1 hash cache | `apps/bardo-gateway/src/cache.rs` |
| L2 semantic cache | `apps/bardo-gateway/src/semantic_cache.rs` |
| L3 prefix cache | `apps/bardo-gateway/src/prefix.rs` |
| Pricing | `apps/bardo-gateway/src/pricing.rs` |
| Cost DB | `apps/bardo-gateway/src/cost_db.rs` |
| Budget | `apps/bardo-gateway/src/budget.rs` |
| Safety | `apps/bardo-gateway/src/safety/` |
| Venice classifier | `apps/bardo-gateway/src/venice/security_class.rs` |
| Bankr | `apps/bardo-gateway/src/bankr/` |
| MPP crate | `crates/mpp/` |
| TierRouter | `crates/golem-inference/src/routing.rs` |
| GatewayClient | `crates/golem-inference/src/client.rs` |
| Startup script | `mori-gateway.sh` |
| Cargo.toml | `apps/bardo-gateway/Cargo.toml` |
