# Gateway: Known Issues & Design Decisions

## Issues Inherited from Bardo Gateway

### I1: Monolithic AppState
**Problem**: Bardo's `AppState` holds 30+ fields in one struct. No trait boundaries between cache, cost, routing, safety.
**Resolution**: Split into focused structs behind traits. `GatewayState` composes `CacheStack`, `CostTracker`, `ProviderRegistry`, `Router`, `SafetyPipeline`, etc. Each injectable/mockable.

### I2: Hardcoded Pricing Table
**Problem**: Model prices hardcoded in Rust source. Stale within weeks of provider price changes.
**Resolution**: Config-driven pricing in TOML. Optional auto-update from provider APIs (OpenRouter metadata, HF model cards). Fallback defaults only for unknown models.

### I3: No Multi-Tenant Isolation
**Problem**: Single API key namespace. No org/project/team boundaries.
**Resolution**: Hierarchical tenancy: Org → Project → API Key → Agent. Cache, budget, rate limits all isolated per tenant.

### I4: Provider Trait Too Thin
**Problem**: Just `can_handle()` + `forward()`. No structured capability negotiation, no health reporting.
**Resolution**: Rich `Provider` trait with `capabilities()`, `health_check()`, and typed `ProviderCapabilities` struct (chat, stream, tools, vision, thinking, batch, cache, embed, json_mode, context limits).

### I5: Domain-Specific Logic in Core
**Problem**: Venice privacy classification has DeFi-specific keyword rules. Bankr has self-funding inference semantics.
**Resolution**: Privacy classification → pluggable `PrivacyClassifier` trait. Self-funding → separate optional module, not in core proxy path.

### I6: SQLite Single-Writer Bottleneck
**Problem**: All cost writes funnel through one SQLite connection.
**Resolution**: Write-ahead log (WAL) mode + batch inserts via async channel. Or switch to per-shard SQLite files for multi-writer. Consider DuckDB for analytics queries.

### I7: No Graceful Degradation on Cache Failure
**Problem**: If SQLite cache persistence fails, unclear what happens.
**Resolution**: Explicit fallback: cache persistence failure → log warning, continue with in-memory only. Gateway always serves requests even if analytics/persistence is degraded.

---

## Design Decisions to Make

### D1: Wire Format Canonical Form
**Options**:
- (a) Normalize everything to Anthropic Messages internally (bardo approach)
- (b) Normalize everything to OpenAI Chat Completions internally
- (c) Keep a neutral internal representation, convert at edges

**Recommendation**: (c) — neutral `ProxyRequest`/`ProxyResponse` types. Both Anthropic and OpenAI wire formats are lossy translations of each other. A neutral form preserves full fidelity from either direction.

### D2: Streaming Architecture
**Options**:
- (a) Buffer entire response, then forward (simple, high latency)
- (b) Stream-through with interception (low latency, complex)
- (c) Hybrid: stream-through with async tap for cost/cache (low latency, moderate complexity)

**Recommendation**: (c) — stream bytes to client immediately, tap the stream for token counting and cache population asynchronously.

### D3: Cache Invalidation Strategy
**Options**:
- (a) TTL only (simple, stale responses possible)
- (b) TTL + model version invalidation (invalidate when model updates)
- (c) Regime-aware TTL (bardo approach — shorter TTL in volatile conditions)

**Recommendation**: (c) — regime-aware TTL. Agent workloads have natural phases (exploration, exploitation, debugging). Cache lifetime should adapt.

### D4: How to Expose CascadeRouter to Gateway
**Options**:
- (a) Gateway owns its own router (independent of roko-learn)
- (b) Gateway accepts `Box<dyn Router>`, roko-serve injects CascadeRouter
- (c) Gateway depends on roko-learn directly

**Recommendation**: (b) — trait injection. The gateway is a library; the host (roko-serve or standalone binary) provides the router implementation. This keeps the gateway independent of roko's learning crates.

### D5: Standalone vs Embedded
**Options**:
- (a) Standalone binary only (simpler, separate process)
- (b) Library only (embedded in roko-serve)
- (c) Both (library crate + thin binary wrapper)

**Recommendation**: (c) — both. Library crate for embedding, `[[bin]]` target for standalone. Same code, two entry points.

### D6: Provider Registration
**Options**:
- (a) Static provider list (hardcoded in code)
- (b) Config-driven (TOML defines providers, gateway instantiates)
- (c) Plugin-based (dynamic loading via shared libraries)

**Recommendation**: (b) — config-driven. Built-in provider types (Anthropic, OpenAI, OpenAI-compat, Gemini) cover 95% of cases. New providers = new TOML block. Custom protocol = implement `Provider` trait and register.

### D7: Pricing Source of Truth
**Options**:
- (a) Hardcoded in source (bardo approach)
- (b) TOML config (model profiles)
- (c) Auto-fetched from provider APIs (OpenRouter, HF)
- (d) Combination: config as primary, auto-fetch as discovery

**Recommendation**: (d) — config primary, auto-fetch for discovery. Config is the source of truth. Optional background job queries OpenRouter metadata API and HF model cards, proposes new entries that require human approval (or auto-merge for price updates on known models).

---

## Risks

### R1: Streaming Complexity
Streaming SSE passthrough with interception (for token counting) is the hardest part of the proxy. Need to handle: partial JSON in SSE data fields, provider-specific event formats, connection drops mid-stream, backpressure. Budget significant testing time.

### R2: Provider API Drift
Providers change their APIs. Anthropic has versioned their Messages API 4 times. OpenAI changes response shapes. Need a provider version negotiation strategy and defensive parsing.

### R3: Cache Correctness
Semantic cache can return wrong responses if the similarity threshold is too loose. Tool_use responses with stale tool IDs will break agents. Conservative defaults + exclusion rules are critical. SimHash with Hamming distance 3 is well-tested from bardo.

### R4: Multi-Provider Key Management
Users need to bring their own API keys (for direct API tiers) or use Nunchi's pooled keys (for subscription tiers). Need clear key isolation — one customer's key must never be used for another customer's request.

### R5: Latency Budget
The gateway adds latency. Target: <1ms for uncached requests (auth + routing + forward). Caching should reduce total latency (no provider round trip). Anything >5ms overhead is unacceptable — agents are latency-sensitive.
