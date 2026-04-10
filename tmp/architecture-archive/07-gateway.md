# Inference gateway

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---

Agents never hold API keys. A centralized `InferenceGateway` inside the roko process owns all secrets, runs every request through a multi-stage pipeline, and calls providers. The gateway is designed as a standalone, reusable system -- it handles caching, cost tracking, loop detection, output budgeting, tool pruning, convergence detection, thinking caps, and batch submission. The `CascadeRouter` from `roko-learn` handles model selection upstream; the gateway handles everything after a model is chosen.

Crate: `crates/roko-gateway/`

### Pipeline overview

Every inference request passes through these stages in order:

```
                              InferenceRequest
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  1. Loop detection   │  Ring buffer of recent tool calls.
                          │     (per-session)    │  Retry / oscillation / drift check.
                          └──────────┬──────────┘
                                     │ pass
                                     ▼
                          ┌─────────────────────┐
                          │  2. Cache lookup     │  L1 hash (blake3) → L2 semantic
                          │     (L1 → L2)       │  (SimHash, Hamming ≤ 3).
                          └──────────┬──────────┘
                               hit / │ miss
                          ┌─────┐    │
                          │return│    │
                          └─────┘    ▼
                          ┌─────────────────────┐
                          │  3. Tool pruning     │  Remove unused tool schemas.
                          │     (per-session)    │  Never prunes core tools.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  4. Output budget    │  EMA-based max_tokens cap.
                          │     (per-model)      │  p95 x 1.5, floor 1024.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  5. Thinking cap     │  Per-model thinking budget.
                          │     (per-model)      │  Only when thinking enabled.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  6. Convergence      │  SimHash of recent responses.
                          │     detection        │  3+ similar → inject guidance.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  7. Provider call    │  ProviderBackend::complete()
                          │                      │  or ::stream().
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  8. Cache store      │  Write to L1 + L2 (unless
                          │                      │  excluded by cache policy).
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  9. Cost tracking    │  Compute actual vs naive cost.
                          │                      │  Record per-agent, per-model.
                          └──────────┬──────────┘
                                     │
                                     ▼
                              InferenceResponse
```

### 1. Protocol types

Core types that every subsystem shares.

