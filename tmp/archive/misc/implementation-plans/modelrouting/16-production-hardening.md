# 16 — Production Hardening: Timeouts, Retries, Concurrency, Shutdown, Serve API

> **Priority**: 🟡 P1 — Prevents cascade failures, retry storms, cost runaways, data loss
> **Status**: Not started
> **Depends on**: 02 (registry), 03 (adapters), 14 (tool loop wiring)
> **Blocks**: None

## Problem Statement

The provider adapter refactor creates multi-provider routing but doesn't harden it for production:

1. **No per-request timeouts** — a slow provider stalls the entire plan executor
2. **No retry jitter** — N agents retrying simultaneously create a thundering herd
3. **No concurrency limits** — 20 parallel tasks can overwhelm a provider with 10 RPM limit
4. **No context overflow handling** — tool results accumulate until context exceeds model limits
5. **No graceful shutdown** — Ctrl+C during plan execution loses in-flight work
6. **No serve API for providers** — roko-serve has 18 route modules but zero provider/model endpoints

## What Exists

| Component | Path | Status |
|---|---|---|
| ProviderHealthTracker | `crates/roko-learn/src/provider_health.rs` | 🔌 3-state breaker, in-memory |
| ProcessSupervisor | `crates/bardo-runtime/` | 🔌 Tracks agent processes |
| Executor snapshot | `.roko/state/executor.json` | 🔌 Checkpoint on save |
| `--resume` flag | `crates/roko-cli/src/orchestrate.rs` | 🔌 Resume from checkpoint |
| roko-serve routes | `crates/roko-serve/src/routes/` | 🔌 18 modules, 0 provider routes |
| `/api/learn/cascade` | `crates/roko-serve/src/routes/learning.rs` | 🔌 Router state exposed |
| `/api/config` | `crates/roko-serve/src/routes/config.rs` | 🔌 Config read/write |
| AppState | `crates/roko-serve/src/state.rs` | 🔌 Shared server state |

---

## A. Adaptive Timeouts

### 2N.01 — Add per-provider timeout configuration

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Extend `ProviderConfig` with timeout settings:

```rust
pub struct ProviderConfig {
    // ... existing fields ...
    pub timeout_ms: Option<u64>,          // Hard per-request timeout
    pub ttft_timeout_ms: Option<u64>,     // Time-to-first-token timeout
    pub connect_timeout_ms: Option<u64>,  // TCP connection timeout
}
```

Defaults: `timeout_ms = 120_000`, `ttft_timeout_ms = 15_000`, `connect_timeout_ms = 5_000`.

**Context**: TTFT timeout detects stalls before the hard timeout. If a provider hasn't sent a single token in 15 seconds, something is wrong — fail fast and try fallback rather than waiting 2 minutes.

**Acceptance**: All 3 timeouts are configurable per provider.
**Verification**: `cargo test -p roko-core -- provider_timeouts`

---

### 2N.02 — Implement adaptive timeout based on observed latency

**File**: `crates/roko-learn/src/latency.rs`
**What**: Auto-adjust timeouts based on observed p95 latency:

```rust
impl LatencyStats {
    /// Recommended timeout = 2x the observed p95 latency, clamped to [5s, 300s].
    pub fn adaptive_timeout_ms(&self) -> u64 {
        if self.observations < 10 { return 120_000; }  // Not enough data
        let p95 = self.p95_ms();
        let timeout = (p95 * 2.0) as u64;
        timeout.clamp(5_000, 300_000)
    }
}
```

**Context**: From production monitoring research: "P99 reveals worst-case performance for the slowest 1% of requests — this is the metric to base adaptive timeouts on." Using p95 * 2 provides headroom while still detecting degradation.

**Acceptance**: Provider with p95=3s gets 6s timeout. Provider with p95=30s gets 60s timeout.
**Verification**: `cargo test -p roko-learn -- adaptive_timeout`

---

## B. Retry with Jitter

### 2N.03 — Implement full-jitter exponential backoff

**File**: `crates/roko-agent/src/retry.rs` (new)
**What**: AWS-style full jitter to prevent thundering herd:

```rust
use rand::Rng;

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub retryable_errors: Vec<ErrorClass>,
}

impl RetryPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let exp_delay = self.base_delay_ms.saturating_mul(1 << attempt.min(10));
        let capped = exp_delay.min(self.max_delay_ms);
        // Full jitter: uniform random in [0, capped]
        rand::thread_rng().gen_range(0..=capped)
    }

    pub fn should_retry(&self, error: &ProviderError, attempt: u32) -> bool {
        if attempt >= self.max_attempts { return false; }
        match error {
            ProviderError::RateLimit { retry_after_ms } => true,  // Always retry rate limits
            ProviderError::AuthFailure => false,      // Never retry auth
            ProviderError::ContentPolicy => false,    // Never retry content policy
            ProviderError::Timeout => true,
            ProviderError::ServerError(_) => true,
            ProviderError::ContextOverflow => false,  // Need different model, not retry
            _ => attempt < 2,  // Unknown errors: try once more
        }
    }

    /// Respect provider's Retry-After header when available.
    pub fn delay_with_retry_after(&self, attempt: u32, retry_after_ms: Option<u64>) -> u64 {
        retry_after_ms.unwrap_or_else(|| self.delay_for_attempt(attempt))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1_000,
            max_delay_ms: 60_000,
            retryable_errors: vec![ErrorClass::RateLimit, ErrorClass::Timeout, ErrorClass::ServerError],
        }
    }
}
```

**Context**: From thundering herd research: "If 1,000 clients all do the same exponential backoff starting at the same time, they all retry at exactly the same moments." Full jitter spreads retries uniformly across the entire backoff window.

**Acceptance**: 100 concurrent retries produce uniformly distributed delays (not clustered).
**Verification**: `cargo test -p roko-agent -- full_jitter_distribution`

---

### 2N.04 — Wire retry policy into ToolLoop LlmBackend calls

**File**: `crates/roko-agent/src/tool_loop/backends/openai_compat.rs`
**What**: Wrap each `send_turn()` call with retry logic:

```rust
async fn send_turn_with_retry(
    &self,
    messages: &[Value],
    tools: &RenderedTools,
    retry: &RetryPolicy,
) -> Result<BackendResponse, LlmError> {
    for attempt in 0..retry.max_attempts {
        match self.send_turn(messages, tools).await {
            Ok(response) => return Ok(response),
            Err(LlmError::Provider(ref e)) if retry.should_retry(e, attempt) => {
                let delay = retry.delay_with_retry_after(attempt, e.retry_after_ms());
                tokio::time::sleep(Duration::from_millis(delay)).await;
                continue;
            },
            Err(e) => return Err(e),
        }
    }
    Err(LlmError::RetriesExhausted)
}
```

**Acceptance**: Rate limit (429) retries with jitter. Auth error (401) fails immediately.
**Verification**: `cargo test -p roko-agent -- retry_with_jitter`

---

## C. Concurrency Limits

### 2N.05 — Add semaphore-based per-provider concurrency control

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Limit concurrent in-flight requests per provider:

```rust
use tokio::sync::Semaphore;

pub struct ProviderSemaphores {
    semaphores: HashMap<String, Arc<Semaphore>>,
    default_permits: usize,
}

impl ProviderSemaphores {
    pub fn new(configs: &HashMap<String, ProviderConfig>) -> Self {
        let mut semaphores = HashMap::new();
        for (id, config) in configs {
            let permits = config.max_concurrent.unwrap_or(10) as usize;
            semaphores.insert(id.clone(), Arc::new(Semaphore::new(permits)));
        }
        Self { semaphores, default_permits: 10 }
    }

    pub async fn acquire(&self, provider_id: &str) -> SemaphorePermit {
        let sem = self.semaphores.get(provider_id)
            .cloned()
            .unwrap_or_else(|| Arc::new(Semaphore::new(self.default_permits)));
        sem.acquire_owned().await.expect("semaphore closed")
    }
}
```

**Context**: Without this, 20 parallel tasks can fire 20 simultaneous requests to a provider with a 10 RPM limit, causing 10 immediate 429 errors. The semaphore ensures only `max_concurrent` requests are in-flight at once.

