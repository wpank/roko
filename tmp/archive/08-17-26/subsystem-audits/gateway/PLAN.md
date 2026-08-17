# Gateway Implementation Plan

## Phase 1: Core Proxy (MVP)

**Goal**: A working HTTP proxy that accepts Anthropic/OpenAI requests and forwards to upstream providers.

### 1.1 Scaffold crate
- Create `crates/roko-gateway/` with Cargo.toml
- Define `Provider` trait, `ProxyRequest`/`ProxyResponse` types
- Define `GatewayConfig` (TOML + env)
- Wire `GatewayBuilder` → Axum router

### 1.2 Wire format detection
- `format.rs`: detect Anthropic vs OpenAI from request body shape
- `normalize.rs`: internal `ProxyRequest` representation
- Roundtrip: client format → internal → upstream → internal → client format

### 1.3 Provider implementations (Tier 1)
- `anthropic.rs`: Messages API forward (streaming + buffered)
- `openai.rs`: Chat Completions forward
- `openai_compat.rs`: Generic base for DeepSeek, Mistral, HF, OpenRouter, Ollama
- `ProviderRegistry`: resolve model → provider at runtime

### 1.4 Auth + rate limiting
- API key middleware (header extraction, validation)
- Per-key rate limiter (governor token bucket)

### 1.5 Cost tracking
- `PricingTable` (config-driven, not hardcoded)
- Per-request cost computation (input + output + cached tokens)
- Response headers: `X-Roko-Cost-Usd`, `X-Roko-Provider`, `X-Roko-Tokens-In/Out`

### 1.6 Health + stats endpoints
- `GET /health` — provider connectivity
- `GET /models` — available models with capabilities
- `GET /stats` — request counts, costs, cache hits

### 1.7 Integration with roko-serve
- Feature-gated mount: `roko-serve` can embed gateway routes
- Standalone binary entry point: `roko gateway serve`

**Deliverable**: Proxy that can forward Claude/OpenAI requests with cost tracking and provider failover.

---

## Phase 2: Caching + Optimization

**Goal**: Multi-layer caching and request optimization that delivers measurable cost savings.

### 2.1 L1 hash cache
- BLAKE3 hash of normalized request
- Moka LRU (configurable capacity + TTL)
- Request normalization: UUID stripping, tool sorting, key ordering
- Cache isolation modes: shared, per-key, tiered

### 2.2 L3 prefix cache
- Inject `cache_control: {"type": "ephemeral"}` for Anthropic
- Restructure prompts so stable prefix maximizes cross-request hits

### 2.3 In-flight coalescing
- DashMap of in-flight request hashes → broadcast channels
- Duplicate requests subscribe instead of spawning new provider calls

### 2.4 Tool pruning
- Track per-session tool usage
- After N requests, strip unused tool definitions from request
- Save 2-5K tokens/request on tool-heavy agents

### 2.5 Context compression
- Truncate old tool_result content blocks
- Compress conversation history for long sessions

### 2.6 Loop + convergence detection
- Detect 3+ similar tool calls or responses
- Inject guidance into system prompt to break loops
- Track oscillation patterns (A→B→A→B)