```rust
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ToolSchema>>,
    pub stream: bool,
    pub thinking: Option<ThinkingConfig>,
    pub metadata: InferenceMeta,
}

pub struct InferenceMeta {
    pub session_id: String,
    pub agent_id: AgentId,
    pub tier: Tier,              // T0, T1, T2
    pub budget_remaining: u64,   // microdollars
}

pub struct InferenceResponse {
    pub text: String,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
    pub model: String,
    pub latency_ms: u64,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub thinking_tokens: u64,       // Anthropic extended thinking
    pub reasoning_tokens: u64,      // OpenAI reasoning tokens
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    ContentFilter,
}

#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

All types derive `Serialize` + `Deserialize`. `TokenUsage` implements `Add` for aggregation across a session.

### 2. Hash cache (L1)

Exact-match cache. Fast path for repeated identical requests.

**How it works**: Hash the normalized request body with blake3, look up in a moka async LRU cache. If the hash matches, return the cached response without calling a provider.

**Normalization** (applied before hashing):
- Strip UUIDs matching `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}`
- Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers
- Replace git status blocks with `[GIT_STATUS]` placeholder
- Sort JSON keys alphabetically
- Sort tool definitions by name

This ensures that two requests differing only in timestamps or working-directory metadata produce the same hash.

**Cache entry**:

```rust
pub struct CachedResponse {
    pub body: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub cached_at: Instant,
    pub effective_ttl: Duration,
}
```

**Regime-aware TTL**: The system's cortical state controls how long cache entries live.

| Regime | TTL | Rationale |
|--------|-----|-----------|
| Normal | 3600s | Standard operating conditions. |
| Calm | 7200s | Low activity -- cached responses stay valid longer. |
| Volatile | 900s | Rapid changes -- cache expires faster to avoid stale responses. |
| Crisis | 300s | Active failures -- almost no caching, maximize freshness. |

**Exclusions** (never cached):
- Responses containing `tool_use` stop reason (tool call IDs are ephemeral)
- Responses with fewer than 3 output tokens (too short to be useful)
- Error responses

**Storage**: `moka::future::Cache<[u8; 32], CachedResponse>` with configurable max capacity (default 10,000 entries).

### 3. Semantic cache (L2)

Near-miss cache. Catches requests that are semantically equivalent but textually different.

**How it works**: Compute a 64-bit SimHash fingerprint of the request text. Compare against stored fingerprints using Hamming distance. A distance of 3 bits or fewer counts as a cache hit.

**SimHash algorithm**:
1. Tokenize request text (whitespace + punctuation boundaries)
2. Hash each token with a fast 64-bit hash
3. For each bit position: if the token hash has a 1, increment a counter; if 0, decrement
4. Final fingerprint: 1 for each positive counter, 0 for each negative

**Storage**: `DashMap<u64, SimHashEntry>` for lock-free concurrent reads.

```rust
pub struct SimHashEntry {
    pub response: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,
}
```

**Parameters**:
- Max entries: 5,000
- TTL: 7,200s (fixed, not regime-aware -- semantic matches are fuzzier so the TTL is conservative)
- Eviction: LRU by age when capacity reached
- Hamming threshold: 3 bits (configurable)

**Namespace isolation**: Each tenant/workspace prefixes its cache text with a namespace identifier. This prevents cross-tenant cache hits in multi-user deployments. A `default` namespace is used for single-user setups.

**Exclusions**: Same as L1 -- no tool_use, no sub-3-token, no errors.

### 4. Provider backends and key rotation

Each LLM provider implements a `ProviderBackend` trait:

```rust
#[async_trait]
pub trait ProviderBackend: Send + Sync {
    fn name(&self) -> &str;
    fn supports_model(&self, model: &str) -> bool;
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: &InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

**Anthropic backend** (`POST https://api.anthropic.com/v1/messages`):
- Streaming via SSE
- Tool use with full schema
- Extended thinking (`thinking.type = "enabled"`, `thinking.budget_tokens`)
- Prefix caching: system block annotated with `cache_control: {"type": "ephemeral", "ttl": "1h"}`
- Extracts `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens` from response usage

**OpenAI backend** (`POST https://api.openai.com/v1/chat/completions`):
- Format translation: Anthropic message format <-> OpenAI chat format
- Reasoning token extraction from `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens`
- Model routing: handles `gpt-*`, `o1`, `o3-*`, `o4-*`

**Key rotation**: Each provider holds a `Vec<String>` of API keys. On a 429 (rate limit) response, the provider rotates to the next key in the list. An `AtomicUsize` index tracks the active key. Rotation is lock-free.

```rust
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    pub fn current(&self) -> &str {
        let idx = self.active.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    pub fn rotate(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }
}
```

**Provider resolution order**: Anthropic for `claude-*` models, OpenAI for `gpt-*/o1/o3-*/o4-*`. Additional providers (Gemini, Perplexity, Ollama, OpenRouter) use the existing `roko-agent` backends and are registered by config.

### 5. Cost computation

Per-request cost calculation with actual vs naive pricing comparison.

**Pricing table**: `HashMap<String, ModelPricing>` loaded from config. Supports substring matching for model families (e.g., `claude-sonnet` matches `claude-sonnet-4-20250514`).

```rust
pub struct ModelPricing {
    pub input_per_m: f64,          // USD per 1M input tokens
    pub output_per_m: f64,         // USD per 1M output tokens
    pub cached_input_per_m: f64,   // USD per 1M cached input tokens
    pub reasoning_per_m: f64,      // USD per 1M reasoning/thinking tokens
}
```

Default fallback: $3/M input, $15/M output (covers unknown models without crashing).

**Cost formula** (per request):

```
fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6
cached_input  = cache_read_tokens * cached_input_per_m / 1e6
cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge
regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6
reasoning     = reasoning_tokens * reasoning_per_m / 1e6
thinking      = thinking_tokens * output_per_m / 1e6

actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking
```

**Batch discount**: Requests submitted through the batch API get a 50% reduction on `actual_cost`.

**Naive cost**: What the provider would charge with no caching at all:

```
naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6
```

**Savings**: `naive_cost - actual_cost`. Tracked per request and aggregated per agent, per session, and per model for dashboard display.

**Attribution**: Every cost record includes `agent_id` and `session_id`. This feeds the Treasury / Cost page in the dashboard and the per-agent cost breakdowns.

### 6. Loop detection

Detects three patterns of agent loops and injects corrective guidance before the agent wastes more tokens.

**Per-session state**:

```rust
pub struct SessionLoopState {
    recent_calls: VecDeque<(String, [u8; 32])>,  // (tool_name, blake3(args))
    consecutive_identical: u32,
    tokens_since_progress: u64,
}
```

Ring buffer capacity: 16 entries. Does not grow.

**Detection rules**:

| Pattern | Trigger | Injected guidance |
|---------|---------|-------------------|
| Retry | Same tool + same args hash called 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." |
| Oscillation | A -> B -> A -> B pattern repeats 3+ full cycles | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." |
| Drift | 15,000+ output tokens accumulated without new `tool_result` content | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |

**Injection mechanism**: The guidance string is prepended to the system prompt on the next request. It appears once and clears itself.

**Counters**: `loops_detected`, `loop_injections`, `loop_retry_detected`, `loop_oscillation_detected`, `loop_drift_detected`. All exposed via the stats endpoint.

### 7. Output budgeting

Prevents runaway output by auto-setting `max_tokens` based on observed behavior.

**Per-model tracking**:

```rust
pub struct ModelOutputStats {
    pub ema: f64,           // exponential moving average of output tokens
    pub ema_sq: f64,        // EMA of squared output tokens (for variance)
    pub max_seen: u64,      // highest output observed
    pub count: u64,         // total observations
}
```

**Algorithm**:
- Alpha: 0.05 (5% weight to new observations)
- Minimum samples: 20 before p95 estimation is trusted
- p95 estimate: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2 standard deviations)
- Cap: `p95 * 1.5`, with a floor of 1,024 tokens

