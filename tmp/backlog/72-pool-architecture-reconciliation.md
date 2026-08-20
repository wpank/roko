# 72 — Pool Architecture Reconciliation

**Priority**: P2 — four overlapping pool abstractions with no unified lifecycle, two of which have zero callers despite having 16-22 tests each
**Size**: S (1-2 days: design decision + deprecation + trait definition; actual `MultiAgentPool` wiring into runner is backlog #55)
**Crates**:
- `crates/roko-agent/src/pool.rs` — `AgentPool` (16 tests, 0 callers outside tests)
- `crates/roko-agent/src/multi_pool.rs` — `MultiAgentPool` (22 tests, 0 callers outside tests)
- `crates/roko-cli/src/dispatch/warm_pool.rs` — `WarmPool` (6 tests, wired in runner dispatch)
- `crates/roko-runtime/src/effect_driver.rs` — `EffectDriver` reference point for model-caller cache design

**Depends on**: None (this spec does not itself wire any pool into the runner; that is backlog #55)

---

## Background

The workspace has four pool-shaped abstractions built by separate development streams:

1. **`WarmPool`** in `roko-cli` — the currently-wired pool. Stores `WarmAgent` handles (id, model slug, TTL). Used by the runner-v2 event loop to pre-spawn agents for fast role transitions. Called from `event_loop.rs` at lines 4128, 4398, 4865, 10820. The pool stores agent identity metadata, not live `Arc<dyn Agent>` instances, because subprocess handles live in the agent runtime. Has 6 unit tests covering TTL eviction, capacity, and take/insert ordering.

2. **`AgentPool`** in `roko-agent` — a sequential single-role queue with fallback retry. Stores `Arc<dyn Agent>`. Built with 16 tests. Has **zero callers** outside its own test module. It runs tasks one at a time for a single role, falling back to an alternate agent if the primary fails.

3. **`MultiAgentPool`** in `roko-agent` — a parallel multi-role pool with warm pre-spawn. Stores `Arc<dyn Agent>` in both `active` and `warm` maps. Built with 22 tests. Has **zero callers** outside its own test module. It delegates per-role sequential execution to sub-queues (like `AgentPool`) and adds concurrent multi-role scheduling, warm pre-spawn eviction, concurrency limits per role, and bulk kill operations.

4. **`WarmDispatchPool`** (design doc only) — a design for a lightweight model-caller cache keyed by `(provider, model)` that eliminates `ModelCallService` construction overhead per dispatch. Referenced in design docs (`solutions/perf/`). Zero code exists.

Backlog #55 proposes replacing `WarmPool` with `MultiAgentPool` in the runner. This spec addresses the prerequisites and structural questions that #55 does not: how the four pools relate, which should be removed, and what shared interface they should implement so future consumers (TUI modal, serve metrics route) do not hard-code against a single implementation.

---

## Current State

1. **`WarmPool`** is at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs`. Struct at line 79. `WarmPoolStats` at line 68 with fields `warm`, `active`, `capacity`. Used in:
   - `event_loop.rs` line 4128: `factory.dispatcher().warm_pool().take(next_role)` — promotes pre-spawned agent
   - `event_loop.rs` line 4398: `warm_pool().evict_expired()` — evicts on gate failure
   - `event_loop.rs` line 4865: periodic TTL housekeeping
   - `event_loop.rs` line 10820: pre-spawns agent slot into warm pool
   - `dispatch/mod.rs` line 140: `warm_pool: WarmPool` field on `Dispatcher`
   - `dispatch/factory.rs` lines 135, 234: `WarmPool::new(2)` created during factory construction

2. **`AgentPool`** is at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs`. Struct at line 162. No `#[deprecated]` attribute. Has 16 `#[test]` tests. No callsite outside the test module. `MultiAgentPool` imports `AgentPool`'s supporting types (`AgentInstanceId`, `AgentTask`, `InstanceStatus`, `TaskOutcome`) in `multi_pool.rs` line 14.

3. **`MultiAgentPool`** is at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/multi_pool.rs`. Struct at line 51. Has 22 `#[test]` tests. No callsite outside test module. Public methods include: `warm_count(role: AgentRole) -> usize` (line 339), `total_warm_count() -> usize` (line 345), `active_count() -> usize` (line 650), `active_count_for_role(role: AgentRole) -> usize` (line 656).

4. **TUI agent pool modal** at `crates/roko-cli/src/tui/modals/agent_pool_modal.rs` uses its own `AgentPoolRow` struct (line 12) to display pool data. It is not connected to either `AgentPool` or `MultiAgentPool` — it gets data from a separate source. A `PoolMetrics` trait would let the modal query any pool uniformly.

5. **`WarmPool`'s `WarmPoolStats`** already has fields `warm`, `active`, `capacity`. This is an existing snapshot type that `PoolMetrics` should supersede with richer counters.

6. **No `PoolMetrics` trait exists** anywhere in the workspace. Each pool has its own ad-hoc metrics methods.

---

## Implementation Plan

### Step 1: Record the two-layer pool design decision

Add a `# Pool Architecture` section to `.roko/GAPS.md` (the canonical gap tracker) documenting the two-layer design decision:

**Layer 1 — Agent lifecycle pool (`MultiAgentPool`)**: owns `Arc<dyn Agent>` identity, role assignment, concurrency limits, warm pre-spawn with eviction, fallback retry, and bulk lifecycle operations. This is the pool that backlog #55 wires into the runner. `AgentPool` is fully subsumed: its sequential single-role behavior is `MultiAgentPool` with `concurrency_limit = 1` for that role.

**Layer 2 — Model-caller cache (future)**: owns `Arc<dyn ModelCaller>` keyed by `(provider, model)`. Eliminates per-dispatch `ModelCallService` construction. Corresponds to the `WarmDispatchPool` design doc. Sits below Layer 1 — an agent acquired from the lifecycle pool optionally uses a cached model caller. Implementation is out of scope for this spec.

The two layers compose without knowing each other's internals.

### Step 2: Mark `AgentPool` as deprecated

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs`, add above the struct definition at line 162:
```rust
/// # Deprecation
///
/// `AgentPool` is deprecated in favour of [`MultiAgentPool`], which provides a
/// strict superset of this type's behaviour: sequential single-role queuing is
/// `MultiAgentPool` with `concurrency_limit = 1` for the relevant role.
/// See backlog item #72 for the pool reconciliation plan.
#[deprecated(
    since = "0.1.0",
    note = "Use `MultiAgentPool` instead. See backlog #72."
)]
pub struct AgentPool { ... }
```

Also mark `AgentPool::new()` and other inherent methods with `#[allow(deprecated)]` inside the `impl` block so the implementation itself does not trigger the warning.