### 2.7 L2 semantic cache
- SimHash backend: 64-bit fingerprint, Hamming distance threshold
- Exclude tool_use responses (can't replay with stale tool IDs)
- Optional embedding backend (fastembed ONNX) behind feature flag

### 2.8 SQLite persistence
- Cache entries persisted periodically
- Cost events written async
- Restore on restart

**Deliverable**: 40-70% cost reduction on typical agent workloads via caching + optimization.

---

## Phase 3: Intelligent Routing

**Goal**: CascadeRouter integration — the gateway learns which model is best for each task.

### 3.1 CascadeRouter integration
- Import from `roko-learn` (existing 3-stage router)
- Feed routing context from request metadata (headers, model hints)
- Observe outcomes (success/failure, cost, latency)

### 3.2 Budget enforcement
- 4-tier degradation: Full → T1Only → Economy → Block
- Per-key budget tracking
- Automatic model downgrade when budget pressure increases

### 3.3 Provider circuit breaker
- Import `ProviderHealthRegistry` from `roko-learn`
- 3-state machine: Closed → Open → HalfOpen
- Error-class-specific cooldowns

### 3.4 Batch API routing
- Detect non-urgent requests (header hint or auto-classification)
- Queue for Anthropic/OpenAI Batch API (50% discount)
- Batch flush on timer or explicit trigger
- Status + result retrieval endpoints

### 3.5 KV affinity tracking
- Track which sessions have warm KV cache at which provider
- Prefer routing to provider with warm cache (latency + cost savings)

**Deliverable**: Gateway automatically selects cheapest model that meets quality bar, learns from outcomes.

---

## Phase 4: Safety + Analytics

### 4.1 Safety pipeline
- PII scanning: hard-block private keys/seeds, soft-mask other PII
- Injection detection: regex patterns + optional ONNX classifier
- Privacy classification: route sensitive requests to privacy-preserving providers

### 4.2 Prometheus metrics
- Request count, latency histogram, token counts, cost, cache hit rates
- Per-provider, per-model breakdowns
- Budget utilization gauges

### 4.3 WebSocket stats stream
- Real-time stats broadcast for dashboard clients

### 4.4 Analytics endpoints
- `/analytics/spend` — cost over time, by model, by key
- `/analytics/cost-history` — historical trends
- `/analytics/cache` — hit rates, savings, regime detection

### 4.5 Embedded dashboard
- Static HTML/JS dashboard served from gateway
- Cost graphs, provider health, cache analytics, live request stream

**Deliverable**: Full observability — know exactly what's being spent, where, and why.

---

## Phase 5: Billing + Multi-Tenancy

### 5.1 Stripe integration
- Subscription tiers (Free, Starter, Pro, Custom)
- Usage metering → Stripe usage records
- Overage billing
- Webhook handlers for subscription events

### 5.2 MPP micropayments
- ERC-3009 USDC on Base
- Per-request (402 challenge/response flow)
- Session mode (pre-funded balance)
- Spread calculator with tier discounts

### 5.3 Multi-tenant isolation
- Organization → project → API key hierarchy
- Per-tenant model access control
- Per-tenant rate limits + budgets
- Cache isolation between tenants

**Deliverable**: Production billing — both Stripe for humans and crypto for autonomous agents.

---

## Phase 6: HuggingFace + Self-Learning

### 6.1 HuggingFace Inference Provider
- Dedicated provider with routing policy support (`:fastest`, `:cheapest`)
- Model metadata fetching from Hub API
- Dynamic model discovery → CascadeRouter arm injection

### 6.2 Hub integration
- Model discovery: periodically query for new models matching criteria
- Auto-populate ModelProfile from model cards
- Publish learning artifacts (playbooks, episodes) to Hub

### 6.3 Dataset loading
- REST + Parquet loading for benchmarks (SWE-bench, MBPP, etc.)
- No Python dependency

### 6.4 AutoTrain loop (future)
- Trigger fine-tuning from successful episodes
- Push fine-tuned model to Hub
- Add as CascadeRouter arm
- The exponential self-improvement loop

**Deliverable**: Gateway that discovers new models, explores them, and improves itself.

---

## Phase 7: Provider Expansion

### 7.1 Additional direct providers
- DeepSeek (with off-peak discount awareness)
- Mistral (Codestral, Ministral)
- Groq (LPU inference)
- Together (fine-tuning support)
- Fireworks (fast inference)
- Cerebras (wafer-scale)
- SambaNova (long context)

### 7.2 Enterprise providers
- AWS Bedrock (SigV4 auth)
- Azure OpenAI (endpoint routing)

### 7.3 Local providers
- Ollama auto-discovery
- vLLM / TGI / llama.cpp server detection

### 7.4 Provider auto-discovery
- Query OpenRouter model metadata API
- Query HuggingFace model listings
- Auto-configure providers from discovered endpoints

**Deliverable**: 20+ providers available out of the box, extensible via config.

---

## Estimated Effort

| Phase | Estimated LOC | Priority |
|---|---|---|
| Phase 1: Core Proxy | ~3,000 | Critical (MVP) |
| Phase 2: Caching + Optimization | ~2,500 | Critical (value prop) |
| Phase 3: Intelligent Routing | ~1,500 | High (differentiator) |
| Phase 4: Safety + Analytics | ~2,000 | High (production readiness) |
| Phase 5: Billing + Multi-Tenancy | ~2,000 | Medium (monetization) |
| Phase 6: HuggingFace + Self-Learning | ~1,500 | Medium (moat) |
| Phase 7: Provider Expansion | ~1,500 | Low (incremental) |
| **Total** | **~14,000** | |

Phases 1-3 are the core product. Phases 4-5 make it production-ready. Phases 6-7 build the moat.
