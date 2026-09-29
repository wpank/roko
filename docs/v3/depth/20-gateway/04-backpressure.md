# 20-gateway/04 -- Three-Level Backpressure

> Cancellation-safe RAII concurrency guards at provider, agent, and global levels.
> Bounded waiting rooms, HTTP error mapping, and telemetry.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md), section 12

**Source:** `crates/roko-gateway/src/backpressure.rs`

---

## 1. Design Rationale

Without backpressure, a burst of agent requests can overwhelm provider rate limits,
exhaust process memory with queued requests, or cause cascading timeouts. The gateway
enforces concurrency at three independent levels, each serving a different purpose:

| Level | What It Protects | Failure Mode Without It |
|-------|-----------------|------------------------|
| **Per-provider** (circuit) | Provider API rate limits | 429 storms, provider bans |
| **Per-agent** (request) | Individual agent runaway | One agent monopolizes all slots |
| **Global** (budget) | Process resources | Memory exhaustion, cascading timeouts |

All three levels are checked in order (global -> agent -> provider) before a provider
call proceeds. If any check fails, earlier reservations are released.

---

## 2. Configuration

```rust
pub struct BackpressureConfig {
    pub providers: HashMap<String, ProviderLimitConfig>,
    pub per_agent: u32,   // max in-flight per agent (default: 8)
    pub global: u32,      // max in-flight globally (default: 200)
}

pub struct ProviderLimitConfig {
    pub concurrency: u32,     // execution slots
    pub queue_capacity: u32,  // waiting room slots
}
```

### Default Provider Limits

| Provider | Concurrency | Queue Capacity | Total Outstanding |
|----------|-------------|----------------|-------------------|
| Anthropic | 50 | 100 | 150 |
| OpenAI | 50 | 100 | 150 |
| Gemini | 30 | 60 | 90 |
| Perplexity | 20 | 40 | 60 |
| Ollama | 4 | 8 | 12 |
| OpenRouter | 50 | 100 | 150 |
| Other | 20 | 40 | 60 |

Queue capacity defaults to 2x the concurrency limit.

---

## 3. Acquisition Protocol

The `BackpressureGuard::acquire(provider, agent_id)` method checks all three levels
in order:

```
acquire(provider, agent_id)
    |
    v
1. Global: try_reserve(global_in_flight, 200)
    |
    +--> fail --> GlobalOverload (503, Retry-After: 5)
    |
    v
2. Agent: try_reserve(agent_counter, 8)
    |
    +--> fail --> release(global), AgentQueueFull (429, Retry-After: 2)
    |
    v
3. Provider outstanding: try_reserve(outstanding, concurrency + queue)
    |
    +--> fail --> release(agent, global), ProviderFull (503, Retry-After: 5)
    |
    v
4. Provider semaphore: try_acquire_owned()
    |
    +--> acquired --> BackpressurePermit (immediate)
    +--> blocked  --> wait in queue (bounded by outstanding check)
                      |
                      +--> acquired --> activate pending, BackpressurePermit
                      +--> error   --> release all, ProviderFull (503)
```

### Atomic Reservation

The `try_reserve` function uses `fetch_update` with `AcqRel` ordering:

```rust
fn try_reserve(counter: &AtomicU32, limit: u32) -> bool {
    counter.fetch_update(Ordering::AcqRel, Ordering::Relaxed, |current| {
        (current < limit).then_some(current + 1)
    }).is_ok()
}
```

This is a CAS loop that atomically increments the counter only if it is below the
limit. No locks are held during reservation.

---

## 4. RAII Permit

```rust
pub struct BackpressurePermit {
    _provider_permit: OwnedSemaphorePermit,
    provider_outstanding: Arc<AtomicU32>,
    active: Arc<AtomicU32>,
    agent: Arc<AtomicU32>,
    global: Arc<AtomicU32>,
}

impl Drop for BackpressurePermit {
    fn drop(&mut self) {
        release(&self.active);
        release(&self.provider_outstanding);
        release(&self.agent);
        release(&self.global);
    }
}
```

Key properties:
- **Cancellation-safe:** If the task is cancelled (e.g., client disconnects), the
  `Drop` impl releases all four counters automatically.
- **No leaks:** The `OwnedSemaphorePermit` releases the provider semaphore slot on
  drop. The atomic counters are decremented via `saturating_sub`.
- **Order-independent release:** Each counter is decremented independently.

### Pending Reservation

When a request can enter the provider's waiting room but not execute immediately, a
`PendingReservation` guard tracks the intermediate state. If the waiting task is
cancelled (e.g., by `tokio::select!` or task abort), the `PendingReservation::Drop`
releases the outstanding, queued, agent, and global slots -- preventing capacity leaks
from abandoned waiters.

---

## 5. HTTP Error Mapping

```rust
impl BackpressureError {
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::AgentQueueFull { .. } => 429,  // Too Many Requests
            Self::ProviderFull { .. } | Self::GlobalOverload => 503,
        }
    }

    pub const fn retry_after_seconds(&self) -> u64 {
        match self {
            Self::AgentQueueFull { .. } => 2,
            Self::ProviderFull { .. } | Self::GlobalOverload => 5,
        }
    }
}
```

The HTTP adapter in `http.rs` translates these into proper `Retry-After` headers:

```json
HTTP 429 Too Many Requests
Retry-After: 2

{ "error": "agent_queue_full", "agent_id": "coder-1", "max_depth": 8 }
```

```json
HTTP 503 Service Unavailable
Retry-After: 5

{ "error": "gateway_overloaded", "queued": 200, "active": 184 }
```

---

## 6. Telemetry

`BackpressureStats` provides a non-blocking snapshot:

```rust
pub struct BackpressureStats {
    pub providers: HashMap<String, ProviderBackpressureStats>,
    pub global_in_flight: u32,
    pub global_rejected: u32,
}

pub struct ProviderBackpressureStats {
    pub queue_depth: u32,        // waiting for execution
    pub active_requests: u32,    // currently executing
    pub rejected_count: u32,     // rejected by this provider
    pub concurrency_limit: u32,
    pub queue_capacity: u32,
}
```

All counters are read with `Ordering::Relaxed` -- the stats endpoint is advisory and
does not need sequential consistency.

---

## 7. Testing

Three tests verify the backpressure contract:

1. **Default limits:** Anthropic=50, OpenAI=50, Ollama=4, queue capacity = 2x
2. **Agent rejection:** 9th request for one agent returns 429, release-then-reacquire
   works
3. **Global overload:** Exceeding global=2 returns 503, `global_rejected` increments
4. **Cancellation safety:** Aborting a waiting task releases the queued slot, and
   subsequent requests can proceed
