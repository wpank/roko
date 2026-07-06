# 08 — Learning Loops & Cybernetic Feedback

> **Priority**: 🟡 P1 — Self-optimizing model routing, anomaly detection, provider health
> **Status**: Not started
> **Depends on**: None (can start in parallel with 02–07)
> **Blocks**: 10 (model experiments)

## Problem Statement

The CascadeRouter learns which model is best per task type, but it's blind to:
1. **Provider health** — rate limits, outages, latency spikes
2. **Latency variance** — p50 vs p95 per provider, no SLA tracking
3. **Cache affinity** — switching models between tasks kills prefix caching
4. **Dominated models** — never prunes models that are both worse AND more expensive
5. **Runaway loops** — no detection of spinning agents or cost spikes
6. **Error classification** — all failures treated equally (rate limit vs auth vs timeout)

## What Exists

| Component | Path | Status |
|---|---|---|
| CascadeRouter (3-stage) | `crates/roko-learn/src/cascade_router.rs` | 🔌 Static→Confidence→UCB |
| LinUCB (17-dim context) | `crates/roko-learn/src/model_router.rs` | 🔌 Full bandit |
| compute_routing_reward | `crates/roko-learn/src/model_router.rs` L180 | 🔌 0.5*pass+0.3*cost+0.2*dur |
| RoutingContext | `crates/roko-learn/src/model_router.rs` L72 | 🔌 6 fields, 17-dim vector |
| AgentEfficiencyEvent | `crates/roko-learn/src/efficiency.rs` L67 | 🔌 20+ fields per turn |
| CostsDb | `crates/roko-learn/src/costs_db.rs` | 🔌 In-memory + JSONL |
| CascadeModel.fallback | `crates/roko-learn/src/cascade_router.rs` L65 | 🔌 Single fallback |
| Feedback loop | `crates/roko-serve/src/feedback.rs` | 🔌 GitHub/Slack polling |
| Adaptive gate thresholds | `crates/roko-gate/src/adaptive_threshold.rs` | 🔌 EMA per rung |

---

## Checklist

### ~~2G.01–2G.03 — CORRECTION: ProviderHealthTracker Already Exists~~

> **CRITICAL DISCOVERY**: `crates/roko-learn/src/provider_health.rs` already implements
> `ProviderHealthTracker` with a 3-state circuit breaker (Healthy → Unhealthy → Probing).
> It has: `record_success()`, `record_failure()`, `is_healthy()`, `filter_arms()`, `snapshot()`.
> It tracks consecutive failures, last success/failure times, lifetime attempts/successes.
>
> Tasks 2G.01–2G.03 should be revised to **extend** the existing tracker, not build a new one.
> Specifically, what's missing from the existing tracker:
> - Error classification by type (rate limit vs auth vs timeout) — currently treats all failures equally
> - Error-type-specific cooldown durations
> - Persistence to disk (currently in-memory only?)
> - HTTP API exposure (exists in code but not wired to serve routes)
>
> See also doc 16 for production hardening extensions (adaptive timeouts, retry jitter,
> semaphore-based concurrency limits).

### 2G.01 — ~~Define ProviderHealth struct~~ → Extend existing ProviderHealthTracker

