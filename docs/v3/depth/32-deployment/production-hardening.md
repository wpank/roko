# 32-deployment/14 -- Production Hardening

> Adaptive timeouts, exponential backoff, graceful shutdown, zero-downtime
> upgrades, observability, health checks, and multi-tenant safety across
> all five deployment shapes.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-agent/src/provider/`, `crates/roko-serve/src/`,
`crates/roko-runtime/src/`

---

## 1. Shape-Aware Hardening

Production hardening applies the same principles across all five
deployment shapes (laptop-local, single-server, container, clustered,
edge), with profile-aware defaults:

| Principle | laptop-local | container | clustered |
|-----------|-------------|-----------|-----------|
| Timeouts | Conservative | Tuned | Adaptive |
| Retries | 2 attempts | 3 attempts | 3 + failover |
| Concurrency | 2-4 agents | Per-node cap | Horizontal |
| Shutdown | Immediate drain | Grace period | Rolling |
| Observability | stderr logs | Structured + metrics | Full OTel |

---

## 2. Adaptive Timeouts

Static timeouts are brittle. Roko tracks per-provider latency and sets
timeouts from recent observations.

### Algorithm

Use p95 latency multiplied by 2, clamped to a sane range:

```rust
pub fn timeout_for(samples: &[Duration]) -> Duration {
    if samples.is_empty() {
        return Duration::from_secs(30);
    }
    let mut sorted = samples.to_vec();
    sorted.sort();
    let p95_idx = (sorted.len() as f64 * 0.95) as usize;
    let p95 = sorted[p95_idx.min(sorted.len() - 1)];
    Duration::from_secs_f64(
        (p95.as_secs_f64() * 2.0).clamp(5.0, 300.0)
    )
}
```

### Per-Provider Tracking

Separate timeout histories per provider. Container and clustered
deployments benefit most (many concurrent requests), but the same logic
applies in laptop-local and single-server profiles. The provider health
registry persists latency samples across restarts.

---

## 3. Exponential Backoff with Full Jitter

```
sleep = random_between(0, min(cap, base * 2^attempt))
```

The jitter prevents synchronized retries from stampeding a provider
during transient failures. Without jitter, N agents hitting a rate limit
simultaneously would all retry at exactly the same time, creating
repeated spikes.

### Retry Classification

Retryable failures are explicit:

| Error Type | Action |
|------------|--------|
| Timeout | Retry with backoff |
| 500 Internal Server Error | Retry with backoff |
| 502/503/504 Gateway errors | Retry with backoff |
| 429 Rate Limited | Retry after Retry-After header |
| 401 Unauthorized | Fail immediately (bad key) |
| 400 Bad Request | Fail immediately (malformed) |
| Connection refused | Fail over to next provider |
| DNS resolution failure | Fail over to next provider |

### RetryAction Enum

The RetryAction enum returns a structured decision so the caller can
retry, fail over, or stop:

```rust
pub enum RetryAction {
    Retry { delay: Duration },
    Failover { reason: String },
    Fail { error: ProviderError },
}
```

---

## 4. Per-Provider Concurrency Control

Provider-specific concurrency limits via semaphores prevent both provider
overload and local resource exhaustion.

Default limits are profile-aware:

| Profile | Concurrency Posture |
|---------|-------------------|
| laptop-local | Conservative, interactive |
| single-server | Moderate, shared-machine safe |
| container | Tuned for one instance per node |
| clustered | Horizontal scale with per-node caps |
| edge | Minimal, request-scoped |

Per-tenant quotas layer on top in shared deployments. The inference
gateway enforces three-level backpressure: per-provider semaphore,
per-tenant token bucket, and global rate limiter.

---

## 5. Context Overflow Handling

When context approaches model capacity, reduce before the model fails.

### 80% Trigger Threshold

At ~80% usage:
1. Summarize older context into a smaller prompt Signal
2. Accelerate demurrage for low-value material
3. Evict least-useful items first and log the decision

At critical usage, force eviction and continue with reduced state. The
Composer tracks token counts and triggers reduction automatically.

---

## 6. Graceful Shutdown

Shutdown drains work, checkpoints state, then exits. The same path
supports regular exits and rolling upgrades.

### Phase 1: Stop Accepting

Mark the service unavailable for new work. Flip readiness probe to false.
The load balancer (Fly.io, Railway, or a reverse proxy) stops sending
new requests.

### Phase 2: Drain

Wait for in-flight requests to finish within a bounded window (default:
30 seconds). For real-time subscribers, readiness should fail before
liveness so new subscriptions stop landing while existing WebSocket/SSE
clients finish or reconnect elsewhere with their last cursor.

### Phase 3: Checkpoint and Exit

Flush durable state (Graph checkpoints, snapshot JSON, subscription
state). Persist executor progress. Close transports cleanly. The
ProcessSupervisor force-kills any agents that exceed the drain timeout.

---

## 7. Zero-Downtime Upgrades

Single-server and clustered deployments upgrade without losing in-flight
work:

- Drain traffic before terminating the old process
- Resume from the last checkpoint or saved state archive
- Keep health checks aligned so orchestrators replace one node at a time
- For clustered deployments, rolling replacement behind the load balancer

Container deployments treat upgrades as new image + state handoff, not
manual reinstall. The state volume persists across container replacements.

Clustered deployments should not rely on sticky sessions for continuity.
Shared cursor retention and replayable projection state matter more than
pinning browsers to nodes.

---

## 8. Observability

Production deployments use the same observability contract in every shape:

- Structured logs to stderr by default (via `tracing`)
- Prometheus-compatible metrics on `/metrics`
- OpenTelemetry traces around the orchestration pipeline
- Readiness (`/readyz`) and liveness (`/healthz`) probes

### Roko-Specific Metrics

| Metric | Meaning |
|--------|---------|
| `roko.gate.pass_rate` | Gate success rate |
| `roko.bus.pulses_per_second` | Bus throughput |
| `roko.substrate.query_latency_p99` | Storage latency |
| `roko.agent.active_count` | Running agents |
| `roko.provider.request_latency_p95` | Provider latency |
| `roko.c_factor` | Collective intelligence health |

### Real-Time Surface Telemetry

| Metric | Meaning |
|--------|---------|
| `roko.realtime.connections` | Open connections by transport |
| `roko.realtime.subscriptions` | Active subs by channel family |
| `roko.realtime.cursor_lag` | How far behind subscribers are |
| `roko.realtime.reconnects` | Reconnect churn during deploys |
| `roko.realtime.backpressure_dropped_total` | Updates dropped under pressure |
| `roko.realtime.auth_denied_total` | Subscribe/publish denials |

These metrics carry shape and tenant labels where cardinality is safe.

---

## 9. Health Check Patterns

`/healthz` and `/readyz` mean the same thing across Docker, Compose,
Fly.io, systemd, and clustered orchestrators.

### Readiness vs Liveness

- **Readiness** (`/readyz`): "Should traffic be sent here now?"
- **Liveness** (`/healthz`): "Is the process still healthy enough to stay up?"

During shutdown or upgrade, readiness fails before liveness so traffic
drains cleanly.

For real-time traffic:
- SSE endpoints must disable proxy buffering (`X-Accel-Buffering: no`)
- WebSocket endpoints must preserve upgrade headers through ingress
- Replay retention must outlive short restarts so reconnecting clients
  do not fall off the log

---

## 10. Content-Addressed Dedup Cache

Duplicate requests reuse cached responses when request, model, and
parameters match. Reduces cost and latency in every profile, especially
clustered and container deployments. The inference gateway manages L1
(in-process) and L2 (disk) caches with both exact and semantic matching.

Cache keys include: model identifier, temperature, system prompt hash,
and user message hash. The L1 cache is an LRU with configurable capacity;
L2 uses content-addressed files on disk.

---

## 11. Hedged Requests

Hedged requests send the same work to multiple providers when latency
matters more than token cost. Use sparingly and only when the profile
can afford duplicate work. The first response wins; the slower request
is cancelled.

---

## 12. Multi-Tenant Safety

Shared deployments need explicit tenant boundaries:

- Scope substrate keys by tenant
- Enforce per-tenant quotas (tokens, spend, episode counts)
- Keep auth and role checks tenant-aware
- Label metrics with tenant identifiers (low-cardinality only)

The point is isolation without separate codepaths per tenant. A single
roko-serve binary serves multiple tenants by scoping all state access
through tenant-qualified keys.

---

## 13. Implementation Status

> **Implementation status:** Adaptive timeouts, exponential backoff with
> jitter, and the RetryAction enum are implemented in roko-agent.
> Per-provider concurrency semaphores are scaffolded. Graceful shutdown
> is partial (ProcessSupervisor handles agents). Health check endpoints
> (`/readyz`, `/healthz`) are wired in roko-serve. The inference gateway
> manages L1/L2 dedup caches. Hedged requests are designed but not
> implemented. Multi-tenant safety is designed. Context overflow handling
> is wired in the Composer. The PeriodicObserver provides telemetry
> sampling every 30 seconds with rotation-bounded JSONL output.