**Behavior**:
- When a request has no `max_tokens` set, the gateway auto-sets it to the computed cap
- When a request has an unreasonably high `max_tokens` (above 2x the cap), the gateway reduces it to the cap
- When a request has an explicit `max_tokens` that is *below* the cap, the gateway does not touch it

**Counters**: `output_budgets_applied`, `output_tokens_bounded`.

### 8. Tool pruning

Removes unused tool schemas from requests to reduce input token count. Tool schemas are verbose (often 200-500 tokens each), and most sessions use a small subset.

**Usage tracking**: Two maps:
- Per-session: `HashMap<String, u32>` -- how many times each tool was called in this session
- Global: `HashMap<String, u64>` -- how many times each tool has been called across all sessions

**Never-prune list** (core tools that must always be available):
`Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `TaskCreate`, `TaskUpdate`, `TaskList`, `Agent`, `SendMessage`

**Two-tier pruning**:

| Tier | Trigger | Logic |
|------|---------|-------|
| Session (Tier 1) | 50+ requests in the current session | Remove tools never used in this session. Protected + used tools survive. |
| Global (Tier 2) | < 50 session requests but 50+ total global requests | Remove tools never used by any session. Catches tools that are defined but universally ignored. |

**Metrics**: `tools_pruned` count, `tool_tokens_saved` estimate (removed schemas x average schema size of ~300 tokens).

### 9. Convergence detection

Detects when an agent is producing repetitive responses and needs a nudge.

**Per-session state**:

```rust
pub struct ConvergenceState {
    recent_hashes: VecDeque<u64>,  // last 8 response SimHashes
    consecutive_similar: u32,
}
```

**Detection**: After each response, compute its SimHash. Compare to the previous response's SimHash via Hamming distance. If the distance is 2 bits or fewer, increment `consecutive_similar`. Three or more consecutive similar responses triggers convergence.

**Injection**: On the next request, prepend: "Your recent responses are converging. Try a different angle or move to the next step."

A dissimilar response (Hamming > 2) resets the counter to zero.

**Counters**: `convergence_detected`, `convergence_injections`.

### 10. Thinking cap

Per-model defaults for extended thinking budgets. Prevents agents from using unbounded thinking tokens when the budget is unset.

| Model family | Default thinking budget |
|-------------|------------------------|
| Opus | 32,768 tokens |
| Sonnet | 16,384 tokens |
| Haiku | 4,096 tokens |

**Rules**:
- Activates only when thinking is already enabled (`thinking.type = "enabled"`) but `budget_tokens` is absent
- Never forces thinking on. If thinking is disabled, the cap does nothing.
- Never overrides explicit user budgets. If the user sets `budget_tokens: 8192`, the cap does not increase it.

**Counters**: `thinking_budgets_applied`, `thinking_tokens_capped_estimate`.

### 11. Batch API

Queues inference requests for asynchronous batch processing at a 50% cost discount. Useful for non-time-sensitive work: plan generation, research, code review.

**Queue behavior**:
- Requests submitted via `POST /api/gateway/batch/submit` return `202 Accepted` with a `custom_id` (`roko-{uuid}`)
- Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed
- Manual flush: `POST /api/gateway/batch/flush`

**Submission**: On flush, the gateway submits the batch to `POST https://api.anthropic.com/v1/messages/batches`.

