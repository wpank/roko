# roko-gateway: Crate Architecture

## Crate Layout

```
roko-gateway/                          # Library crate (core gateway logic)
├── Cargo.toml
├── src/
│   ├── lib.rs                         # Public API: GatewayBuilder, start_server()
│   │
│   ├── config.rs                      # GatewayConfig (TOML + env + CLI)
│   ├── state.rs                       # GatewayState (shared AppState)
│   │
│   ├── server/                        # HTTP layer
│   │   ├── mod.rs                     # Axum router assembly
│   │   ├── routes.rs                  # Route handlers (messages, completions, batch, stats)
│   │   ├── format.rs                  # Wire format detection + normalization (Anthropic ↔ OpenAI)
│   │   ├── auth.rs                    # API key auth middleware
│   │   └── headers.rs                 # Response header injection (cost, cache, provider)
│   │
│   ├── provider/                      # Provider abstraction + implementations
│   │   ├── mod.rs                     # Provider trait + ProviderRegistry
│   │   ├── registry.rs               # Runtime provider registration + resolution
│   │   ├── anthropic.rs              # Anthropic Messages API
│   │   ├── openai.rs                 # OpenAI Chat Completions
│   │   ├── openai_compat.rs          # Generic OpenAI-compatible (base for many providers)
│   │   ├── gemini.rs                 # Google Gemini (native + compat paths)
│   │   ├── deepseek.rs               # DeepSeek (OpenAI-compat + off-peak awareness)
│   │   ├── mistral.rs                # Mistral (OpenAI-compat + code models)
│   │   ├── huggingface.rs            # HuggingFace Inference Providers (routing policies)
│   │   ├── openrouter.rs             # OpenRouter (catch-all + metadata API)
│   │   ├── bedrock.rs                # AWS Bedrock (SigV4 auth)
│   │   ├── azure.rs                  # Azure OpenAI (endpoint-based)
│   │   ├── perplexity.rs             # Perplexity Sonar
│   │   ├── local.rs                  # Ollama / vLLM / TGI / llama.cpp (any local)
│   │   └── health.rs                 # Circuit breaker + capability tracking
│   │
│   ├── routing/                       # Model selection
│   │   ├── mod.rs                     # Router trait + static/cascade dispatch
│   │   ├── cascade.rs                 # CascadeRouter integration (from roko-learn)
│   │   ├── failover.rs                # Provider failover chain
│   │   └── affinity.rs                # KV cache affinity tracking
│   │
│   ├── cache/                         # Multi-layer caching
│   │   ├── mod.rs                     # CacheStack (L1 → L2 → L3 pipeline)
│   │   ├── hash.rs                    # L1: BLAKE3 hash + Moka LRU
│   │   ├── semantic.rs                # L2: SimHash / embedding similarity
│   │   ├── prefix.rs                  # L3: Anthropic cache_control injection
│   │   ├── normalize.rs               # Request normalization (UUID strip, tool sort, key order)
│   │   └── coalesce.rs                # In-flight request coalescing
│   │
│   ├── cost/                          # Cost tracking + budget
│   │   ├── mod.rs                     # CostTracker
│   │   ├── pricing.rs                 # PricingTable (config-driven, auto-updatable)
│   │   ├── budget.rs                  # BudgetEnforcer (4-tier degradation)
│   │   ├── rate_limit.rs              # Per-key / per-session rate limiting
│   │   └── persistence.rs             # SQLite cost event storage
│   │
│   ├── optimize/                      # Request/response optimization
│   │   ├── mod.rs                     # OptimizationPipeline
│   │   ├── tool_prune.rs              # Strip unused tool definitions
│   │   ├── tool_compress.rs           # Compress tool schemas
│   │   ├── context_compress.rs        # Truncate old tool results
│   │   ├── output_budget.rs           # Auto-cap max_tokens
│   │   ├── thinking_cap.rs            # Extended thinking budget limits
│   │   ├── loop_detect.rs             # Retry loop / oscillation detection
│   │   └── convergence.rs             # Repeated response detection
│   │
│   ├── safety/                        # Request safety filtering
│   │   ├── mod.rs                     # SafetyPipeline
│   │   ├── pii.rs                     # PII scanning (keys, seeds, patterns)
│   │   ├── injection.rs               # Prompt injection detection
│   │   └── privacy.rs                 # Privacy classification + routing rules
│   │
│   ├── batch/                         # Batch API routing
│   │   ├── mod.rs                     # BatchQueue
│   │   ├── anthropic.rs               # Anthropic Batch API client
│   │   ├── openai.rs                  # OpenAI Batch API client
│   │   └── scheduler.rs               # Urgency classification + auto-batch routing
│   │
│   ├── billing/                       # Payment & billing
│   │   ├── mod.rs                     # BillingProvider trait
│   │   ├── stripe.rs                  # Stripe subscription management
│   │   ├── mpp.rs                     # Machine Payment Protocol (USDC micropayments)
│   │   ├── session.rs                 # Pre-funded MPP sessions
│   │   └── spread.rs                  # Markup calculation + tier discounts
│   │
│   ├── analytics/                     # Observability
│   │   ├── mod.rs                     # GatewayStats (atomic counters)
│   │   ├── prometheus.rs              # Prometheus metrics exporter
│   │   ├── websocket.rs               # Live stats WebSocket stream
│   │   └── dashboard.rs               # Embedded dashboard UI
│   │
│   └── tenant/                        # Multi-tenancy
│       ├── mod.rs                     # TenantResolver
│       ├── org.rs                     # Organization → projects → keys
│       ├── limits.rs                  # Per-tenant rate limits + model access
│       └── isolation.rs               # Cache + budget isolation
```

