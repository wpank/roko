# Inference Cache: Two-Layer Exact + Semantic Cache

**Status:** Backlog
**Priority:** P1 (direct cost savings)
**Size:** L (0 days — already implemented)
**Origin:** `tmp/architecture-archive/07-gateway.md`, Stages 2-3 "Cache lookup"

---

## Problem Statement

Every provider call costs money. A significant fraction of inference requests
during plan execution are redundant: the same agent receives the same task
description, the same tool schemas, and the same context, differing only in
volatile metadata like session IDs, timestamps, and git status blocks. Without
caching, each of these pays full provider cost.

Two failure modes require two distinct solutions:

1. **Exact duplicates** — byte-for-byte identical requests after stripping
   volatile fields. Common during plan retries, TUI polling agents, and test
   harness reruns.
2. **Near-duplicates** — semantically equivalent requests that differ in
   superficial text (e.g., whitespace, non-semantic field reorder, minor
   prompt variation). Exact hashing misses these.

---

## Proposed Solution

### L1: Exact hash cache (blake3)

Normalize the request body before hashing:

- Strip UUIDs matching `[0-9a-f]{8}-...-[0-9a-f]{12}`
- Strip ISO timestamps
- Strip `cch=` hashes, `CWD:` lines, `Date:` headers
- Replace git status blocks with `[GIT_STATUS]`
- Sort JSON keys alphabetically
- Sort tool definitions by name

Hash the normalized body with blake3 (32-byte output). Look up in a bounded LRU
map (default capacity: 10,000 entries). On a hit, return the cached response
without calling any provider.

**Regime-aware TTL** (driven by `CacheRegime`):

| Regime | TTL | Rationale |
|--------|-----|-----------|
| Normal | 3600s | Standard operating conditions |
| Calm | 7200s | Low activity; cached responses stay valid longer |
| Volatile | 900s | Rapid changes; expire faster to avoid stale responses |
| Crisis | 300s | Active failures; maximize freshness |

**Exclusions** (never cached):
- `stop_reason == ToolUse` (tool call IDs are ephemeral)
- `output_tokens < 3` (too short to be a useful cache entry)
- Error responses

### L2: Semantic near-miss cache (SimHash)

Compute a 64-bit SimHash fingerprint of the request's semantic text:

1. Tokenize on non-alphanumeric boundaries
2. For each token, hash to 64 bits; increment/decrement per-bit counters
3. Final fingerprint: 1 for positive counter, 0 for negative

Store fingerprint → response in a `DashMap<u64, SimHashEntry>` (max 5,000
entries). On lookup, scan for fingerprints with Hamming distance ≤ 3 from the
query fingerprint. Return the closest match within the same namespace and model.

Fixed TTL: 7200s (not regime-aware; semantic matches are fuzzier, so a
conservative fixed window avoids stale near-misses).

**Namespace isolation**: each tenant/workspace prefixes its semantic text with
its namespace, preventing cross-tenant hits in multi-user deployments.

---

## Implementation Location

**Already fully implemented** in `crates/roko-gateway/src/cache.rs`.

Key types:

```rust
pub struct InferenceCache {
    l1: Mutex<L1State>,              // HashMap<[u8; 32], CachedResponse> + VecDeque LRU
    l2: DashMap<u64, SimHashEntry>,  // lock-free concurrent read
    l1_capacity: usize,              // default 10_000
    l2_capacity: usize,              // default 5_000
    // atomic counters for l1_hits, l2_hits, misses, excluded
}

pub struct CachedResponse {
    pub body: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub cached_at: Instant,
    pub effective_ttl: Duration,
}

pub struct SimHashEntry {
    pub response: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,
}
```

Public API:

```rust
impl InferenceCache {
    pub fn lookup(&self, request: &InferenceRequest) -> Option<CachedResponse>;
    pub fn lookup_with_layer(&self, request: &InferenceRequest) -> Option<CacheHit>;
    pub fn store(&self, request: &InferenceRequest, response: &InferenceResponse);
    pub fn store_with_cost(&self, request: &InferenceRequest, response: &InferenceResponse, cost_usd: f64);
    pub fn stats(&self) -> CacheStats;
    pub fn sizes(&self) -> (usize, usize);
}
```

Helper functions `normalize_request`, `simhash`, `hamming_distance`, and
`ttl_for` are all public and independently testable.

The `InferenceCache` is wired into the nine-stage `InferenceGateway` pipeline
in `crates/roko-gateway/src/gateway.rs` (Stages 2 and 8: cache lookup and
cache store). The pipeline is exposed through `POST /api/gateway/inference`
in `roko-serve`. Cache hit counters (`l1_hits`, `l2_hits`, `misses`,
`excluded`) are returned by `GET /api/gateway/stats` via the `pipeline` field.

Note: the L1 implementation uses a `Mutex<HashMap>` + `VecDeque` for LRU
tracking rather than the `moka` async LRU mentioned in the architecture doc.
This is intentional — `moka` was not added as a dependency; the manual approach
provides equivalent behavior with no additional crate.

---

## Acceptance Criteria

All acceptance criteria are already met:

- [x] `normalize_request` strips UUIDs, timestamps, `cch=` hashes, `CWD:` and
  `Date:` lines, git status blocks, and sorts tool definitions by name.
- [x] Two normalized requests that differ only in volatile fields produce the
  same blake3 hash.
- [x] L1 cache enforces the capacity limit via LRU eviction.
- [x] L1 TTLs are regime-aware: `Normal=3600s`, `Calm=7200s`,
  `Volatile=900s`, `Crisis=300s`.
- [x] `stop_reason=ToolUse` and `output_tokens<3` responses are excluded.
- [x] L2 cache enforces a 5,000-entry capacity limit by evicting the oldest
  entry when full.
- [x] L2 lookup matches fingerprints within Hamming distance ≤ 3.
- [x] L2 hits respect namespace isolation (different namespaces never match).
- [x] `lookup_with_layer` returns a `CacheLayer` discriminant (`L1` or `L2`).
- [x] Four unit tests cover: normalization, TTL/exclusion policy, L1→L2
  promotion and namespace isolation, and capacity eviction.

---

## Current State

**This feature is complete.** No implementation work is needed.

The main inference path used by CLI agents (`ModelCallService` in `roko-agent`)
does not flow through this cache — it calls providers directly. If L1/L2
savings are wanted for runner-dispatched tasks (not just HTTP gateway calls),
a separate task is needed to route `ModelCallService` calls through
`InferenceGateway` or to apply `InferenceCache` inside `ModelCallService`.

The L2 eviction under concurrent write is a potential improvement: when `l2.len()
>= l2_capacity`, the current code scans all entries to find the oldest
(`O(n)`). Under high concurrency this could be replaced with a secondary
timestamp-ordered index, but for the current volume (5,000 entries) this is
not a bottleneck.

---

## References

- `crates/roko-gateway/src/cache.rs` — implementation and unit tests
- `crates/roko-gateway/src/gateway.rs` — Stages 2 and 8 pipeline wiring
- `crates/roko-serve/src/routes/gateway.rs` — HTTP surface, `pipeline` stats field
- `tmp/architecture-archive/07-gateway.md` — original design (Sections 2 and 3)
- `.roko/GAPS.md` — E26 entry: "Inference gateway 12/12 ... two-layer caching"
