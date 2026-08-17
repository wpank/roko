# Warm Agent Pool Design

Date: 2026-04-29 (expanded from 2026-04-28)

---

## Problem Statement

Every `roko run` / plan task / agent dispatch creates a fresh agent instance:
- **API path**: New `ModelCallService` construction + provider resolution (~20-50ms)
- **Claude CLI path**: Fork subprocess (200-500ms)
- **All paths**: Safety contract load, tool dispatcher init, MCP discovery

For sub-500ms targets with standard workflows (2 agent calls per run), this cold-start
cost multiplies: 2x agents x 50ms = 100ms overhead minimum.

For plan execution (10+ tasks, potentially 3-wide parallel), cold-start adds up:
10 tasks x 50ms = 500ms total agent construction time.

---

## Current State: What Already Exists

### MultiAgentPool (built, not wired)

**File**: `crates/roko-agent/src/multi_pool.rs`

The `MultiAgentPool` already supports:
- Pre-spawned warm entries (`WarmEntry` struct with `agent`, `spawned_at`, `reuse_policy`)
- Active instance tracking with `InstanceStatus` lifecycle
- Concurrency limits per role (default: 4)
- Fallback agents per role
- Bulk kill operations (all, by plan, by role)

```rust
// crates/roko-agent/src/multi_pool.rs:51-62
pub struct MultiAgentPool {
    active: HashMap<AgentInstanceId, ActiveEntry>,
    warm: HashMap<(AgentRole, String), WarmEntry>,
    fallbacks: HashMap<AgentRole, Arc<dyn Agent>>,
    concurrency_limits: HashMap<AgentRole, usize>,
    default_concurrency: usize,
}
```

### WarmReusePolicy (built, not wired)

**File**: `crates/roko-agent/src/session.rs`

Session management with reuse policies:
```rust
pub struct WarmReusePolicy {
    pub scope: ReuseScope,
    pub max_reuses: u32,
    pub ttl: Duration,
}

pub enum ReuseScope {
    SameRun,      // reuse within a single workflow run
    SameModel,    // reuse for any task using the same model
    SamePlan,     // reuse across tasks in the same plan
    Global,       // reuse across all dispatches
}
```

### AgentPool (built, partially wired)

**File**: `crates/roko-agent/src/pool.rs`

Lower-level pool that tracks individual agent instances:
- `AgentInstanceId` with plan, role, instance name
- `InstanceStatus` enum: Idle, Running, Failed, Completed
- `TaskOutcome` with timing and token usage

### Shared HTTP Client (built, fully wired)

**File**: `crates/roko-agent/src/provider/mod.rs:88-110`

The `SHARED_HTTP_CLIENT` static is fully wired and used by all HTTP-based backends.
This eliminates TLS handshake costs for repeated requests to the same provider within
the 90s idle timeout window.

### ResponseCache (built, wired in tool loop)

**File**: `crates/roko-agent/src/cache.rs`

Content-addressed response cache with 30s TTL. Used by the tool loop to deduplicate
identical backend requests within a turn. Not yet used at the workflow engine level.

---

## Architecture: Three-Tier Warm Pool

```
                                 ┌──────────────────────────────────┐
                                 │       WorkflowEngine             │
                                 │  crates/roko-runtime/src/        │
                                 │  workflow_engine.rs              │
                                 └──────────┬───────────────────────┘
                                            │
                                            ▼
                         ┌──────────────────────────────────────────┐
                         │            EffectDriver                   │
                         │  spawn_agent() → pool.acquire()           │
                         │  crates/roko-runtime/src/effect_driver.rs │
                         └──────────┬───────────────────────────────┘
                                    │
                         ┌──────────▼───────────────────────────────┐
                         │         WarmDispatchPool                  │
                         │  (NEW: wraps MultiAgentPool + services)   │
                         └──────────┬───────────────────────────────┘
                                    │
              ┌─────────────────────┼─────────────────────┐
              │                     │                     │
        ┌─────▼─────┐        ┌─────▼─────┐        ┌─────▼─────┐
        │  Tier 1    │        │  Tier 2    │        │  Tier 3    │
        │  HOT       │        │  WARM      │        │  COLD      │
        │            │        │            │        │            │
        │  Active    │        │  Pre-built │        │  On-demand │
        │  agents    │        │  agents    │        │  construct │
        │  in-flight │        │  idle,     │        │  from      │
        │            │        │  ready     │        │  scratch   │
        │  Acquire:  │        │  Acquire:  │        │  Acquire:  │
        │  0ms       │        │  <5ms      │        │  20-50ms   │
        └────────────┘        └────────────┘        └────────────┘
```