### Step 3: Audit `AgentPool` tests for coverage gaps

Compare the 16 `AgentPool` tests against the 22 `MultiAgentPool` tests. The key `AgentPool` behaviors to verify are covered by `MultiAgentPool`:
- Primary agent success: `MultiAgentPool` `accept()` + `complete()` covers this
- Fallback on primary failure: `MultiAgentPool` has `fallbacks: HashMap<AgentRole, Arc<dyn Agent>>`; check that the fallback path is tested
- Queue ordering (FIFO task delivery): covered by `MultiAgentPool`'s per-role queue
- Empty pool rejection: covered by concurrency limit check

For any coverage gap, add a test to `multi_pool.rs` before this spec is complete.

### Step 4: Define `PoolMetrics` trait in `roko-agent`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs` (or a new `pool_metrics.rs` module), define:

```rust
/// Uniform observability interface for any agent pool implementation.
pub trait PoolMetrics: Send + Sync {
    /// Number of agents in the warm (pre-spawned, idle) pool.
    fn warm_count(&self) -> usize;
    /// Number of currently active (running) agent instances.
    fn active_count(&self) -> usize;
    /// Maximum number of concurrent active instances across all roles.
    fn total_capacity(&self) -> usize;
    /// Point-in-time snapshot of all pool counters.
    fn snapshot(&self) -> PoolSnapshot;
}

/// Point-in-time pool metrics snapshot.
#[derive(Debug, Clone, Default)]
pub struct PoolSnapshot {
    pub warm: usize,
    pub active: usize,
    pub capacity: usize,
    /// Number of idle-TTL evictions since the pool was created.
    pub idle_evictions: u64,
    /// Number of warm-pool hits (task got a pre-spawned agent).
    pub warm_hits: u64,
    /// Number of warm-pool misses (task had to cold-spawn).
    pub cold_misses: u64,
}
```

Export `PoolMetrics` and `PoolSnapshot` from `roko-agent`'s crate root (`lib.rs`).

### Step 5: Implement `PoolMetrics` for `MultiAgentPool`

In `multi_pool.rs`, add hit/miss/eviction counters to `MultiAgentPool`. Use `AtomicU64` for lockless accumulation:
```rust
pub struct MultiAgentPool {
    // ... existing fields ...
    warm_hits: std::sync::atomic::AtomicU64,
    cold_misses: std::sync::atomic::AtomicU64,
    idle_evictions: std::sync::atomic::AtomicU64,
}
```

Increment `warm_hits` when `accept()` promotes a warm entry. Increment `cold_misses` when no warm entry is available. Increment `idle_evictions` when `evict_stale()` removes an entry.

Then implement the trait:
```rust
impl PoolMetrics for MultiAgentPool {
    fn warm_count(&self) -> usize { self.total_warm_count() }
    fn active_count(&self) -> usize { self.active_count() }
    fn total_capacity(&self) -> usize {
        // sum of per-role concurrency limits, or a configured total
        self.concurrency_limits.values().sum::<usize>()
            .max(self.default_concurrency_limit * 8)
    }
    fn snapshot(&self) -> PoolSnapshot {
        use std::sync::atomic::Ordering::Relaxed;
        PoolSnapshot {
            warm: self.total_warm_count(),
            active: self.active_count(),
            capacity: self.total_capacity(),
            idle_evictions: self.idle_evictions.load(Relaxed),
            warm_hits: self.warm_hits.load(Relaxed),
            cold_misses: self.cold_misses.load(Relaxed),
        }
    }
}
```