## Key Traits

### `Provider`

```rust
/// A backend LLM provider that can forward proxy requests.
#[async_trait]
pub trait Provider: Send + Sync + 'static {
    /// Unique provider ID (e.g., "anthropic", "openai", "deepseek").
    fn id(&self) -> &str;

    /// Check if this provider handles the given model slug.
    fn can_handle(&self, model: &str) -> bool;

    /// Declared capabilities for this provider.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Forward a normalized proxy request to the upstream provider.
    /// Returns the raw HTTP response (streaming or buffered).
    async fn forward(
        &self,
        request: ProxyRequest,
        config: &ProviderConfig,
    ) -> Result<ProxyResponse, ProviderError>;

    /// Health check — ping the provider (optional, default: infer from forward() errors).
    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Unknown
    }
}

pub struct ProviderCapabilities {
    pub chat: bool,
    pub stream: bool,
    pub tools: bool,
    pub vision: bool,
    pub thinking: bool,
    pub batch: bool,
    pub cache: bool,
    pub embeddings: bool,
    pub json_mode: bool,
    pub max_context: usize,
    pub max_output: usize,
}
```

### `Router`

```rust
/// Selects which model + provider to use for a request.
#[async_trait]
pub trait Router: Send + Sync + 'static {
    /// Pick the best model for this request context.
    async fn route(&self, ctx: &RoutingContext) -> RoutingDecision;

    /// Record the outcome of a routed request (for learning).
    async fn observe(&self, outcome: &RoutingOutcome);
}

pub struct RoutingContext {
    pub requested_model: Option<String>,
    pub task_category: Option<String>,
    pub complexity: Option<f32>,
    pub role: Option<String>,
    pub budget_remaining: Option<f64>,
    pub cache_affinity: Option<String>,
}

pub struct RoutingDecision {
    pub model: String,
    pub provider: String,
    pub fallback_chain: Vec<(String, String)>,  // (model, provider) pairs
    pub stage: RoutingStage,                     // Static | Confidence | UCB
    pub reason: String,
}
```

### `CacheLayer`

```rust
/// One layer of the cache stack.
#[async_trait]
pub trait CacheLayer: Send + Sync + 'static {
    /// Look up a cached response.
    async fn get(&self, request: &NormalizedRequest) -> Option<CachedResponse>;

    /// Store a response in cache.
    async fn put(&self, request: &NormalizedRequest, response: &ProxyResponse);

    /// Cache statistics.
    fn stats(&self) -> CacheStats;
}
```

### `Optimizer`

```rust
/// A request/response optimization pass.
pub trait Optimizer: Send + Sync + 'static {
    /// Transform the request before forwarding (e.g., prune tools).
    fn optimize_request(&self, request: &mut ProxyRequest, session: &SessionState);

    /// Transform the response before returning (e.g., inject headers).
    fn optimize_response(&self, response: &mut ProxyResponse, session: &SessionState);
}
```

### `BillingProvider`

