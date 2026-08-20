# 96 — API Key Comparison Not Timing-Safe in Auth Middleware

**Priority**: P1 — Security: the legacy single-key auth path uses `==` on strings, which leaks timing information that can be exploited to recover the key character-by-character
**Size**: XS (1 hour)
**Crates**: `crates/roko-serve/` (`src/routes/middleware.rs`, `src/routes/team.rs`, `src/routes/webhooks.rs`), `crates/roko-plugin/` (`src/registry.rs`)
**Depends on**: None

---

## Background

The HTTP control plane (`roko serve`) authenticates incoming requests via bearer tokens or `X-Api-Key` headers. There are two auth code paths:

1. **Named API keys** (the modern path): the bearer token is hashed with SHA-256 and the hash is compared against stored hashes using `==` on hex strings. Because the attacker cannot feasibly control the hash output character-by-character, this path has acceptable timing characteristics.

2. **Legacy single `api_key`** (the backward-compatibility path): the bearer token is compared directly against the configured plaintext `api_key` using Rust's `==` operator. Rust's `==` on `str` delegates to `memcmp`, which short-circuits on the first differing byte. An attacker who can make many requests and measure response latency can exploit this to recover the key one character at a time (a timing side-channel attack).

The codebase already has three independent constant-time comparison implementations in `webhooks.rs`, `team.rs`, and `registry.rs`. They differ in subtle ways (one lacks `black_box`, one operates on `str` instead of `[u8]`). The fix is to consolidate into one well-implemented function and use it in the auth middleware.

## Current State

### Vulnerable comparison

`crates/roko-serve/src/routes/middleware.rs`, line 389 (inside `authenticate_api_key()`):
```rust
// 2. Fall back to legacy single api_key for backwards compatibility.
if !auth.api_key.is_empty() && token == auth.api_key {
```

This `token == auth.api_key` comparison is the timing-vulnerable path.

### Hash-based comparison (not vulnerable)

`crates/roko-serve/src/routes/middleware.rs`, line 297 (inside `match_api_key_entry()`):
```rust
if entry.key_hash == token_hash {
```

This compares SHA-256 hex strings. Because the pre-image (the actual key) is not recoverable from the hash character-by-character, this is not timing-sensitive in practice. No change needed here.

### Three existing constant-time implementations

1. **`crates/roko-serve/src/routes/team.rs`, line 602**: Operates on `&str`, uses XOR fold, no `black_box`. Less robust against compiler optimization.
   ```rust
   fn constant_time_eq(left: &str, right: &str) -> bool {
       if left.len() != right.len() { return false; }
       left.as_bytes().iter().zip(right.as_bytes())
           .fold(0_u8, |difference, (a, b)| difference | (*a ^ *b)) == 0
   }
   ```

2. **`crates/roko-serve/src/routes/webhooks.rs`, line 744**: Operates on `&[u8]`, uses XOR accumulation with `core::hint::black_box`. Most robust.
   ```rust
   fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
       if a.len() != b.len() { return false; }
       let mut diff = 0u8;
       for (lhs, rhs) in a.iter().zip(b.iter()) { diff |= lhs ^ rhs; }
       core::hint::black_box(diff) == 0
   }
   ```

3. **`crates/roko-plugin/src/registry.rs`, line 878**: Operates on `&[u8]`, uses XOR fold, no `black_box`.
   ```rust
   fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
       if left.len() != right.len() { return false; }
       left.iter().zip(right)
           .fold(0u8, |difference, (left, right)| difference | (left ^ right)) == 0
   }
   ```

## Implementation Plan

### Step 1: Create a shared `constant_time_eq` function

Add a shared utility in the `roko-serve` crate. The most natural location is a new utility module at `crates/roko-serve/src/util.rs`, or in the existing `middleware.rs` if no utility module exists. Alternatively, if `roko-core` already has a suitable utils module (check `crates/roko-core/src/`), place it there since `roko-plugin` and `roko-serve` both depend on `roko-core`.

For this task, place it in `crates/roko-serve/src/routes/middleware.rs` (since that is the primary fix site) and re-export it for the other callers:

```rust
/// Constant-time byte comparison. Returns `true` iff slices are equal in
/// both length and content.
///
/// Uses `core::hint::black_box` to prevent the compiler from optimizing the
/// XOR accumulation into a short-circuiting branch. This makes the function
/// resistant to timing side-channel attacks where an attacker could recover
/// secret values by measuring response latency.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (lhs, rhs) in a.iter().zip(b.iter()) {
        diff |= lhs ^ rhs;
    }
    core::hint::black_box(diff) == 0
}
```

If placing it in `roko-core`, the function should be `pub` so downstream crates can use it.

### Step 2: Replace the vulnerable comparison in middleware.rs

In `crates/roko-serve/src/routes/middleware.rs`, line 389, change:

```rust
// Before:
if !auth.api_key.is_empty() && token == auth.api_key {

// After:
if !auth.api_key.is_empty()
    && constant_time_eq(token.as_bytes(), auth.api_key.as_bytes())
{
```

### Step 3: Replace the three duplicate implementations

**`crates/roko-serve/src/routes/team.rs`**: Delete the local `constant_time_eq` at line 602 and import from the shared location (e.g. `use crate::routes::middleware::constant_time_eq;` or from `roko_core::util::constant_time_eq`). Update the call site at line 317 to convert `&str` to `&[u8]`:
```rust
// Before:
!invitation.consumed && constant_time_eq(&invitation.invite_token_hash, &supplied_hash)

// After (if both are &str):
!invitation.consumed
    && constant_time_eq(invitation.invite_token_hash.as_bytes(), supplied_hash.as_bytes())
```

**`crates/roko-serve/src/routes/webhooks.rs`**: Delete the local `constant_time_eq` at line 744 and import from the shared location. The existing call sites at lines 657 and 687 already pass `&[u8]`, so no changes needed at the call sites.

**`crates/roko-plugin/src/registry.rs`**: Delete the local `constant_time_eq` at line 878. Import from `roko-core` if placed there; otherwise keep the local copy but use the `black_box` variant. The call site at line 176 already passes `&[u8]`.

**Important**: If `roko-plugin` does not depend on `roko-serve`, you cannot import from `roko-serve`. In that case:
- Place the canonical implementation in `roko-core` (which both crates depend on).
- Or keep a copy in `roko-plugin/src/registry.rs` but update it to use `black_box`.

The minimal-change approach: update `roko-plugin/src/registry.rs` to add `black_box` to the existing implementation, and consolidate the two `roko-serve` copies into one shared location.

### Step 4: Add or update tests

In `crates/roko-serve/src/routes/middleware.rs`, add or confirm existing tests cover both matching and non-matching keys through the legacy path. If no test exists for the `auth.api_key` comparison path, add:

```rust
#[test]
fn legacy_api_key_auth_accepts_correct_key() {
    let config = ServeAuthConfig {
        api_key: "test-secret-key".to_string(),
        ..ServeAuthConfig::default()
    };
    let result = authenticate_api_key("test-secret-key", &config, &[], false);
    assert!(matches!(result, ApiKeyAuthResult::Ok(_, _, _)));
}

#[test]
fn legacy_api_key_auth_rejects_wrong_key() {
    let config = ServeAuthConfig {
        api_key: "test-secret-key".to_string(),
        ..ServeAuthConfig::default()
    };
    let result = authenticate_api_key("wrong-key", &config, &[], false);
    assert!(matches!(result, ApiKeyAuthResult::NoMatch));
}
```

Confirm `constant_time_eq` itself has a unit test:
```rust
#[test]
fn constant_time_eq_basic() {
    assert!(constant_time_eq(b"hello", b"hello"));
    assert!(!constant_time_eq(b"hello", b"world"));
    assert!(!constant_time_eq(b"hello", b"hell"));
    assert!(!constant_time_eq(b"", b"x"));
    assert!(constant_time_eq(b"", b""));
}
```

## Acceptance Criteria

1. The comparison at `middleware.rs` line 389 (`token == auth.api_key`) is replaced with `constant_time_eq(token.as_bytes(), auth.api_key.as_bytes())`.
2. There is exactly one canonical `constant_time_eq` implementation with `core::hint::black_box`, and it is used by all three original callers (middleware, team, webhooks) plus registry.
3. `cargo test -p roko-serve` passes, including tests that verify correct-key acceptance and wrong-key rejection through the legacy path.
4. `cargo test -p roko-plugin` passes.
5. `cargo clippy --workspace --no-deps -- -D warnings` passes.

## Verification Checklist

- [ ] `grep -rn "token == auth.api_key" crates/ --include="*.rs"` returns no results
- [ ] `grep -rn "fn constant_time_eq" crates/ --include="*.rs"` returns exactly the correct number of definitions (1 canonical, possibly 0 or 1 secondary in roko-plugin depending on dependency graph)
- [ ] `cargo test -p roko-serve -- authenticate_api_key` passes
- [ ] `cargo test -p roko-serve -- constant_time_eq` passes
- [ ] `cargo build --workspace` compiles cleanly

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-serve/src/routes/middleware.rs` | Replace `token == auth.api_key` with `constant_time_eq(...)`; add canonical `pub(crate) fn constant_time_eq` with `black_box`; add unit tests |
| `crates/roko-serve/src/routes/team.rs` | Delete local `constant_time_eq`; import and use the shared one; update call site to use `as_bytes()` if needed |
| `crates/roko-serve/src/routes/webhooks.rs` | Delete local `constant_time_eq`; import and use the shared one |
| `crates/roko-plugin/src/registry.rs` | Update local `constant_time_eq` to use `black_box`, or import from shared location if crate dependency allows |