### Step 6: Plan `WarmPool` retirement (post-#55)

Do NOT remove `WarmPool` in this spec. The removal sequence is:
1. This spec: mark `AgentPool` deprecated; define `PoolMetrics`; implement on `MultiAgentPool`.
2. Backlog #55: wire `MultiAgentPool` into runner, replacing the `WarmPool` usage in `dispatch/mod.rs` and `event_loop.rs`.
3. After #55 merges: delete `warm_pool.rs`, remove `WarmPool` from `dispatch/mod.rs` and `factory.rs`.

Document this in the deprecation note on `WarmPool` itself (add a doc comment but no `#[deprecated]` attribute yet — the attribute would cause compile warnings in production code before the replacement is wired):
```rust
/// # Future removal
///
/// `WarmPool` will be replaced by `MultiAgentPool` when backlog #55 lands.
/// See pool architecture decision in `.roko/GAPS.md`.
```

---

## Acceptance Criteria

1. A written two-layer pool architecture decision is recorded in `.roko/GAPS.md` explaining that `MultiAgentPool` is Layer 1 (agent lifecycle) and a future model-caller cache is Layer 2.

2. `AgentPool` in `pool.rs` carries a `#[deprecated]` attribute with a migration note pointing to `MultiAgentPool` and backlog #72.

3. All `AgentPool` test coverage scenarios are also covered by `MultiAgentPool` tests. If any gap exists, a new test is added to `multi_pool.rs`.

4. `PoolMetrics` trait and `PoolSnapshot` struct are defined in `roko-agent` and exported from its crate root.

5. `MultiAgentPool` implements `PoolMetrics`. Its `snapshot()` method returns accurate warm, active, capacity, and counter values.

6. `WarmPool` has a doc comment noting its planned retirement after #55 but does NOT have a `#[deprecated]` attribute (it is still in use).

7. No new pool abstractions are introduced without implementing `PoolMetrics`.

8. `cargo test -p roko-agent` passes.

9. `cargo clippy --workspace --no-deps -- -D warnings` is clean. The `#[deprecated]` attribute on `AgentPool` will cause warnings in code that calls it; all call sites are only in the test module (which can add `#[allow(deprecated)]`).

---

## Verification Checklist

- [ ] `grep -n "deprecated" crates/roko-agent/src/pool.rs` shows the attribute on `AgentPool`
- [ ] `grep -n "PoolMetrics\|PoolSnapshot" crates/roko-agent/src/ -r` shows the trait definition and impl
- [ ] `grep -n "pub use.*PoolMetrics\|pub use.*PoolSnapshot" crates/roko-agent/src/lib.rs` shows the exports
- [ ] `cargo test -p roko-agent` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes (the `AgentPool` test module needs `#[allow(deprecated)]` to silence the deprecation warning within the test)
- [ ] `.roko/GAPS.md` contains a "Pool Architecture" section documenting the two-layer decision
- [ ] No `#[deprecated]` attribute appears on `WarmPool` (it is still in production use)

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs` | Add `#[deprecated]` to `AgentPool`; add `PoolMetrics` trait and `PoolSnapshot` struct; add `#[allow(deprecated)]` in test module |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/multi_pool.rs` | Add `warm_hits`, `cold_misses`, `idle_evictions` atomic counters; increment at correct sites; implement `PoolMetrics` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lib.rs` | Export `PoolMetrics` and `PoolSnapshot` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs` | Add a doc comment noting planned retirement after #55 (no `#[deprecated]` attribute) |
| `/Users/will/dev/nunchi/roko/roko/tmp/backlog/` / `.roko/GAPS.md` | Record two-layer pool design decision |

## Files NOT to Modify

| File | Why |
|---|---|
| `crates/roko-cli/src/dispatch/warm_pool.rs` | Still in production use; only add doc comment |
| `crates/roko-cli/src/runner/event_loop.rs` | WarmPool wiring is backlog #55 scope; do not touch |
| `crates/roko-cli/src/dispatch/factory.rs` | WarmPool construction is #55 scope |
| `crates/roko-runtime/src/effect_driver.rs` | Layer 2 model-caller cache is future scope |

---

## Not in Scope

- Actually wiring `MultiAgentPool` into the runner — that is backlog #55.
- Implementing the model-caller cache (`WarmDispatchPool`) — that is a separate performance backlog item.
- Claude CLI subprocess warming (requires process handle management orthogonal to pool abstractions).
- `ProviderSemaphores` — not a pool; it is a concurrency control primitive and remains separate.
- Removing `WarmPool` — not until backlog #55 completes the wiring.
- Removing `AgentPool` — not until all references are gone; the `#[deprecated]` attribute is sufficient for now.