**File**: `crates/roko-learn/src/provider_health.rs` (EXISTING — extend, don't recreate)
**What**: ~~Track per-provider health state with circuit breaker:~~ Add error classification to the existing tracker:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub total_failures: u64,
    pub last_failure_at: Option<i64>,     // unix ms
    pub cooldown_until: Option<i64>,      // unix ms
    pub failure_window: Vec<FailureRecord>,  // last 20 failures
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // All requests blocked, in cooldown
    HalfOpen,  // Allowing 1 probe request
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub timestamp_ms: i64,
    pub error_class: ErrorClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    RateLimit,
    AuthFailure,
    Timeout,
    ServerError,
    ContentPolicy,
    ContextOverflow,
    Unknown,
}
```

**Acceptance**: All types compile with Serialize/Deserialize.
**Verification**: `cargo test -p roko-learn -- provider_health_types`

---

### 2G.02 — Implement circuit breaker state machine

**File**: `crates/roko-learn/src/provider_health.rs`
**What**: State transition logic:

```rust
impl ProviderHealth {
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        if self.state == CircuitState::HalfOpen {
            self.state = CircuitState::Closed;  // Probe succeeded
        }
    }

    pub fn record_failure(&mut self, error: ErrorClass, now_ms: i64) {
        self.consecutive_failures += 1;
        self.total_failures += 1;
        self.last_failure_at = Some(now_ms);
        self.failure_window.push(FailureRecord { timestamp_ms: now_ms, error_class: error });
        if self.failure_window.len() > 20 { self.failure_window.remove(0); }

        // Trip to Open after 3 consecutive failures
        if self.consecutive_failures >= 3 {
            self.state = CircuitState::Open;
            self.cooldown_until = Some(now_ms + self.cooldown_ms(error));
        }
    }

    pub fn is_available(&self, now_ms: i64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(until) = self.cooldown_until {
                    if now_ms >= until {
                        // Transition to HalfOpen (allow probe)
                        return true; // caller should set state to HalfOpen
                    }
                }
                false
            },
            CircuitState::HalfOpen => true, // allow probe
        }
    }

    fn cooldown_ms(&self, error: ErrorClass) -> i64 {
        match error {
            ErrorClass::RateLimit => 5_000,     // 5 seconds
            ErrorClass::Timeout => 10_000,      // 10 seconds
            ErrorClass::ServerError => 30_000,  // 30 seconds
            ErrorClass::AuthFailure => 300_000, // 5 minutes (likely persistent)
            _ => 5_000,
        }
    }
}
```

**Context**: Based on LiteLLM's cooldown logic. Error-type-specific cooldowns prevent wasting requests on degraded endpoints.

**Acceptance**: 3 consecutive failures → Open state. After cooldown → HalfOpen. Success in HalfOpen → Closed.
**Verification**: `cargo test -p roko-learn -- circuit_breaker`

---

### 2G.03 — Create ProviderHealthRegistry

**File**: `crates/roko-learn/src/provider_health.rs`
**What**: Centralized registry of per-provider health:

```rust
pub struct ProviderHealthRegistry {
    providers: Mutex<HashMap<String, ProviderHealth>>,
}

