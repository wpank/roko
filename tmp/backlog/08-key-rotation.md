# Key Rotation: Lock-Free Provider Credential Ring

**Status:** Backlog
**Priority:** P2
**Size:** S (0 days — already implemented)
**Origin:** `tmp/architecture-archive/07-gateway.md`, Stage 4 "Key rotation"

---

## Problem Statement

Multi-key provider setups are common in production: a team may hold several
Anthropic API keys and wants to distribute load across them, or route around
per-key rate limits without manual intervention. Without automatic rotation,
hitting a 429 on one key stalls all requests until the key cools down, even
when other valid keys are immediately available.

The solution must be:

- **Lock-free**: key reads must not block under concurrent request load.
- **Simple**: no priority queue, no weight-based selection — just round-robin.
- **Safe**: construction must fail if the key list is empty, preventing
  silent "no credentials" failures at call time.

---

## Proposed Solution

A `KeyRing` struct wraps a `Vec<String>` of API keys and an `AtomicUsize`
tracking the active index. On a 429 response, the provider calls `rotate()`,
which increments the index lock-free. The `current()` method computes
`active % keys.len()` so the index wraps naturally without any branch.

```rust
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    pub fn new(keys: Vec<String>) -> GatewayResult<Self>;  // fails if empty
    pub fn current(&self) -> &str;                         // no lock
    pub fn rotate(&self);                                  // no lock
    pub fn active_index(&self) -> usize;                   // diagnostics, no key material
}
```

The `ProviderBackend` trait in `roko-gateway` exposes a `rotate_key` hook:

```rust
pub trait ProviderBackend: Send + Sync {
    fn rotate_key(&self) {}   // default no-op; key-ring backends override
    // ...
}
```

The gateway calls `rotate_key()` automatically when a provider returns a
`RateLimited` failure kind, before attempting the next fallback model.

---

## Implementation Location

**Already fully implemented** in `crates/roko-gateway/src/provider.rs`.

The `KeyRing` struct, its `new` / `current` / `rotate` / `active_index`
methods, and the `ProviderBackend::rotate_key` hook are all present.

The `ModelCallerBackend` (the production adapter that bridges `roko-gateway`
to the existing `roko-agent` provider dispatch) does not carry a `KeyRing`
because credentials in the current architecture are owned by `roko-agent`
provider backends, not by the gateway itself. The gateway's `ProviderBackend`
trait provides the `rotate_key` hook so that future dedicated gateway backends
(e.g., a native Anthropic HTTP client that holds its own keys) can implement
rotation directly.

---

## Acceptance Criteria

All acceptance criteria are already met:

- [x] `KeyRing::new` returns `Err` when the input list is empty.
- [x] `current()` always returns a valid key string without acquiring a lock.
- [x] `rotate()` advances the index without acquiring a lock.
- [x] After N full rotations across K keys, `active_index()` equals `N % K`.
- [x] Concurrent rotation from 8 threads × 1,000 rotations each yields a
  consistent atomic count (8,000) and `current()` returns a valid key.
- [x] `ProviderBackend::rotate_key` is a no-op by default; implementors
  can override it.
- [x] Two unit tests cover: single-key wrap-around, and concurrent rotation
  consistency.

---

## Current State

**This feature is complete.** No implementation work is needed.

The `KeyRing` is ready to be used by any future dedicated gateway provider
backend that owns its own credentials. The current `ModelCallerBackend`
delegates to `roko-agent` providers which manage their own credential
lifecycle — if multi-key rotation is needed for the CLI agent dispatch path,
it would need to be wired into `roko-agent`'s `ProviderRateLimiter` or the
individual provider backends (`AnthropicApiAgent`, etc.) separately.

One minor gap: the gateway does not currently persist the active key index
across process restarts. After a restart, rotation always resets to index 0.
This is harmless for most deployments since all keys in the ring should be
valid, but if key 0 is at its rate limit when the process restarts, it will
receive all traffic until the next 429 triggers rotation.

---

## References

- `crates/roko-gateway/src/provider.rs` — `KeyRing` implementation and unit tests
- `crates/roko-gateway/src/gateway.rs` — `rotate_key` call site in fallback logic
- `tmp/architecture-archive/07-gateway.md` — original design (Section 4)
- `.roko/GAPS.md` — E26 entry: "Inference gateway 12/12 ... rotating keys"