```rust
/// Payment/billing backend.
#[async_trait]
pub trait BillingProvider: Send + Sync + 'static {
    /// Check if this request is authorized to proceed.
    async fn authorize(&self, key: &ApiKey, cost_estimate: f64) -> BillingDecision;

    /// Record actual cost after completion.
    async fn record(&self, key: &ApiKey, cost: CostEvent);
}

pub enum BillingDecision {
    Approved,
    ApprovedWithDegradation(BudgetTier),
    PaymentRequired { challenge: PaymentChallenge },  // HTTP 402 for MPP
    Denied { reason: String },
}
```

## Data Flow

```
Client Request (Anthropic or OpenAI wire format)
  │
  ▼
┌─ Auth Middleware ─────────────────────────────────┐
│  Validate API key, resolve tenant, check rate limit│
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Format Detection ────────────────────────────────┐
│  Detect Anthropic vs OpenAI, normalize to internal │
│  ProxyRequest. Preserve original format for response│
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Safety Pipeline ─────────────────────────────────┐
│  PII scan → Injection detect → Privacy classify    │
│  Block or warn, never modify silently              │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Cache Stack ─────────────────────────────────────┐
│  L1 hash → L2 semantic → check in-flight coalesce │
│  HIT: return cached, skip provider                 │
│  MISS: continue to routing                         │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Router ──────────────────────────────────────────┐
│  If model specified: validate, find provider       │
│  If no model: CascadeRouter selects (task/budget)  │
│  Apply budget degradation (haiku if budget tight)  │
│  Check circuit breaker health                      │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Optimization Pipeline ───────────────────────────┐
│  Tool prune → schema compress → context compress   │
│  → output budget → thinking cap → loop detect      │
│  → L3 prefix cache injection                       │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Billing Check ───────────────────────────────────┐
│  Estimate cost → authorize (Stripe or MPP)         │
│  Approved: continue. 402: return payment challenge │
│  Degraded: route to cheaper model                  │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Provider Forward ────────────────────────────────┐
│  Forward to upstream provider (reqwest)            │
│  Handle streaming (SSE passthrough)                │
│  Retry on 429/5xx (key rotation, failover chain)   │
│  Timeout enforcement (connect + TTFT + total)      │
└───────────────────────────────────────────────────┘
  │
  ▼
┌─ Response Processing ─────────────────────────────┐
│  Count tokens → compute cost → inject headers      │
│  Record to cost DB → update cache → update stats   │
│  Observe outcome → CascadeRouter learns            │
│  Convert back to client's original wire format     │
└───────────────────────────────────────────────────┘
  │
  ▼
Client Response (same wire format as request)
```

## Integration with roko-serve

The gateway is a library crate. `roko-serve` mounts it as an optional feature:

```rust
// In roko-serve/src/lib.rs
#[cfg(feature = "gateway")]
{
    let gateway = roko_gateway::GatewayBuilder::new()
        .config(gateway_config)
        .router(cascade_router)           // From roko-learn
        .cost_table(cost_table)           // From roko-learn
        .provider_health(health_registry) // From roko-learn
        .build()?;

    // Mount gateway routes under /gateway/v1/
    router = router.nest("/gateway/v1", gateway.router());
}
```

The gateway can also run standalone:

```bash
# Standalone binary
cargo run -p roko-gateway -- --port 4000 --config gateway.toml

# Or via roko CLI
roko gateway serve --port 4000
```

## Configuration (gateway.toml)