impl ProviderHealthRegistry {
    pub fn new() -> Self;
    pub fn record_success(&self, provider_id: &str);
    pub fn record_failure(&self, provider_id: &str, error: ErrorClass);
    pub fn is_available(&self, provider_id: &str) -> bool;
    pub fn available_providers(&self, candidates: &[String]) -> Vec<String>;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn load_or_new(path: &Path) -> Self;
}
```

**Persistence**: `.roko/learn/provider-health.json`

**Acceptance**: Registry tracks health per provider, persists to disk.
**Verification**: `cargo test -p roko-learn -- provider_health_registry`

---

> **ADDITIONAL CORRECTION (doc 17 final audit)**: `runtime_feedback.rs` is THE integration hub.
> It already calls `provider_health.record()` at L~521. Tasks 2G.04 should wire health into
> the routing PATH (before `cascade_router.route()`), not just into the recording path.
> Also: 10 conductor watchers already exist in `roko-conductor/src/watchers/` — tasks 2G.12–16
> (AnomalyDetector) should WIRE existing watchers, not rebuild anomaly detection from scratch.
> See doc 17 task 2O.02 for wiring conductor stuck detection into negative routing signal.

### 2G.04 — Wire ProviderHealth into CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Add health-aware model filtering. Before scoring models, filter out those whose provider is in Open state:

```rust
impl CascadeRouter {
    pub fn route_with_health(
        &self,
        ctx: &RoutingContext,
        health: &ProviderHealthRegistry,
        model_providers: &HashMap<String, String>,  // model_slug → provider_id
    ) -> CascadeModel {
        let available: Vec<_> = self.model_slugs.iter()
            .filter(|slug| {
                model_providers.get(*slug)
                    .map(|pid| health.is_available(pid))
                    .unwrap_or(true)  // unknown provider = available
            })
            .collect();
        // Route among available models only
        // ...
    }
}
```

**Acceptance**: Models with unhealthy providers are excluded from selection.
**Verification**: `cargo test -p roko-learn -- cascade_health_aware`

---

### 2G.05 — Define LatencyStats struct

**File**: `crates/roko-learn/src/latency.rs` (new)
**What**: Track per-model, per-provider latency percentiles:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub model_slug: String,
    pub provider_id: String,
    pub ttft_ema_ms: f64,              // time to first token EMA
    pub total_latency_ema_ms: f64,     // total response time EMA
    pub tokens_per_second_ema: f64,    // output throughput
    pub observations: u64,
    pub recent_latencies: Vec<f64>,    // last 100 for percentile calc
}

impl LatencyStats {
    pub fn record(&mut self, ttft_ms: f64, total_ms: f64, output_tokens: u64) {
        let alpha = 0.1;
        self.ttft_ema_ms = alpha * ttft_ms + (1.0 - alpha) * self.ttft_ema_ms;
        self.total_latency_ema_ms = alpha * total_ms + (1.0 - alpha) * self.total_latency_ema_ms;
        if total_ms > 0.0 && output_tokens > 0 {
            let tps = output_tokens as f64 / (total_ms / 1000.0);
            self.tokens_per_second_ema = alpha * tps + (1.0 - alpha) * self.tokens_per_second_ema;
        }
        self.observations += 1;
        self.recent_latencies.push(total_ms);
        if self.recent_latencies.len() > 100 { self.recent_latencies.remove(0); }
    }

    pub fn p50_ms(&self) -> f64 { self.percentile(0.50) }
    pub fn p95_ms(&self) -> f64 { self.percentile(0.95) }
    pub fn p99_ms(&self) -> f64 { self.percentile(0.99) }
}
```

**Acceptance**: `LatencyStats` tracks EMA and computes p50/p95/p99.
**Verification**: `cargo test -p roko-learn -- latency_stats`

---

### 2G.06 — Create LatencyRegistry

**File**: `crates/roko-learn/src/latency.rs`
**What**: Centralized latency tracking:

```rust
pub struct LatencyRegistry {
    stats: Mutex<HashMap<(String, String), LatencyStats>>,  // (model, provider) → stats
}

impl LatencyRegistry {
    pub fn record(&self, model: &str, provider: &str, ttft_ms: f64, total_ms: f64, tokens: u64);
    pub fn get(&self, model: &str, provider: &str) -> Option<LatencyStats>;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn load_or_new(path: &Path) -> Self;
}
```

**Persistence**: `.roko/learn/latency-stats.json`

**Acceptance**: Latency is tracked per (model, provider) pair.
**Verification**: `cargo test -p roko-learn -- latency_registry`

---

### 2G.07 — Wire latency into compute_routing_reward

**File**: `crates/roko-learn/src/model_router.rs`
**What**: Replace the static `normalized_duration` with actual observed latency:

```rust
pub fn compute_routing_reward_v2(
    pass_rate: f64,
    normalized_cost: f64,
    observed_latency_ms: f64,
    latency_sla_ms: f64,        // from tier (10s/30s/120s)
) -> f64 {
    let normalized_duration = (observed_latency_ms / latency_sla_ms).min(1.0);
    pass_rate * 0.5 + (1.0 - normalized_cost) * 0.3 + (1.0 - normalized_duration) * 0.2
}
```

**Acceptance**: Faster models get higher rewards.
**Verification**: `cargo test -p roko-learn -- routing_reward_v2`

---

### 2G.08 — Add cache affinity bias to RoutingContext

**File**: `crates/roko-learn/src/model_router.rs`
**What**: Extend `RoutingContext` with cache affinity:

```rust
pub struct RoutingContext {
    // ... existing 6 fields ...
    pub previous_model: Option<String>,     // model used for previous task in same plan
    pub plan_context_tokens: Option<u64>,   // estimated shared prefix size
}
```

Update `to_features()` to encode cache affinity as feature dim 17 (extending to 18-dim):
- Feature[17] = 1.0 if `previous_model` matches the candidate, 0.0 otherwise

**Context**: When consecutive tasks in the same plan use the same model, the system prompt + context is cached, saving 50-90% on input tokens. The bandit should learn this correlation.

**Acceptance**: `RoutingContext` with `previous_model` produces 18-dim feature vector.
**Verification**: `cargo test -p roko-learn -- cache_affinity_feature`

---

### 2G.09 — Add cache affinity score bonus (heuristic)

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Before the LinUCB kicks in (< 200 observations), add a simple bonus:

```rust
const CACHE_AFFINITY_BONUS: f64 = 0.15;  // 15% score bonus for same model

fn apply_cache_affinity(scores: &mut [(String, f64)], previous_model: Option<&str>) {
    if let Some(prev) = previous_model {
        for (slug, score) in scores.iter_mut() {
            if slug == prev {
                *score += CACHE_AFFINITY_BONUS;
            }
        }
    }
}
```

**Context**: This is a heuristic for the cold-start period. Once LinUCB has enough observations, it learns the cache affinity correlation from the feature vector (2G.08).

**Acceptance**: Same-model selection gets 15% bonus.
**Verification**: `cargo test -p roko-learn -- cache_affinity_bonus`

---

### 2G.10 — Implement Pareto frontier computation

**File**: `crates/roko-learn/src/pareto.rs` (new)
**What**: Compute which models are on the cost-quality Pareto frontier:

```rust
/// A model is Pareto-optimal if no other model has both:
///   - higher pass_rate AND
///   - lower cost_per_successful_task
pub fn compute_pareto_frontier(
    stats: &HashMap<String, ModelObservation>,
) -> Vec<String> {
    let mut frontier = Vec::new();
    for (slug_a, obs_a) in stats {
        let dominated = stats.iter().any(|(slug_b, obs_b)| {
            slug_b != slug_a
                && obs_b.pass_rate >= obs_a.pass_rate
                && obs_b.cost_per_success <= obs_a.cost_per_success
                && (obs_b.pass_rate > obs_a.pass_rate || obs_b.cost_per_success < obs_a.cost_per_success)
        });
        if !dominated {
            frontier.push(slug_a.clone());
        }
    }
    frontier
}

pub struct ModelObservation {
    pub pass_rate: f64,
    pub cost_per_success: f64,          // total_cost / successful_tasks
    pub avg_latency_ms: f64,
    pub observations: u64,
}
```

**Acceptance**: With 3 models where A dominates B, only A and C are on the frontier.
**Verification**: `cargo test -p roko-learn -- pareto_frontier`

---

### 2G.11 — Apply Pareto pruning to LinUCB exploration

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Reduce exploration of dominated models:

```rust
fn pareto_adjusted_alpha(base_alpha: f64, slug: &str, frontier: &[String]) -> f64 {
    if frontier.contains(&slug.to_string()) {
        base_alpha  // Full exploration for Pareto-optimal
    } else {
        base_alpha * 0.1  // 90% reduction for dominated
    }
}
```

Recompute frontier every 50 observations.

**Acceptance**: Non-Pareto models get 90% less exploration.
**Verification**: `cargo test -p roko-learn -- pareto_pruning`

---

### 2G.12 — Define AnomalyDetector struct

**File**: `crates/roko-learn/src/anomaly.rs` (new)
**What**: Detect runaway agent loops and cost spikes:

```rust
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,   // last 20 prompt hashes
    cost_ewma: EwmaState,
    quality_history: VecDeque<f64>,      // last 50 quality scores
    session_cost_usd: f64,
    session_start_ms: i64,
}

pub struct EwmaState {
    pub mean: f64,
    pub variance: f64,
    alpha: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Anomaly {
    PromptLoop { repeated_count: usize },
    CostSpike { z_score: f64 },
    QualityDegradation { avg_drop: f64 },
    BudgetExhausted { used: f64, limit: f64 },
}

impl AnomalyDetector {
    pub fn check_prompt(&mut self, prompt_hash: u64) -> Option<Anomaly>;
    pub fn check_cost(&mut self, cost_usd: f64) -> Option<Anomaly>;
    pub fn check_quality(&mut self, score: f64) -> Option<Anomaly>;
    pub fn check_budget(&self, limit_usd: f64) -> Option<Anomaly>;
}
```

**Acceptance**: 5 identical prompt hashes → `Anomaly::PromptLoop`. Z-score > 3.0 → `Anomaly::CostSpike`.
**Verification**: `cargo test -p roko-learn -- anomaly_detector`

---

### 2G.13 — Implement prompt loop detection

**File**: `crates/roko-learn/src/anomaly.rs`
**What**: Hash the prompt, track in a sliding window of 20:

```rust
pub fn check_prompt(&mut self, prompt_hash: u64) -> Option<Anomaly> {
    self.prompt_hash_window.push_back(prompt_hash);
    if self.prompt_hash_window.len() > 20 { self.prompt_hash_window.pop_front(); }

    let count = self.prompt_hash_window.iter().filter(|&&h| h == prompt_hash).count();
    if count >= 5 {
        Some(Anomaly::PromptLoop { repeated_count: count })
    } else {
        None
    }
}
```

**Acceptance**: 5 identical hashes in 20 triggers loop detection.
**Verification**: `cargo test -p roko-learn -- prompt_loop_detection`

---

### 2G.14 — Implement cost spike detection via EWMA

**File**: `crates/roko-learn/src/anomaly.rs`
**What**: Track cost with EWMA and detect z-score > 3.0:

```rust
impl EwmaState {
    pub fn update(&mut self, value: f64) {
        let diff = value - self.mean;
        self.mean += self.alpha * diff;
        self.variance = (1.0 - self.alpha) * (self.variance + self.alpha * diff * diff);
    }

    pub fn z_score(&self, value: f64) -> f64 {
        let stddev = self.variance.sqrt();
        if stddev < 1e-10 { return 0.0; }
        (value - self.mean) / stddev
    }
}
```

**Acceptance**: Sudden 10x cost spike produces z-score > 3.0.
**Verification**: `cargo test -p roko-learn -- cost_spike_detection`

---

### 2G.15 — Implement quality degradation detection

**File**: `crates/roko-learn/src/anomaly.rs`
**What**: Track rolling quality scores and detect degradation:

```rust
pub fn check_quality(&mut self, score: f64) -> Option<Anomaly> {
    self.quality_history.push_back(score);
    if self.quality_history.len() > 50 { self.quality_history.pop_front(); }
    if self.quality_history.len() < 10 { return None; }

    let recent: Vec<_> = self.quality_history.iter().rev().take(5).collect();
    let earlier: Vec<_> = self.quality_history.iter().rev().skip(10).take(10).collect();

    let recent_avg: f64 = recent.iter().copied().sum::<f64>() / recent.len() as f64;
    let earlier_avg: f64 = earlier.iter().copied().sum::<f64>() / earlier.len() as f64;

    if recent_avg < earlier_avg - 0.15 && recent_avg < 0.5 {
        Some(Anomaly::QualityDegradation { avg_drop: earlier_avg - recent_avg })
    } else {
        None
    }
}
```

**Acceptance**: Quality dropping from 0.8 to 0.3 triggers degradation alert.
**Verification**: `cargo test -p roko-learn -- quality_degradation`

---

### 2G.16 — Wire AnomalyDetector into dispatch pipeline