```toml
[providers.zai]
max_concurrent = 5   # Z.AI supports ~5 concurrent requests on standard tier
```

**Acceptance**: With `max_concurrent = 3`, a 4th concurrent request blocks until one completes.
**Verification**: `cargo test -p roko-agent -- provider_semaphore`

---

### 2N.06 — Wire semaphores into LlmBackend send path

**File**: `crates/roko-agent/src/tool_loop/backends/openai_compat.rs`
**What**: Acquire semaphore permit before each HTTP request:

```rust
impl LlmBackend for OpenAiCompatBackend {
    async fn send_turn(&self, messages: &[Value], tools: &RenderedTools) -> Result<BackendResponse> {
        let _permit = self.semaphore.acquire().await;  // blocks if at limit
        // ... existing HTTP request logic ...
    }
}
```

**Acceptance**: Concurrent requests are bounded by semaphore permits.
**Verification**: `cargo test -p roko-agent -- semaphore_wired`

---

## D. Context Overflow Handling

### 2N.07 — Add context overflow detection to ToolLoop

**File**: `crates/roko-agent/src/tool_loop/mod.rs`
**What**: The existing ToolLoop has `context_token_limit` and pruning. Extend with model-aware overflow detection:

```rust
impl ToolLoop {
    fn check_context_overflow(&self, messages: &[Value], model: &ModelProfile) -> OverflowAction {
        let estimated_tokens = estimate_message_tokens(messages);
        let limit = model.context_window.unwrap_or(128_000) as usize;

        if estimated_tokens > limit {
            return OverflowAction::CompactRequired;
        }
        if estimated_tokens > (limit * 80 / 100) {
            return OverflowAction::CompactRecommended;
        }
        OverflowAction::Ok
    }
}

enum OverflowAction {
    Ok,
    CompactRecommended,  // > 80% — trigger compaction proactively
    CompactRequired,     // > 100% — must compact before next request
}
```

**Context**: From Semantic Kernel research: trigger compaction at 80% utilization, not 100%. At 100% the request fails; at 80% there's time to compact gracefully.

**Acceptance**: Messages exceeding 80% of context trigger compaction. 100% triggers required compaction.
**Verification**: `cargo test -p roko-agent -- context_overflow_detection`

---

### 2N.08 — Implement tool result compaction (gentle strategy)

**File**: `crates/roko-agent/src/tool_loop/compaction.rs` (new)
**What**: Collapse verbose tool outputs into summaries:

```rust
pub fn compact_tool_results(messages: &mut Vec<Value>) {
    // For each tool result message older than the 2 most recent tool-call groups:
    //   If content > 500 chars: truncate to first 200 chars + "... [truncated, N chars]"
    //   Preserve the tool_call_id (required for API validity)
    for msg in messages.iter_mut() {
        if msg["role"] == "tool" && !is_recent_tool_group(msg) {
            if let Some(content) = msg["content"].as_str() {
                if content.len() > 500 {
                    msg["content"] = Value::String(format!(
                        "{}... [truncated, {} chars total]",
                        &content[..200], content.len()
                    ));
                }
            }
        }
    }
}
```

**Context**: From Semantic Kernel's layered compaction: tool result compaction is the gentlest strategy. It removes verbose file contents and command outputs from old turns while preserving the structure (tool_call_id matching).

**Critical**: Tool call messages and their results must always be removed together. Removing a tool_call without its result (or vice versa) causes API errors.

**Acceptance**: Old tool results > 500 chars are truncated. Recent results are preserved. tool_call_id integrity maintained.
**Verification**: `cargo test -p roko-agent -- tool_result_compaction`

---

## E. Graceful Shutdown

### 2N.09 — Add signal-based graceful shutdown to orchestrator

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Handle SIGTERM/SIGINT with drain-and-checkpoint:

```rust
use tokio::signal;

pub async fn run_with_shutdown(executor: PlanExecutor, snapshot_path: &Path) -> Result<()> {
    let shutdown = signal::ctrl_c();

    tokio::select! {
        result = executor.run() => result,
        _ = shutdown => {
            tracing::warn!("shutdown signal received, draining in-flight tasks...");

            // Phase 1: stop accepting new tasks
            executor.stop_accepting();

            // Phase 2: drain with timeout (30s grace period)
            let drain_result = tokio::time::timeout(
                Duration::from_secs(30),
                executor.drain_in_flight(),
            ).await;

            if drain_result.is_err() {
                tracing::warn!("drain timeout, killing remaining agents");
                executor.kill_all_agents().await;
            }

            // Phase 3: checkpoint
            executor.save_snapshot(snapshot_path)?;
            tracing::info!("checkpoint saved to {}", snapshot_path.display());

            // Phase 4: flush logs
            executor.flush_logs().await;

            Ok(())
        }
    }
}
```

**Context**: The existing `--resume` flag can reload the checkpoint. This task ensures the checkpoint is always written on shutdown, and that in-flight agent processes are cleaned up.

**Acceptance**: Ctrl+C during plan execution saves checkpoint. `--resume` continues from saved state.
**Verification**:
```bash
# Start a plan, Ctrl+C after 5 seconds
timeout 5 cargo run -p roko-cli -- plan run plans/ || true
# Verify checkpoint exists
ls -la .roko/state/executor.json
# Resume
cargo run -p roko-cli -- plan run plans/ --resume .roko/state/executor.json
```

---

### 2N.10 — Atomic checkpoint writes

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Write checkpoint to temp file then rename, preventing corruption from mid-write crashes:

```rust
fn save_snapshot_atomic(snapshot: &ExecutorSnapshot, path: &Path) -> Result<()> {
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;  // Atomic on POSIX
    Ok(())
}
```

**Acceptance**: A crash during write doesn't corrupt the checkpoint file.
**Verification**: `cargo test -p roko-cli -- atomic_checkpoint`

---

## F. Serve API for Providers and Models

### 2N.11 — Add `GET /api/providers` endpoint

**File**: `crates/roko-serve/src/routes/providers.rs` (new)
**What**: List all configured providers with health status:

```rust
pub async fn list_providers(State(state): State<AppState>) -> Json<ProvidersResponse> {
    let config = state.config.read();
    let health = state.provider_health.snapshot();

    let providers: Vec<ProviderInfo> = config.effective_providers().iter().map(|(id, pc)| {
        ProviderInfo {
            id: id.clone(),
            kind: pc.kind.label().to_string(),
            base_url: pc.base_url.clone(),
            has_api_key: pc.resolve_api_key().is_some(),
            health: health.get(id).cloned(),
            model_count: config.effective_models().values()
                .filter(|m| &m.provider == id).count(),
        }
    }).collect();

    Json(ProvidersResponse { providers })
}
```

**Acceptance**: `GET /api/providers` returns provider list with health status.
**Verification**: `curl http://localhost:9090/api/providers | jq`

---

### 2N.12 — Add `GET /api/models` endpoint

**File**: `crates/roko-serve/src/routes/providers.rs`
**What**: List all configured models with capabilities and cost:

```rust
pub async fn list_models(State(state): State<AppState>) -> Json<ModelsResponse> {
    let config = state.config.read();
    let models: Vec<ModelInfo> = config.effective_models().iter().map(|(key, profile)| {
        ModelInfo {
            key: key.clone(),
            slug: profile.slug.clone(),
            provider: profile.provider.clone(),
            context_window: profile.context_window,
            supports_tools: profile.supports_tools,
            supports_thinking: profile.supports_thinking,
            supports_vision: profile.supports_vision,
            cost_input_per_m: profile.cost_input_per_m,
            cost_output_per_m: profile.cost_output_per_m,
        }
    }).collect();

    Json(ModelsResponse { models })
}
```

**Acceptance**: `GET /api/models` returns all models with capabilities.
**Verification**: `curl http://localhost:9090/api/models | jq`

---

### 2N.13 — Add `GET /api/providers/{id}/health` endpoint

**File**: `crates/roko-serve/src/routes/providers.rs`
**What**: Detailed health for a specific provider:

```rust
pub async fn provider_health(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Json<ProviderHealthResponse> {
    let health = state.provider_health.get(&provider_id);
    let latency = state.latency_registry.get_all_for_provider(&provider_id);

    Json(ProviderHealthResponse {
        provider_id,
        state: health.state,
        consecutive_failures: health.consecutive_failures,
        lifetime_attempts: health.total_requests,
        lifetime_successes: health.total_successes,
        last_success_at: health.last_success_at,
        last_failure_at: health.last_failure_at,
        latency_p50_ms: latency.p50_ms(),
        latency_p95_ms: latency.p95_ms(),
        latency_p99_ms: latency.p99_ms(),
        error_rate: health.error_rate(),
    })
}
```

**Acceptance**: Provider health endpoint returns circuit breaker state + latency percentiles.
**Verification**: `curl http://localhost:9090/api/providers/zai/health | jq`

---

### 2N.14 — Add `POST /api/providers/{id}/test` endpoint

**File**: `crates/roko-serve/src/routes/providers.rs`
**What**: Send a test request to verify provider connectivity:

```rust
pub async fn test_provider(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Json<ProviderTestResponse> {
    // Build a minimal "Say hello" request
    // Send via the provider adapter
    // Return success/failure with latency and token count
}
```

**Acceptance**: Test endpoint sends a real request and returns the result.
**Verification**: `curl -X POST http://localhost:9090/api/providers/zai/test | jq`

---

### 2N.15 — Add `GET /api/routing/explain` endpoint

**File**: `crates/roko-serve/src/routes/providers.rs`
**What**: Explain a routing decision via HTTP (same data as `roko model route --explain`):

```rust
pub async fn explain_routing(
    State(state): State<AppState>,
    Query(params): Query<RoutingExplainParams>,
) -> Json<RoutingExplanation> {
    // params: model, role, complexity
    // Return: candidates with scores, health status, cache affinity, Pareto status
}
```

**Acceptance**: Routing explanation matches CLI output.
**Verification**: `curl "http://localhost:9090/api/routing/explain?model=glm-5-1&role=implementer" | jq`

---

### 2N.16 — Wire provider routes into roko-serve router

**File**: `crates/roko-serve/src/routes/mod.rs`
**What**: Add the new routes to the Axum router:

```rust
.nest("/api/providers", providers::router())
.nest("/api/models", providers::models_router())
.nest("/api/routing", providers::routing_router())
```

Add `ProviderHealthTracker` and `LatencyRegistry` to `AppState`.

**Acceptance**: All new endpoints are accessible.
**Verification**: `cargo run -p roko-cli -- serve & curl http://localhost:9090/api/providers`

---

## G. Request Deduplication

### 2N.17 — Add content-addressed response cache for identical prompts

**File**: `crates/roko-agent/src/cache.rs` (new)
**What**: When multiple agents send identical prompts within a time window, share the response:

```rust
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct ResponseCache {
    entries: Mutex<HashMap<u64, CacheEntry>>,  // prompt_hash → entry
    ttl_ms: u64,
}

struct CacheEntry {
    response: BackendResponse,
    created_at: Instant,
}

impl ResponseCache {
    pub async fn get_or_compute<F, Fut>(
        &self,
        prompt_hash: u64,
        compute: F,
    ) -> Result<BackendResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<BackendResponse>>,
    {
        let mut cache = self.entries.lock().await;
        if let Some(entry) = cache.get(&prompt_hash) {
            if entry.created_at.elapsed() < Duration::from_millis(self.ttl_ms) {
                return Ok(entry.response.clone());
            }
        }
        drop(cache);

        let response = compute().await?;

        let mut cache = self.entries.lock().await;
        cache.insert(prompt_hash, CacheEntry {
            response: response.clone(),
            created_at: Instant::now(),
        });
        Ok(response)
    }
}
```

**Context**: When 20 agents with the same role ask "list the files in src/", the first call costs tokens; the rest get cached. TTL is short (30s default) to avoid stale results. Only applies to identical full prompts (content-addressed hash), not semantic similarity.