### Tier 1: HOT (in-flight agents)

Agents currently executing a task. Not available for new work until their current
task completes. Tracked in `MultiAgentPool.active`.

### Tier 2: WARM (pre-built, idle agents)

Agents that have been constructed and are ready to accept work immediately. They have:
- Model caller pre-configured for a specific provider
- Prompt assembler loaded with conventions and knowledge context
- Safety contract cached
- HTTP connections warm (via SHARED_HTTP_CLIENT)

Stored in `MultiAgentPool.warm`. Evicted after `idle_timeout` (default: 5 minutes).

### Tier 3: COLD (on-demand construction)

When no warm agent matches the requested provider/model, construct one from scratch.
This is the current behavior -- all dispatches go through tier 3 today.

---

## Implementation Plan

### Step 1: WarmDispatchPool (new struct)

**File to create**: `crates/roko-runtime/src/warm_dispatch_pool.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use roko_core::foundation::{ModelCallRequest, ModelCallResponse, ModelCaller};
use tokio::sync::Mutex;

/// Pool configuration.
pub struct WarmPoolConfig {
    /// Maximum number of warm slots across all providers.
    pub max_warm_slots: usize,               // default: 4
    /// Maximum concurrent active dispatches.
    pub max_active: usize,                   // default: 8
    /// How long a warm slot stays alive without use.
    pub idle_timeout: Duration,              // default: 5 minutes
    /// Whether to pre-warm slots on pool creation.
    pub pre_warm: bool,                      // default: false for CLI, true for serve
    /// Provider/model pairs to pre-warm.
    pub pre_warm_targets: Vec<(String, String)>, // e.g., [("openai", "gpt-4.1-mini")]
}

impl Default for WarmPoolConfig {
    fn default() -> Self {
        Self {
            max_warm_slots: 4,
            max_active: 8,
            idle_timeout: Duration::from_secs(300),
            pre_warm: false,
            pre_warm_targets: Vec::new(),
        }
    }
}

/// A warm dispatch slot: pre-configured model caller ready for immediate use.
struct WarmSlot {
    /// Provider identifier (e.g., "openai", "moonshot").
    provider: String,
    /// Model slug (e.g., "gpt-4.1-nano").
    model: String,
    /// Pre-configured model caller.
    caller: Arc<dyn ModelCaller>,
    /// When this slot was created.
    created_at: Instant,
    /// When this slot was last used.
    last_used: Instant,
    /// Number of dispatches served by this slot.
    dispatches_served: u64,
    /// Current state.
    state: SlotState,
}

enum SlotState {
    Idle,
    Active { run_id: String, since: Instant },
    Draining, // finishing current work, will not accept new
}

/// Metrics for the warm pool.
#[derive(Debug, Default, Clone)]
pub struct WarmPoolMetrics {
    pub total_dispatches: u64,
    pub warm_hits: u64,       // reused a warm slot
    pub cold_misses: u64,     // had to construct fresh
    pub evictions: u64,       // idle timeout evictions
    pub peak_active: u64,
    pub avg_acquire_us: f64,  // microseconds
    pub avg_dispatch_ms: f64,
}

/// Pool that manages warm model callers for fast dispatch.
pub struct WarmDispatchPool {
    config: WarmPoolConfig,
    slots: Mutex<Vec<WarmSlot>>,
    metrics: Mutex<WarmPoolMetrics>,
    /// Factory function to construct a new ModelCaller for a given provider+model.
    factory: Arc<dyn Fn(&str, &str) -> Option<Arc<dyn ModelCaller>> + Send + Sync>,
}

impl WarmDispatchPool {
    /// Create a new pool with the given config and factory.
    pub fn new(
        config: WarmPoolConfig,
        factory: Arc<dyn Fn(&str, &str) -> Option<Arc<dyn ModelCaller>> + Send + Sync>,
    ) -> Self {
        Self {
            config,
            slots: Mutex::new(Vec::new()),
            metrics: Mutex::new(WarmPoolMetrics::default()),
            factory,
        }
    }

    /// Acquire a model caller for the given provider and model.
    ///
    /// Returns a guard that auto-releases the slot on drop.
    pub async fn acquire(
        &self,
        provider: &str,
        model: &str,
    ) -> Option<WarmSlotGuard> {
        let start = Instant::now();
        let mut slots = self.slots.lock().await;

        // 1. Try exact match (same provider + model)
        if let Some(idx) = slots.iter().position(|s|
            matches!(s.state, SlotState::Idle) &&
            s.provider == provider &&
            s.model == model
        ) {
            slots[idx].state = SlotState::Active {
                run_id: String::new(),
                since: Instant::now(),
            };
            let caller = Arc::clone(&slots[idx].caller);
            self.record_hit(start).await;
            return Some(WarmSlotGuard {
                pool: self,
                slot_idx: idx,
                caller,
            });
        }

        // 2. Try same provider, different model (model is a request parameter, not baked in)
        if let Some(idx) = slots.iter().position(|s|
            matches!(s.state, SlotState::Idle) && s.provider == provider
        ) {
            slots[idx].model = model.to_string();
            slots[idx].state = SlotState::Active {
                run_id: String::new(),
                since: Instant::now(),
            };
            let caller = Arc::clone(&slots[idx].caller);
            self.record_hit(start).await;
            return Some(WarmSlotGuard {
                pool: self,
                slot_idx: idx,
                caller,
            });
        }

        // 3. Construct cold (if under max)
        if slots.len() < self.config.max_warm_slots + self.config.max_active {
            if let Some(caller) = (self.factory)(provider, model) {
                let slot = WarmSlot {
                    provider: provider.to_string(),
                    model: model.to_string(),
                    caller: Arc::clone(&caller),
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    dispatches_served: 0,
                    state: SlotState::Active {
                        run_id: String::new(),
                        since: Instant::now(),
                    },
                };
                let idx = slots.len();
                slots.push(slot);
                self.record_miss(start).await;
                return Some(WarmSlotGuard {
                    pool: self,
                    slot_idx: idx,
                    caller,
                });
            }
        }

        self.record_miss(start).await;
        None
    }

    /// Pre-warm slots for configured targets.
    pub async fn pre_warm(&self) {
        for (provider, model) in &self.config.pre_warm_targets {
            if let Some(caller) = (self.factory)(provider, model) {
                let slot = WarmSlot {
                    provider: provider.clone(),
                    model: model.clone(),
                    caller,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    dispatches_served: 0,
                    state: SlotState::Idle,
                };
                self.slots.lock().await.push(slot);
            }
        }
    }

    /// Evict idle slots past their timeout.
    pub async fn evict_idle(&self) {
        let mut slots = self.slots.lock().await;
        let deadline = Instant::now() - self.config.idle_timeout;
        let before = slots.len();
        slots.retain(|s| {
            !matches!(s.state, SlotState::Idle) || s.last_used > deadline
        });
        let evicted = before - slots.len();
        if evicted > 0 {
            self.metrics.lock().await.evictions += evicted as u64;
        }
    }

    /// Release a slot back to warm state.
    async fn release(&self, idx: usize) {
        let mut slots = self.slots.lock().await;
        if idx < slots.len() {
            slots[idx].state = SlotState::Idle;
            slots[idx].last_used = Instant::now();
            slots[idx].dispatches_served += 1;
        }
    }

    async fn record_hit(&self, start: Instant) {
        let mut m = self.metrics.lock().await;
        m.total_dispatches += 1;
        m.warm_hits += 1;
        let us = start.elapsed().as_micros() as f64;
        m.avg_acquire_us = (m.avg_acquire_us * (m.total_dispatches - 1) as f64 + us)
            / m.total_dispatches as f64;
    }

    async fn record_miss(&self, start: Instant) {
        let mut m = self.metrics.lock().await;
        m.total_dispatches += 1;
        m.cold_misses += 1;
        let us = start.elapsed().as_micros() as f64;
        m.avg_acquire_us = (m.avg_acquire_us * (m.total_dispatches - 1) as f64 + us)
            / m.total_dispatches as f64;
    }

    /// Current pool metrics.
    pub async fn metrics(&self) -> WarmPoolMetrics {
        self.metrics.lock().await.clone()
    }
}

/// RAII guard that releases a slot back to the pool on drop.
pub struct WarmSlotGuard<'a> {
    pool: &'a WarmDispatchPool,
    slot_idx: usize,
    pub caller: Arc<dyn ModelCaller>,
}

impl<'a> WarmSlotGuard<'a> {
    /// Dispatch a model call through this warm slot.
    pub async fn call(&self, request: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
        self.caller.call(request).await
    }
}

// Note: Drop cannot be async, so release must be called explicitly
// or via a background task. In practice, the WorkflowEngine calls
// pool.release(idx) after each spawn_agent completes.
```