**Polling**: Background task polls `GET /v1/messages/batches/{batch_id}` every 60 seconds until the batch completes.

**Results**: Stored in `DashMap<String, BatchResult>` keyed by `custom_id`. Retrieved via `GET /api/gateway/batch/result/{custom_id}`.

**Preprocessing**: Batch requests go through the same pipeline stages as real-time requests (prefix caching, output budget, tool pruning). Cost calculation applies the 50% batch discount.

### Gateway HTTP routes

```
POST   /api/gateway/inference         Main inference proxy endpoint.
                                       Auth required (agent token).
                                       Runs full pipeline.
                                       Returns InferenceResponse.

GET    /api/gateway/stats             Aggregate gateway statistics:
                                       cache hit rates, total cost,
                                       active sessions, loop detections,
                                       convergence events, tool pruning savings.

GET    /api/gateway/ws                WebSocket endpoint streaming per-request
                                       StatsEvents in real time.
                                       Broadcast channel (1024 slot capacity).

POST   /api/gateway/batch/submit      Queue a request for batch processing.
                                       Returns 202 + custom_id.

POST   /api/gateway/batch/flush       Force-flush the current batch queue.

GET    /api/gateway/batch/result/:id  Retrieve completed batch result by
                                       custom_id.
```

**StatsEvent** (broadcast on the WebSocket per completed request):

```rust
pub struct StatsEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    pub naive_cost_usd: f64,
    pub savings_usd: f64,
    pub cache_hit: bool,
    pub elapsed_ms: u64,
    pub session_id: String,
    pub gateway_actions: Vec<String>,  // e.g., ["output_budget", "tool_prune"]
}
```

### InferenceHandle

In-process agents get an `InferenceHandle` -- a channel sender that communicates with the gateway without holding any secrets.

```rust
/// Handle given to agents for inference requests.
/// Contains no API keys -- only a channel sender.
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceRequest>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,  // remaining budget in microdollars
}

impl InferenceHandle {
    /// Send an inference request and await the response.
    pub async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to: tx,
        }).await?;
        rx.await?
    }

    /// Stream an inference response (for LLM output).
    pub async fn infer_stream(
        &self,
        request: InferenceRequest,
    ) -> Result<impl Stream<Item = InferenceChunk>> {
        let (tx, rx) = mpsc::channel(64);
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to_stream: tx,
        }).await?;
        Ok(ReceiverStream::new(rx))
    }

    /// Remaining budget in microdollars.
    pub fn remaining_budget(&self) -> u64 {
        self.budget.load(Ordering::Relaxed)
    }
}
```

### 12. Concurrency and backpressure

The gateway enforces concurrency limits at three levels to prevent overload.

**Per-provider concurrency**:

```
Provider      Max concurrent requests
────────      ──────────────────────
Anthropic     50
OpenAI        50
Gemini        30
Perplexity    20
Ollama        4  (local hardware bound)
OpenRouter    50
```

