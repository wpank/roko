# Gateway Goals: Desired End State

## Core Properties

### 1. Transparent Proxy + Intelligent Router
Accepts Anthropic Messages API and OpenAI Chat Completions wire format. Agents don't know they're talking to a gateway — they use their existing SDKs. The gateway adds intelligence (routing, caching, optimization) without changing the API contract.

### 2. Provider-Agnostic
Adding a new provider = adding a TOML config block (for OpenAI-compat) or implementing a small trait (for custom protocols). No code changes for standard providers.

### 3. Cybernetic Self-Optimization
The gateway gets cheaper and better over time:
- **CascadeRouter** learns which model works best for each task category, complexity, and role
- **Prompt experiments** A/B test prompt sections, converge on winners
- **Cache policies** adapt to workload patterns (regime detection)
- **Budget degradation** adjusts automatically based on utilization

### 4. Multi-Tenant with Isolation
Organizations → projects → API keys → per-agent budgets. Full cost isolation, configurable model access, per-tenant rate limits.

### 5. Dual Billing
Stripe subscriptions for humans. USDC micropayments (MPP) for autonomous agents. Both first-class.

### 6. Composable Middleware
Each optimization is an independent layer that can be enabled/disabled/configured:
- Caching (L1 hash, L2 semantic, L3 prefix)
- Safety (PII, injection, privacy)
- Optimization (tool pruning, compression, loop detection)
- Budget (per-agent, per-session, per-org)
- Analytics (cost, tokens, cache, per-model stats)

### 7. Observable
Every request produces: cost headers, cache status, provider used, tokens counted. Prometheus metrics. WebSocket live stats. Per-key analytics dashboard.

---

## Feature Checklist

### Must Have (MVP)

- [ ] HTTP proxy: POST `/v1/messages` (Anthropic) + POST `/v1/chat/completions` (OpenAI)
- [ ] Auto-detect wire format, respond in kind
- [ ] Provider trait with implementations for: Anthropic, OpenAI, Google Gemini, DeepSeek, Mistral, OpenRouter, HuggingFace, Ollama
- [ ] L1 hash cache (BLAKE3 + Moka LRU)
- [ ] L3 prefix cache injection for Anthropic
- [ ] Cost tracking per-request with response headers
- [ ] Per-key API authentication
- [ ] Per-key rate limiting
- [ ] Budget enforcement with tier degradation
- [ ] Health check + models list endpoints
- [ ] Provider failover (retry on 429/5xx, rotate keys)
- [ ] Provider circuit breaker (consecutive failure → open)
- [ ] Prometheus metrics endpoint
- [ ] Configuration via TOML + env vars + CLI flags
- [ ] Embeddable as library in roko-serve OR standalone binary

### Should Have (v1.0)

- [ ] L2 semantic cache (SimHash)
- [ ] CascadeRouter integration (3-stage model selection)
- [ ] In-flight request coalescing
- [ ] Batch API routing (non-urgent → 50% off)
- [ ] Tool pruning (strip unused tool definitions after warmup)
- [ ] Context compression (truncate old tool results)
- [ ] Loop detection + convergence detection
- [ ] Multi-key rotation for Anthropic
- [ ] WebSocket stats stream
- [ ] Analytics endpoints (spend, cost history, per-model)
- [ ] SQLite persistence for costs + cache
- [ ] Safety layer (PII scan, injection detection)
- [ ] Dashboard UI (embedded static or SSR)

### Nice to Have (v1.x)

- [ ] MPP micropayments (USDC on Base, ERC-3009)
- [ ] Session pre-funding (MPP sessions)
- [ ] Multi-tenant org/project/key hierarchy
- [ ] Stripe subscription billing integration
- [ ] HuggingFace Hub model auto-discovery
- [ ] HuggingFace dataset loading (for benchmarks)
- [ ] Provider auto-discovery from HF + OpenRouter metadata APIs
- [ ] Output budgeting (auto-cap max_tokens per model)
- [ ] Thinking cap (extended thinking budget limits)
- [ ] KV affinity tracking (route to provider with warm cache)
- [ ] A/B prompt experiments (winner auto-promotion)
- [ ] Self-funding agent mode (Bankr-style metabolic tracking)
- [ ] Semantic cache with embedding backend (fastembed ONNX)
- [ ] Privacy classification (route sensitive requests to TEE providers)
- [ ] OpenTelemetry tracing integration
- [ ] gRPC endpoint (for high-performance internal use)

### Future (v2.0+)

- [ ] Fine-tuning loop (HF AutoTrain integration)
- [ ] Dynamic model discovery → CascadeRouter arm injection
- [ ] Cross-instance learning via HF Hub (publish/pull playbooks)
- [ ] Federated caching across gateway instances
- [ ] Geographic routing (closest provider region)
- [ ] Custom model hosting via HF Inference Endpoints
- [ ] Chain witness anchoring for cost proofs
- [ ] Autonomous budget negotiation (agents negotiate their own inference budgets)

---

## Design Constraints

1. **No Python**: Everything in Rust. HF dataset loading via REST + Parquet, not `datasets` library.
2. **No framework lock-in**: Axum + Tower is fine, but core logic must be framework-agnostic (trait-based).
3. **Sub-1ms overhead**: Gateway adds <1ms latency on top of provider latency for uncached requests.
4. **Graceful degradation**: If cache fails, serve uncached. If one provider fails, try next. If analytics fails, still serve requests.
5. **Config-driven providers**: Adding a new OpenAI-compat provider = 5 lines of TOML. No code changes.
6. **Test without providers**: Mock provider for all integration tests. No real API keys in CI.