### Step 2: Wire into EffectDriver

**File**: `crates/roko-runtime/src/effect_driver.rs`

Add an optional `WarmDispatchPool` to `EffectServices`:

```rust
pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<tokio::sync::Mutex<dyn AffectPolicy>>>,
    // NEW:
    pub warm_pool: Option<Arc<WarmDispatchPool>>,
}
```

In `spawn_agent()`, try the pool first:

```rust
pub async fn spawn_agent(&self, role: &str, user_prompt: &str, context: Option<&str>)
    -> PipelineInput
{
    // ... prompt assembly, modulation, etc. ...

    let caller = if let Some(ref pool) = self.services.warm_pool {
        // Try warm pool first
        if let Some(guard) = pool.acquire(&provider, &model).await {
            guard.caller
        } else {
            Arc::clone(&self.services.model_caller) // fallback to cold
        }
    } else {
        Arc::clone(&self.services.model_caller)
    };

    let result = caller.call(request).await;
    // ... handle result ...
}
```

### Step 3: Wire into WorkflowEngine

**File**: `crates/roko-runtime/src/workflow_engine.rs`

Add pool lifecycle management:

```rust
impl WorkflowEngine {
    pub async fn run(&self, config: WorkflowRunConfig) -> Result<WorkflowResult> {
        // Pre-warm if configured
        if let Some(ref pool) = self.driver.services.warm_pool {
            pool.pre_warm().await;
        }

        let result = self.run_inner(config).await;

        // Evict idle slots
        if let Some(ref pool) = self.driver.services.warm_pool {
            pool.evict_idle().await;
        }

        result
    }
}
```

