# 20-gateway/02 -- Two-Layer Inference Cache

> L1 exact-match (blake3) and L2 semantic (SimHash) cache layers with regime-aware
> TTLs, namespace isolation, and bounded concurrent storage.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md), sections 4 and 10

**Source:** `crates/roko-gateway/src/cache.rs`

---

## 1. Design Rationale

LLM inference is expensive and frequently repetitive. Agent loops often re-issue
identical or near-identical requests as context evolves. The two-layer cache addresses
both cases:

- **L1 (exact):** Catches literally identical requests after volatile field
  normalization. Zero false positives. Regime-aware TTL.
- **L2 (semantic):** Catches semantically equivalent requests that differ textually
  (e.g., different timestamps, reworded system prompts, reordered tool schemas).
  Tunable false positive rate via Hamming distance threshold.

Both layers are bounded and concurrent, suitable for multi-agent workloads.

---

## 2. L1: Exact Hash Cache

### Normalization Pipeline

Before hashing, the request is normalized to strip volatile fields:

1. **Clear identity fields:** `session_id`, `agent_id`, `budget_remaining`,
   `tool_calls`, `progress_marker` are zeroed/cleared.
2. **Sort tool definitions** alphabetically by name.
3. **Serialize to JSON** via `serde_json::to_value`.
4. **Recursively scrub volatile strings:**
   - UUIDs: `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}` -> `[UUID]`
   - ISO timestamps: `2026-08-16T12:30:00Z` -> `[TIMESTAMP]`
   - Cache content hashes: `cch=deadbeef` -> `cch=[HASH]`
   - Git status blocks: delimited by known markers -> `[GIT_STATUS]`
   - `CWD:` and `Date:` lines: removed entirely
5. **Hash** the normalized JSON string with **blake3** -> `[u8; 32]`.

This ensures that two requests differing only in timestamps, working-directory metadata,
or session identity produce the same hash.

### Storage

```rust
struct L1State {
    entries: HashMap<[u8; 32], CachedResponse>,
    recency: VecDeque<[u8; 32]>,
}
```

Default capacity: **10,000 entries**. LRU eviction: when capacity is exceeded, the
oldest entry by access time is removed. Access (`touch`) moves the entry to the back
of the recency deque.

### Regime-Aware TTL

The `CacheRegime` enum maps the agent's operating regime (from `CorticalState`) to a
TTL:

```rust
pub const fn ttl_for(regime: CacheRegime) -> Duration {
    Duration::from_secs(match regime {
        CacheRegime::Normal   => 3_600,   // 1 hour
        CacheRegime::Calm     => 7_200,   // 2 hours
        CacheRegime::Volatile =>   900,   // 15 minutes
        CacheRegime::Crisis   =>   300,   // 5 minutes
    })
}
```

Higher volatility means shorter TTLs -- the cache expires faster to avoid serving stale
responses during active incidents.

### CachedResponse

```rust
pub struct CachedResponse {
    pub body: Bytes,              // serialized InferenceResponse
    pub cost_usd: f64,            // cost of the original provider call
    pub model: String,            // model that served the original
    pub cached_at: Instant,       // insertion time
    pub effective_ttl: Duration,  // regime-aware lifetime
}
```

Expired entries are lazily evicted on lookup.

---

## 3. L2: Semantic Cache

### SimHash Algorithm

SimHash produces a 64-bit fingerprint that preserves semantic similarity -- similar
texts produce fingerprints with low Hamming distance.

```rust
pub fn simhash(text: &str) -> u64 {
    let mut weights = [0_i32; 64];
    for token in text
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|t| !t.is_empty())
        .map(str::to_ascii_lowercase)
    {
        let hash = DefaultHasher::hash(&token);  // 64-bit
        for bit in 0..64 {
            if hash & (1u64 << bit) == 0 {
                weights[bit] -= 1;
            } else {
                weights[bit] += 1;
            }
        }
    }
    // Positive weights -> 1, non-positive -> 0
    weights.iter().enumerate().fold(0u64, |h, (bit, w)| {
        if *w > 0 { h | (1u64 << bit) } else { h }
    })
}
```

### Hamming Distance

```rust
pub const fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}
```

A distance of **3 bits or fewer** counts as a semantic cache hit. This threshold is
conservative enough to avoid false positives while catching near-miss requests.

### Storage

```rust
pub struct SimHashEntry {
    pub response: Bytes,       // serialized response
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,     // tenant isolation key
}
```

Storage: `DashMap<u64, SimHashEntry>` for lock-free concurrent reads.
Default capacity: **5,000 entries**. Fixed TTL: **7,200 seconds** (2 hours). Eviction:
by age when capacity is reached (remove the oldest entry).

### Namespace Isolation

Each request carries a `namespace` field (default: `"default"`). The L2 cache only
returns hits from the **same namespace and same model**. This prevents cross-tenant
cache hits in multi-user deployments. Single-user setups use the `"default"` namespace
transparently.

### Tool-State Fingerprint

`InferenceRequest::semantic_text()` appends a truncated hash of tool names when tools
are present:

```rust
if let Some(tools) = &self.tools && !tools.is_empty() {
    let mut hasher = DefaultHasher::new();
    for tool in tools { tool.name.hash(&mut hasher); }
    text.push_str(&format!("\n__tools:{:016x}", hasher.finish()));
}
```

This ensures that requests with different tool states (e.g., after tool pruning) do not
collide in the semantic cache.

---

## 4. Cache Exclusion Policy

Neither layer caches responses that match any of:

| Condition | Reason |
|-----------|--------|
| `stop_reason == ToolUse` | Tool call IDs are ephemeral; cached tool-use responses would reference stale IDs |
| `output_tokens < 3` | Too short to be useful; likely an error or empty response |
| Serialization failure | Cannot store the response body |

Excluded responses increment the `excluded` counter in `CacheStats`.

---

## 5. Lookup Order

```
Request arrives
    |
    v
1. Compute exact_key = blake3(normalize(request))
2. Check L1 HashMap
    |
    +--> Hit + not expired --> return CacheHit { layer: L1 }
    +--> Hit + expired     --> evict, fall through
    +--> Miss              --> fall through
    |
    v
3. Compute fingerprint = simhash(request.semantic_text())
4. Scan L2 DashMap for entries with:
   - Same namespace
   - Same model
   - hamming_distance(fingerprint, stored) <= 3
   - Not expired (TTL check)
   - Best (lowest) distance if multiple candidates
    |
    +--> Found --> return CacheHit { layer: L2 }
    +--> None  --> return None (miss)
```

L1 is checked first because it is cheaper (hash lookup vs. linear scan) and produces
exact matches. L2 is a fallback for near-miss queries.

---

## 6. Store Operation

Both layers are written simultaneously after a successful provider response:

1. **L1:** Insert `CachedResponse` at `exact_key`. If already present, update in place
   and refresh recency. Evict oldest if capacity exceeded.
2. **L2:** Insert `SimHashEntry` at the request's SimHash fingerprint. Evict oldest
   by `created_at` if capacity exceeded.

Both writes happen after cost computation so that the stored entry carries the correct
`cost_usd` for avoided-cost reporting on future cache hits.

---

## 7. Telemetry

`CacheStats` exposes four counters:

```rust
pub struct CacheStats {
    pub l1_hits: u64,   // exact hits
    pub l2_hits: u64,   // semantic hits
    pub misses: u64,    // both layers missed
    pub excluded: u64,  // responses rejected by cache policy
}
```

The `InferenceCache::sizes()` method returns current `(l1_len, l2_len)` for monitoring
bounded growth.