```toml
[gateway]
port = 4000
bind = "0.0.0.0"
max_concurrent = 256
max_body_size = "10MiB"

[gateway.auth]
api_key_env = "ROKO_GATEWAY_API_KEY"
# Or generate random at startup if not set

[gateway.cache]
l1_capacity = 10000
l1_ttl_secs = 3600
l2_enabled = true
l2_backend = "simhash"       # "simhash" | "embedding"
l2_threshold = 3             # Hamming distance for simhash
l3_prefix_cache = true
coalesce_inflight = true
persistence = "sqlite"        # "sqlite" | "none"
persistence_path = ".roko/gateway/cache.db"

[gateway.cost]
persistence_path = ".roko/gateway/costs.db"
flush_interval_secs = 30

[gateway.budget]
default_per_key_usd = 100.0
degradation_tiers = [
    { threshold = 0.70, action = "warn" },
    { threshold = 0.80, action = "route_cheaper" },
    { threshold = 0.95, action = "economy_only" },
    { threshold = 1.00, action = "block" },
]

[gateway.rate_limit]
per_key_rpm = 120
per_key_tpm = 1_000_000

[gateway.safety]
pii_scan = true
injection_detect = true
# privacy_rules = "..." (optional)

[gateway.batch]
enabled = true
urgency_header = "X-Roko-Urgency"  # "realtime" | "background"
auto_batch_threshold_secs = 5       # Requests older than this → batch

[gateway.billing]
stripe_enabled = false
mpp_enabled = false
mpp_spread = 0.20
mpp_recipient = "0x..."
mpp_chain_id = 8453  # Base

# Provider configs (same format as roko.toml [providers])
[gateway.providers.anthropic]
kind = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
extra_keys = ["ANTHROPIC_API_KEY_2", "ANTHROPIC_API_KEY_3"]
max_concurrent = 20
timeout_ms = 120000

[gateway.providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"
max_concurrent = 20

[gateway.providers.deepseek]
kind = "openai_compat"
base_url = "https://api.deepseek.com/v1"
api_key_env = "DEEPSEEK_API_KEY"
max_concurrent = 10

[gateway.providers.gemini]
kind = "gemini"
api_key_env = "GOOGLE_API_KEY"
max_concurrent = 10

[gateway.providers.huggingface]
kind = "openai_compat"
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
max_concurrent = 20

[gateway.providers.openrouter]
kind = "openai_compat"
base_url = "https://openrouter.ai/api/v1"
api_key_env = "OPENROUTER_API_KEY"
max_concurrent = 20
priority = 99  # Fallback

[gateway.providers.ollama]
kind = "openai_compat"
base_url = "http://localhost:11434/v1"
max_concurrent = 4
priority = 100  # Local fallback

# Model profiles (pricing, capabilities)
[gateway.models.claude-opus-4-6]
provider = "anthropic"
cost_input_per_m = 5.00
cost_output_per_m = 25.00
cost_cache_read_per_m = 0.50
context_window = 200000
supports_tools = true
supports_thinking = true
supports_vision = true
supports_cache = true

# ... (more model profiles)
```

## Dependencies

```toml
[dependencies]
# HTTP
axum = { version = "0.8", features = ["ws", "macros"] }
tower = { version = "0.5", features = ["timeout", "limit", "load-shed"] }
tower-http = { version = "0.6", features = ["cors", "trace", "limit"] }
reqwest = { version = "0.12", features = ["rustls-tls", "stream", "gzip"] }
hyper = { version = "1", features = ["full"] }

# Cache
moka = { version = "0.12", features = ["future"] }
blake3 = "1"

# Persistence
rusqlite = { version = "0.31", features = ["bundled"] }

# Concurrency
tokio = { version = "1", features = ["full"] }
dashmap = "6"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Metrics
prometheus = "0.13"

# Crypto (for MPP)
alloy = { version = "0.12", features = ["signers"], optional = true }

# Billing (optional)
# stripe-rs behind feature flag

[features]
default = ["cache-sqlite"]
cache-sqlite = ["rusqlite"]
semantic-embedding = ["fastembed"]  # ONNX embedding backend for L2 cache
safety-onnx = ["ort", "tokenizers"]  # ONNX injection classifier
mpp = ["alloy"]                      # USDC micropayments
stripe = ["stripe-rs"]               # Stripe billing
full = ["cache-sqlite", "semantic-embedding", "safety-onnx", "mpp", "stripe"]
```

## Testing Strategy

```
tests/
├── mock_provider.rs      # In-memory provider for integration tests
├── wire_format.rs         # Anthropic ↔ OpenAI conversion roundtrips
├── cache_stack.rs         # L1/L2/L3 cache behavior
├── routing.rs             # Router decisions with mock providers
├── budget.rs              # Budget degradation tiers
├── safety.rs              # PII/injection blocking
├── coalesce.rs            # In-flight coalescing
├── batch.rs               # Batch queue scheduling
├── integration/
│   ├── proxy_anthropic.rs # End-to-end with mock Anthropic server
│   ├── proxy_openai.rs    # End-to-end with mock OpenAI server
│   ├── streaming.rs       # SSE streaming passthrough
│   └── failover.rs        # Provider failover chain
```

No real API keys in CI. All integration tests use mock HTTP servers (via `axum::Server` on localhost with random ports).