### Step 4: Wire into roko serve

**File**: `crates/roko-serve/src/runtime.rs`

For the HTTP server, pre-warm on startup and run periodic eviction:

```rust
// In serve startup:
let pool_config = WarmPoolConfig {
    pre_warm: true,
    pre_warm_targets: vec![
        ("openai".into(), "gpt-4.1-mini".into()),
        ("anthropic".into(), "claude-sonnet-4".into()),
    ],
    idle_timeout: Duration::from_secs(600),
    ..Default::default()
};
let pool = Arc::new(WarmDispatchPool::new(pool_config, model_caller_factory));
pool.pre_warm().await;

// Periodic eviction task:
let pool_clone = Arc::clone(&pool);
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        pool_clone.evict_idle().await;
    }
});
```

### Step 5: Config integration

**File**: `crates/roko-core/src/config/mod.rs`

```toml
# roko.toml
[conductor.warm_pool]
enabled = true
max_warm_slots = 4
max_active = 8
idle_timeout_secs = 300
pre_warm_on_serve = true
pre_warm_providers = ["openai", "anthropic"]
pre_warm_models = ["gpt-4.1-mini", "claude-sonnet-4"]
```

---

## Pre-Warming Strategy

### For `roko serve` (HTTP server)

1. Parse config -> identify configured providers
2. For each in `pre_warm_targets`: create one warm slot
3. Slot creation:
   a. Construct `ModelCallService` for the provider+model
   b. Send a lightweight healthcheck request to warm the TLS connection
   c. Load safety contract for `implementer` role, cache
   d. Mark slot as Idle
4. Start periodic eviction (every 60s)

### For `roko run` (CLI one-shot)

1. If pool empty/cold: construct inline (same as today, no regression)
2. First request warms the connection in the SHARED_HTTP_CLIENT
3. Second request (reviewer) reuses the warm connection (<5ms)
4. After completion: no cleanup needed (process exits)

### For `roko plan run` (multi-task plan execution)