**Acceptance**: Second identical prompt within TTL returns cached response without HTTP call.
**Verification**: `cargo test -p roko-agent -- response_cache`

---

## H. Hedged Requests (Optional — Latency-Critical)

### 2N.18 — Add hedged request support for latency-sensitive tasks

**File**: `crates/roko-agent/src/tool_loop/backends/hedged.rs` (new)
**What**: Send to primary provider; if no first token within p95 TTFT, fire same request to backup:

```rust
pub struct HedgedBackend {
    primary: Arc<dyn LlmBackend>,
    backup: Arc<dyn LlmBackend>,
    hedge_after_ms: u64,  // p95 TTFT of primary
}

#[async_trait]
impl LlmBackend for HedgedBackend {
    async fn send_turn(&self, messages: &[Value], tools: &RenderedTools) -> Result<BackendResponse> {
        let primary_fut = self.primary.send_turn(messages, tools);

        // Wait for hedge threshold
        tokio::select! {
            result = primary_fut => result,
            _ = tokio::time::sleep(Duration::from_millis(self.hedge_after_ms)) => {
                // Primary is slow — fire backup
                let backup_fut = self.backup.send_turn(messages, tools);
                // Take whichever finishes first
                tokio::select! {
                    result = primary_fut => result,
                    result = backup_fut => result,
                }
            }
        }
    }
}
```

**Context**: From Dean & Barroso "The Tail at Scale" (2013). Only fires ~5% additional requests (those exceeding p95) while substantially reducing tail latency. Best used across deployments of the same model (e.g., GLM-5.1 on Z.AI vs GLM-5.1 on OpenRouter), not across different models.

**Acceptance**: Primary response within threshold → no hedge. Primary slow → backup fires, first response wins.
**Verification**: `cargo test -p roko-agent -- hedged_backend`

---

## Summary

| Section | Tasks | IDs | Priority |
|---|---|---|---|
| **A. Adaptive Timeouts** | 2 | 2N.01–2N.02 | 🔴 P0 |
| **B. Retry with Jitter** | 2 | 2N.03–2N.04 | 🔴 P0 |
| **C. Concurrency Limits** | 2 | 2N.05–2N.06 | 🔴 P0 |
| **D. Context Overflow** | 2 | 2N.07–2N.08 | 🟡 P1 |
| **E. Graceful Shutdown** | 2 | 2N.09–2N.10 | 🟡 P1 |
| **F. Serve API** | 6 | 2N.11–2N.16 | 🟡 P1 |
| **G. Request Dedup** | 1 | 2N.17 | 🟢 P2 |
| **H. Hedged Requests** | 1 | 2N.18 | 🟢 P2 |
| **Total** | **18** | **2N.01–2N.18** | |

## Execution Order

```
2N.01–2N.02 (timeouts)     ← FIRST: prevents cascade failures
2N.03–2N.04 (retry+jitter) ← SECOND: prevents retry storms
2N.05–2N.06 (semaphores)   ← THIRD: prevents provider overload
2N.09–2N.10 (shutdown)     ← FOURTH: prevents data loss
2N.07–2N.08 (overflow)     ← FIFTH: prevents context errors in long sessions
2N.11–2N.16 (serve API)    ← SIXTH: operational visibility
2N.17 (dedup cache)        ← SEVENTH: optimization
2N.18 (hedging)            ← EIGHTH: latency optimization (optional)
```

## Key Corrections to Earlier Docs

> **ProviderHealthTracker already exists** at `crates/roko-learn/src/provider_health.rs` with a
> 3-state circuit breaker (Healthy → Unhealthy → Probing). Tasks 2G.01–2G.03 in doc 08 and
> 2K.01–2K.03 in doc 13 proposed building new ones. Both should EXTEND the existing tracker
> with error classification, cooldown durations, and persistence — not rebuild.
>
> **roko-serve learning routes already exist** at `routes/learning.rs` with endpoints for
> efficiency, cascade router, experiments, and adaptive thresholds. The new provider routes
> (2N.11–2N.16) complement these, not replace them.
