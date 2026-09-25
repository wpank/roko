# 08-learning/10 -- Provider Health and Circuit Breaker

> Three-state circuit breaker (Closed/Open/Half-Open), error classification,
> exponential backoff, anomaly detection, and routing integration.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/provider_health.rs`

**Cross-references:** [cascade-router](cascade-router.md),
[cost-normalization](cost-normalization.md)

---

## 1. Purpose

The provider health module tracks the operational status of each LLM provider
and implements a three-state circuit breaker that prevents routing requests to
degraded or failing providers. When a provider starts returning errors (rate
limits, timeouts, server errors), the circuit breaker opens, diverting traffic
to healthy alternatives. After a cooldown period, the circuit breaker
transitions to half-open, allowing a single probe request to test recovery
before fully restoring traffic.

This is cybernetic feedback loop 1 (Health->Routing) from the eight feedback
loops: provider health state directly influences routing decisions in the
cascade router.

---

## 2. Three-State Circuit Breaker

```
                 success
    +-----------------------------+
    |                             |
    v                             |
+--------+   failure threshold   +------------+
| CLOSED | -------------------->|   OPEN     |
|(normal)|                      |(no traffic)|
+--------+                      +------+-----+
    ^                                  |
    |                          cooldown expires
    |         success                  |
    |  +------------------+            |
    +--+    HALF-OPEN     |<-----------+
       |(single probe req)|
       +------------------+
              |
              | failure
              v
          OPEN (reset cooldown)
```

### 2.1 States

| State | Behavior | Transition |
|-------|----------|------------|
| **Closed** | Normal operation. Failures counted. | -> Open: failures exceed threshold |
| **Open** | No requests routed. Traffic diverted. | -> Half-Open: after cooldown |
| **Half-Open** | Single probe request allowed. | -> Closed: probe success / -> Open: probe failure |

---

## 3. Error Classification

```rust
pub enum ErrorClass {
    RateLimit,       // HTTP 429
    AuthFailure,     // HTTP 401/403
    Timeout,         // request/response timeout
    ServerError,     // HTTP 5xx
    ContentPolicy,   // filtered response
    ContextOverflow, // context window exceeded
    Unknown,
}
```

### 3.1 Error-Specific Cooldowns

| Error Class | Cooldown | Rationale |
|-------------|----------|-----------|
| `RateLimit` | 60s (escalating) | Provider recovers after rate window |
| `AuthFailure` | 300s (long) | Requires manual API key rotation |
| `Timeout` | 30s | Often transient network issues |
| `ServerError` | 120s | Provider-side, variable recovery |
| `ContentPolicy` | 0s (flag only) | Not a provider health issue |
| `ContextOverflow` | 0s (route to larger) | Not a provider issue |
| `Unknown` | 60s (conservative) | Unknown errors get safe treatment |

---

## 4. ProviderHealth

```rust
pub struct ProviderHealth {
    pub provider_id: String,
    pub state: CircuitState,
    pub recent_failures: VecDeque<FailureRecord>,
    pub failure_count: u64,
    pub success_count: u64,
    pub last_opened: Option<DateTime<Utc>>,
    pub cooldown_until: Option<DateTime<Utc>>,
}
```

### 4.1 Threshold Configuration

The circuit breaker opens when:

- **Failure count** exceeds the threshold within the observation window, OR
- **Failure rate** (failures / total requests) exceeds the rate threshold.

Default values:

- Failure count threshold: 5 failures
- Observation window: 60 seconds
- Failure rate threshold: 50%

---

## 5. ProviderHealthRegistry

```rust
pub struct ProviderHealthRegistry {
    providers: Mutex<HashMap<String, ProviderHealth>>,
}
```

| Method | What it does |
|--------|-------------|
| `record_success(provider)` | Increment success. If half-open, close. |
| `record_failure(provider, error_class)` | Record failure. Check threshold. |
| `is_available(provider)` | True if Closed or Half-Open. |
| `available_providers()` | All Closed or Half-Open providers. |

### 5.1 Integration with Cascade Router

```
CascadeRouter::select(context)
    |
    +-- For each candidate model:
    |     |
    |     +-- ProviderHealthRegistry::is_available(model.provider)?
    |     |     YES -> include in candidate set
    |     |     NO  -> exclude (circuit is Open)
    |     |
    |     +-- Score candidate using stage algorithm
    |
    +-- Select highest-scoring available candidate
```

If all providers for a desired tier are unavailable, the router escalates to
the next tier or returns the fallback model.

---

## 6. Exponential Backoff

When a circuit breaker reopens after a failed half-open probe, the cooldown
period increases exponentially:

```
cooldown(n) = base_cooldown * 2^(n-1)
```

| Cycle | Cooldown (RateLimit) | Cooldown (ServerError) |
|-------|---------------------|----------------------|
| 1 | 60s | 120s |
| 2 | 120s | 240s |
| 3 | 240s | 480s |
| 4 | 480s (max) | 480s (max) |

Maximum cooldown capped at 480 seconds (8 minutes).

---

## 7. Anomaly Detection

The `AnomalyDetector` provides additional provider-health-adjacent checks:

```rust
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,    // last 20 prompt hashes
    cost_ewma: EwmaState,                 // EWMA cost baseline
    quality_history: VecDeque<f64>,        // rolling quality scores
    session_cost_usd: f64,
    session_start_ms: i64,
}
```

### 7.1 Anomaly Types

| Anomaly | Detection | Threshold |
|---------|-----------|-----------|
| Prompt loop | Same hash 5+ times in last 20 | `PROMPT_LOOP_THRESHOLD = 5` |
| Cost spike | Z-score > 3.0 against EWMA | `COST_SPIKE_Z_THRESHOLD = 3.0` |
| Quality degradation | Recent 5 avg < 0.5 AND drop > 0.15 | Composite check |

### 7.2 EWMA Cost Baseline

```
ewma_new = alpha * observation + (1 - alpha) * ewma_old    (alpha = 0.2)
z_score = (observation - ewma) / ewma_stddev
```

The observation is compared against the EWMA *before* the state is updated,
keeping sudden spikes visible.

---

## 8. ProviderHealthTracker

Extends the registry with time-series health metrics:

```
Provider: anthropic
+-- State: Closed
+-- Success rate (1h): 98.2%
+-- Failure rate (1h): 1.8%
+-- Recent errors: [Timeout x 1, RateLimit x 2]
+-- Avg latency (1h): 1,240ms
+-- Circuit opens (24h): 2
```

This data feeds into the TUI dashboard and conductor subsystem.