1. Pre-warm one slot per provider used in the plan
2. As tasks complete, slots return to warm state
3. Next task acquires the warm slot instantly
4. Parallel tasks may exceed warm slots -> construct cold (tier 3)

---

## Connection Reuse Analysis

The shared HTTP client already provides connection reuse within the 90s window.
The warm pool adds on top of this by keeping the full agent construction cached:

| Component | SHARED_HTTP_CLIENT | + Warm Pool |
|---|---|---|
| TCP connection | Pooled (90s) | Same |
| TLS session | Pooled (90s) | Same |
| Model caller construction | Per-dispatch | Cached |
| Safety contract load | Per-dispatch | Cached |
| Prompt assembler setup | Per-dispatch | Cached |
| Provider resolution | Per-dispatch | Cached |
| Rate limiter state | Shared (static) | Same |
| Total acquire time | 20-50ms | <5ms |

---

## Speculative Execution Extension

For predictable workflows (express template: implement -> gate -> commit), the pool
can speculatively pre-warm the next phase's agent while the current phase runs:

```rust
// While implementer is running:
let speculative = pool.pre_warm_for_next_phase("reviewer", &model).await;

// When implementation completes:
let reviewer_guard = pool.acquire("reviewer_provider", "reviewer_model").await;
// Instant -- already warm from speculation
```

This saves the cold-start time of the reviewer agent (20-50ms) by overlapping it
with the implementer's inference time (which is always >500ms).

---

## Self-Learning Loop Integration

The warm pool feeds metrics into the cybernetic learning loop:

```
dispatch → pool.acquire()
         → model_caller.call()
         → measure(latency, quality, cost)
         → cascade_router.observe(model, latency, quality)
         → pool.metrics.update()
         → if warm_hit_rate < 70%:
              adjust pre_warm_targets to match actual usage patterns
         → if avg_acquire_us > 1000:
              scale up max_warm_slots
```

The cascade router at `crates/roko-cli/src/orchestrate.rs` already weights latency
at 0.2 in routing decisions. Making the pool report actual dispatch latencies
(including cold-start penalty) closes the feedback loop:
- Fast models that deliver quality AND have warm connections get routed more tasks
- Models with consistently cold connections (rare providers) get routed fewer tasks
- Pool auto-scales pre-warm targets to match observed dispatch patterns

---

## Files to Create/Modify

| File | Change | Effort |
|------|--------|--------|
| `crates/roko-runtime/src/warm_dispatch_pool.rs` | **NEW** -- WarmDispatchPool, WarmSlotGuard | 4h |
| `crates/roko-runtime/src/lib.rs` | Export `warm_dispatch_pool` module | 5min |
| `crates/roko-runtime/src/effect_driver.rs` | Add `warm_pool` to EffectServices, use in spawn_agent | 1h |
| `crates/roko-runtime/src/workflow_engine.rs` | Pre-warm on start, evict on complete | 30min |
| `crates/roko-cli/src/run.rs` | Construct pool in `run_once()`, pass to EffectServices | 1h |
| `crates/roko-cli/src/orchestrate.rs` | Wire pool into plan execution | 2h |
| `crates/roko-serve/src/runtime.rs` | Pre-warm on serve startup, periodic eviction | 1h |
| `crates/roko-core/src/config/mod.rs` | Add `WarmPoolConfig` to schema | 30min |

**Total estimated effort**: 10-12h

---

## Expected Impact

| Metric | Before | After (warm hit) | After (cold miss) |
|--------|--------|-------------------|-------------------|
| Agent construct (API) | 20-50ms | <5ms | 20-50ms (unchanged) |
| Agent construct (CLI) | 200-500ms | N/A (use API) | 200-500ms |
| TLS handshake | Per-provider, pooled 90s | Same | Same |
| Connection reuse | Via SHARED_HTTP_CLIENT | Same | Same |
| Contract loading | Per-dispatch | Cached | Per-dispatch |
| Provider resolution | Per-dispatch | Cached | Per-dispatch |
| Pool acquire overhead | N/A | <5ms (lock + lookup) | 20-50ms (construct) |
| 10-task plan (sequential) | 500ms total construct | 50ms warm + 50ms cold | 500ms |
| 10-task plan (3-wide) | 150ms total construct | 15ms warm + 50ms cold | 150ms |