**File**: `crates/roko-serve/src/dispatch.rs`
**What**: Before each agent turn, run anomaly checks. On detection:
- `PromptLoop` → abort session with error
- `CostSpike` → log warning, force model downgrade
- `QualityDegradation` → log warning, record negative router observation
- `BudgetExhausted` → abort session with budget error

**Acceptance**: A looping agent is halted after 5 identical prompts.
**Verification**: `cargo test -p roko-serve -- anomaly_dispatch`

---

### 2G.17 — Add error-type-specific retry policy

**File**: `crates/roko-agent/src/provider/mod.rs`
**What**: Define retry behavior per error class:

```rust
pub fn should_retry(error: &ProviderError) -> RetryAction {
    match error {
        ProviderError::RateLimit { retry_after_ms } => RetryAction::WaitAndRetry {
            delay_ms: retry_after_ms.unwrap_or(5000),
        },
        ProviderError::AuthFailure => RetryAction::Skip,  // Don't retry auth errors
        ProviderError::Timeout => RetryAction::TryFallback,
        ProviderError::ServerError(_) => RetryAction::TryFallback,
        ProviderError::ContentPolicy => RetryAction::Skip,
        ProviderError::ContextOverflow => RetryAction::TryWithSmallerContext,
        _ => RetryAction::TryFallback,
    }
}

pub enum RetryAction {
    WaitAndRetry { delay_ms: u64 },
    TryFallback,
    TryWithSmallerContext,
    Skip,
}
```

**Acceptance**: Rate limit → wait, auth → skip, timeout → fallback.
**Verification**: `cargo test -p roko-agent -- retry_policy`

---

### 2G.18 — Persist ProviderHealth and LatencyStats

**File**: `crates/roko-learn/src/provider_health.rs` and `latency.rs`
**What**: Both registries persist to `.roko/learn/`:
- `.roko/learn/provider-health.json`
- `.roko/learn/latency-stats.json`

Load on startup, save after updates (debounced to avoid excessive disk writes).

**Acceptance**: Data survives process restart.
**Verification**: `cargo test -p roko-learn -- health_persistence`

---

### 2G.19 — Add LLM-judge quality signal for non-gateable tasks

**File**: `crates/roko-learn/src/quality_judge.rs` (new)
**What**: For tasks without compilable output (research, documentation, architecture), use a cheap model as judge:

```rust
pub async fn judge_quality(
    agent: &dyn Agent,
    prompt: &str,
    response: &str,
    rubric: &str,
) -> f64 {
    let judge_prompt = format!(
        "Rate the quality of this response on a scale of 0.0 to 1.0.\n\
         Rubric: {rubric}\n\
         Prompt: {prompt}\n\
         Response: {response}\n\
         Score (0.0-1.0):"
    );
    // Send to cheap model (haiku-class), parse float from response
    // Return 0.0 on failure
}
```

**Context**: The CascadeRouter currently only gets binary feedback (gate pass/fail). For non-gateable tasks, this provides a 0-1 quality score that feeds into `compute_routing_reward`.

**Acceptance**: Judge returns a float in [0.0, 1.0] for a sample prompt/response pair.
**Verification**: `cargo test -p roko-learn -- quality_judge` (mock test)

---

### 2G.20 — Wire all learning loops into orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: After each agent turn, record to all learning systems:

```rust
// After agent.run() completes:
provider_health.record_success_or_failure(provider_id, &result);
latency_registry.record(model, provider, ttft_ms, total_ms, output_tokens);
anomaly_detector.check_prompt(prompt_hash);
anomaly_detector.check_cost(result.usage.cost_usd);
cascade_router.record_observation(ctx, model_slug, reward, success);
efficiency_logger.append(event).await;
episode_logger.append(episode).await;
```

**Acceptance**: A single agent turn triggers updates to health, latency, anomaly, router, efficiency, and episode systems.
**Verification**:
```bash
cargo run -p roko-cli -- run "echo test" 2>&1 | grep -E 'health|latency|anomaly|router'
```
