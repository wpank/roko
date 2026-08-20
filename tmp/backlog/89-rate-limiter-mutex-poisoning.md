# 89 — Rate Limiter Panics on Poisoned Mutex

**Priority**: P1 — reliability (a single tool handler panic poisons the mutex and makes all subsequent LLM requests panic)
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-agent` (`src/rate_limit.rs`)
**Depends on**: None

---

## Background

The rate limiter in `roko-agent` throttles LLM requests per provider to stay within RPM (requests per minute) and TPM (tokens per minute) budgets. It uses `std::sync::Mutex` to protect shared per-provider state. The limiter is shared across all concurrent tool dispatch calls, typically wrapped in an `Arc<ProviderRateLimiter>`.

`std::sync::Mutex` in Rust uses a "poisoning" mechanism: if a thread panics while holding a lock, the mutex is permanently poisoned. Any subsequent call to `.lock()` returns `Err(PoisonError)`. If `.expect()` is called on this result (as it currently is), the entire thread panics.

Since the rate limiter is shared across all concurrent agent dispatch calls, a single panic inside a tool handler — while the rate limiter lock is held — cascades into every subsequent LLM request panicking too. The agent becomes unusable until restarted.

The fix is mechanical: replace every `.expect()` on a `std::sync::Mutex::lock()` call with `.unwrap_or_else(|poisoned| poisoned.into_inner())`. This recovers the lock guard from the poisoned mutex without re-panicking. The data inside may be in an inconsistent state, but for a rate limiter (which tracks sliding window token counts), the worst case is a slightly inaccurate rate estimate — far better than a complete agent crash.

Note: `parking_lot::Mutex` does not poison (its `.lock()` returns the guard directly, never an error). Before making changes, verify which mutex type is used. The imports at the top of `rate_limit.rs` will show: if `use std::sync::{Arc, Mutex}` is present, these are `std::sync::Mutex` and poisoning applies.

## Current State

1. File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/rate_limit.rs`

2. The file uses `std::sync::Mutex` (imported at line 32: `use std::sync::{Arc, Mutex}`). Poisoning is a real concern.

3. `.expect()` calls on mutex lock results — all in production code paths (not tests):

| Line | Method | Lock variable | Purpose |
|------|--------|--------------|---------|
| 178 | `TpmTracker::add` | `self.buckets.lock()` | `"tpm tracker lock"` |
| 196 | `TpmTracker::current` | `self.buckets.lock()` | `"tpm tracker lock"` |
| 377 | `ProviderRateLimiter::acquire` | `self.providers.lock()` | `"rate limiter lock"` |
| 450 | `ProviderRateLimiter::record_tokens` | `self.providers.lock()` | `"rate limiter lock"` |
| 484 | `ProviderRateLimiter::current_tpm` | `self.providers.lock()` | `"rate limiter lock"` |
| 496 | `ProviderRateLimiter::snapshot` | `self.providers.lock()` | `"rate limiter lock"` |

4. Lines 263 and 279 have `.expect("default RPM must be non-zero")` on `NonZeroU32::new()` results. These are NOT mutex lock operations — they are fallback value constructors called with compile-time constant values. They should be left as-is (they cannot fail at runtime because the constant is non-zero).

5. In the test-only block (line 512+), there are `.unwrap()` calls in test helper code. These are test-only and should also be left as-is.

6. Lines 377-383 (the most critical path, called before every LLM request):
```rust
pub async fn acquire(&self, provider_id: &str) {
    let dedicated = {
        let providers = self.providers.lock().expect("rate limiter lock");
        providers.get(provider_id).map(|state| {
            (Arc::clone(&state.rpm_limiter), Arc::clone(&state.rpm_tracker))
        })
    };
```

7. Lines 449-456 (`record_tokens`, called after every LLM response):
```rust
pub async fn record_tokens(&self, provider_id: &str, tokens: u64) -> u64 {
    let (tracker, tpm_limit) = {
        let providers = self.providers.lock().expect("rate limiter lock");
        ...
    };
```

## Implementation Plan

### Step 1: Replace all mutex `.expect()` calls with poison recovery

For each `.expect()` on a `Mutex::lock()` call (lines 178, 196, 377, 450, 484, 496), replace the pattern:

```rust
let guard = self.field.lock().expect("lock description");
```

with:

```rust
let guard = self.field.lock().unwrap_or_else(|poisoned| {
    tracing::warn!("rate limiter mutex poisoned; recovering");
    poisoned.into_inner()
});
```

Each replacement should use a descriptive log message that identifies which lock was recovered. Use specific messages:

- Line 178: `"tpm tracker buckets mutex poisoned; recovering"`
- Line 196: `"tpm tracker buckets mutex poisoned; recovering"`
- Line 377: `"provider rate limiter mutex poisoned; recovering"`
- Line 450: `"provider rate limiter mutex poisoned; recovering"`
- Line 484: `"provider rate limiter mutex poisoned; recovering"`
- Line 496: `"provider rate limiter mutex poisoned; recovering"`

### Step 2: Leave NonZeroU32 expects unchanged

Lines 263 and 279 use `.expect()` on `NonZeroU32::new(60)` and similar. These are valid: the value `60` is always non-zero, so `NonZeroU32::new(60)` always returns `Some`. Do not change these.

### Step 3: Add a test for poisoned mutex recovery

Add a test that verifies the rate limiter recovers from a poisoned mutex without panicking. The standard way to poison a `std::sync::Mutex` in a test is to spawn a thread that panics while holding the lock:

```rust
#[test]
fn rate_limiter_recovers_from_poisoned_mutex() {
    use std::sync::Arc;

    let limiter = Arc::new(ProviderRateLimiter::new(60));

    // Poison the providers mutex by panicking inside a lock.
    let limiter_clone = Arc::clone(&limiter);
    let _ = std::thread::spawn(move || {
        let _guard = limiter_clone.providers.lock().unwrap();
        panic!("poisoning the mutex deliberately");
    })
    .join(); // Join returns Err because the thread panicked; that's expected.

    // After poisoning, acquire() must not panic.
    // We test the synchronous path: just call current_tpm which locks providers.
    let tpm = limiter.current_tpm("test-provider");
    assert_eq!(tpm, 0, "should recover and return 0 for unknown provider");
}
```

Note: `providers` is a private field. The test would need to be in the `mod tests` block inside `rate_limit.rs` where private fields are accessible. Check line 525 to find where `mod tests` begins.

If `providers` is truly private and cannot be accessed in test code (even within the same file's `mod tests`), use an alternative: create a `ProviderRateLimiter` with real provider config, call `record_tokens` from a panicking thread to poison it, then call `current_tpm` from a second call and verify it does not panic.

## Acceptance Criteria

1. All 6 `.expect()` calls on `Mutex::lock()` in production code are replaced with `.unwrap_or_else(|p| p.into_inner())` with a `tracing::warn!` log.
2. The two `NonZeroU32::new(60).expect(...)` calls on lines 263 and 279 are left unchanged.
3. Test-only `.unwrap()` calls inside `#[cfg(test)]` blocks are left unchanged.
4. A test verifies that `current_tpm` (or `acquire`) does not panic when the mutex has been poisoned.
5. `cargo test -p roko-agent` passes.
6. `cargo clippy -p roko-agent -- -D warnings` passes (the `unwrap_or_else` pattern does not trigger lint warnings).

## Verification Checklist

- [ ] Read `rate_limit.rs` lines 1-40 to confirm `std::sync::Mutex` is used (not `parking_lot::Mutex`)
- [ ] List all `.expect()` calls in the file to confirm exactly which ones are on mutex lock results vs. other operations: `grep -n '\.expect(' crates/roko-agent/src/rate_limit.rs`
- [ ] Replace the 6 mutex lock `.expect()` calls (lines 178, 196, 377, 450, 484, 496)
- [ ] Confirm lines 263 and 279 are NOT mutex locks and are left unchanged
- [ ] Write the poisoned mutex recovery test
- [ ] Run `cargo test -p roko-agent`
- [ ] Run `cargo clippy -p roko-agent -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/rate_limit.rs` | Replace 6 `.expect()` calls on mutex lock results (lines 178, 196, 377, 450, 484, 496) with `.unwrap_or_else(\|p\| p.into_inner())` and `tracing::warn!`; add poisoned mutex recovery test |