Requests beyond the provider limit queue in a bounded channel. The channel depth is 2x the concurrency limit (e.g., 100 for Anthropic). If the channel is full, the gateway returns `503 Service Unavailable` immediately.

**Per-agent queue depth**: Each agent can have at most 8 in-flight requests (queued + executing). Request number 9 receives:

```json
HTTP 429 Too Many Requests
Retry-After: 2

{ "error": "agent_queue_full", "agent_id": "coder-1", "max_depth": 8 }
```

The agent should use exponential backoff: 2s, 4s, 8s, capped at 30s.

**Global queue**: 200 total requests across all agents and providers. When the global queue is full:

```json
HTTP 503 Service Unavailable
Retry-After: 5

{ "error": "gateway_overloaded", "queued": 200, "active": 184 }
```

**Monitoring**: The `/api/gateway/stats` endpoint includes `queue_depth`, `active_requests`, and `rejected_count` per provider.

### 13. CascadeRouter fallback chain

The `CascadeRouter` returns a ranked list of models, not a single choice:

```rust
pub struct RouteDecision {
    pub preferred: String,      // e.g., "claude-sonnet-4-20250514"
    pub fallback_1: String,     // e.g., "claude-haiku-4-20250514"
    pub fallback_2: String,     // e.g., "gpt-4o-mini"
}
```

The gateway tries each model in order. If the preferred model is unavailable (provider down, rate limited, or timed out after 30s), it falls through to the next.

```
preferred ──► call provider
                │
           success ──► return response
                │
           failure (429 / 503 / timeout)
                │
                ▼
fallback_1 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
fallback_2 ──► call provider
                │
           success ──► return response
                │
           failure
                │
                ▼
           return 503 to agent
```

**Default fallback hierarchies** (used when the router has insufficient data to rank):

```
Anthropic chain:   Opus → Sonnet → Haiku
OpenAI chain:      GPT-4o → GPT-4o-mini
Cross-provider:    Sonnet → GPT-4o → Haiku
```

**Fallback metadata**: When a fallback model serves the request, the response includes `"fallback": true` and `"original_model": "claude-sonnet-4-..."` so the agent and the learning system know what happened. The router records the fallback event to adjust future routing weights.

### CascadeRouter integration

The gateway uses the existing `CascadeRouter` from `roko-learn` for model selection. The router picks the model; the gateway handles everything after that.

```rust
impl InferenceGateway {
    async fn route_request(&self, envelope: InferenceEnvelope) -> Result<()> {
        // 1. Select model via CascadeRouter
        let model = self.cascade_router.select_model(
            &envelope.request.task_type,
            envelope.request.tier,
            &envelope.agent_id,
        );

        // 2. Stamp model onto request
        let mut request = envelope.request;
        request.model = model.clone();

        // 3. Run through gateway pipeline
        //    loop_check -> cache_lookup -> tool_prune -> output_budget
        //    -> thinking_cap -> convergence_check -> provider_call
        //    -> cache_store -> cost_track
        let response = self.pipeline.execute(request).await?;

        // 4. Update router weights from quality signal
        self.cascade_router.record_outcome(
            &model,
            &envelope.request.task_type,
            &response.quality_signal,
        );

        // 5. Publish cost update to relay
        self.relay.publish_cost_update(
            &envelope.agent_id,
            response.usage.total_cost_microdollars,
        ).await;

        envelope.respond(response);
        Ok(())
    }
}
```

### Proxying for isolated agents

Remote agents (Fly Machines, Railway containers) don't have direct access to the inference gateway's channel. They make HTTPS requests to the parent's proxy endpoint:

```
POST /api/inference/proxy
Authorization: Bearer <agent_token>
Content-Type: application/json

{
  "agent_id": "isolated-coder-1",
  "model_hint": "auto",
  "tier": "t1",
  "messages": [ ... ],
  "tools": [ ... ],
  "max_tokens": 4096
}
```

The proxy endpoint validates the agent token, deducts from the agent's budget, and forwards the request through the same gateway pipeline. The agent never sees API keys.
