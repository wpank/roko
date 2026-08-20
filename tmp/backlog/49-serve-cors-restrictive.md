# 49 — Serve CORS Restrictive

**Priority**: P1 — security: `unsafe_public_cors` enables credential-bearing cross-origin requests from any site
**Size**: S (half day)
**Crates**: `crates/roko-serve` (`src/routes/middleware.rs`, `src/lib.rs`, `src/routes/mod.rs`)
**Depends on**: None

---

## Background

The HTTP control plane has a three-tier CORS policy implemented in `cors_layer()`:

1. **Explicit origins**: if `server.cors_origins` is non-empty, only those origins are allowed.
2. **Wildcard**: if `server.unsafe_public_cors = true` and no explicit origins are set, `CorsLayer::permissive()` is returned.
3. **Default (local-only)**: otherwise, a predicate allows only loopback origins on any port.

The third tier is correct. The first tier is correct. The second tier has a problem: `CorsLayer::permissive()` from `tower-http` sets `Access-Control-Allow-Credentials: true` in addition to `Access-Control-Allow-Origin: *`. The combination of a wildcard origin with credentials enabled means that any website can make authenticated cross-origin requests to the roko API using the user's stored session cookies or credentials. This is a classic CSRF vector.

Additionally, the `allowed_cors_headers()` list is missing three headers that are used by the demo frontend and CLI HTTP client: `accept`, `x-request-id`, and `x-roko-session`. Missing headers cause preflight failures for legitimate clients.

Finally, `cors_layer()` is called from two separate code paths in `lib.rs` (lines 431/434 in `start_background` and 888/891 in `run_server_with_state`), and a third call is in `routes/mod.rs` line 249 inside `build_router`. These share the same parameters but the duplication makes it easy for a future change to diverge.

## Current State

1. `cors_layer()` is defined at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs:1418`. It takes `cors_origins: &[String]` and `unsafe_public: bool`. When `unsafe_public` is true and origins is empty, it returns `CorsLayer::permissive()` at line 1435.

2. `allowed_cors_methods()` at line 1391 returns 6 methods: `[GET, POST, PUT, DELETE, PATCH, OPTIONS]`. This is correct.

3. `allowed_cors_headers()` at line 1407 returns 5 headers: `[CONTENT_TYPE, AUTHORIZATION, x-api-key, x-user-id, x-user-email]`. Missing: `ACCEPT`, `x-request-id`, `x-roko-session`.

4. `cors_layer()` is exported from `routes/mod.rs` at line 233 (`pub(crate) use self::middleware::cors_layer`) and called:
   - In `build_router()` at `routes/mod.rs:249`
   - In `build_server_router()` at `lib.rs:955`
   - `build_server_router()` is called from `ServerBuilder::start_background()` at line 431 and from `run_server_with_state()` at line 888

5. `cors_layer()` does not receive `auth.enabled` — it has no way to gate the `unsafe_public_cors` branch on auth being enabled.

6. Tests for CORS are at `middleware.rs:2810-2894` (labeled "T55"). They cover the default local-only path and the `unsafe_public_cors` wildcard path, but do not test the auth-gating behavior requested here because that logic does not currently exist.

7. `UNSAFE_PUBLIC_CORS_WARNING` (a `std::sync::OnceLock<()>` deduplicated tracing warning) is used at line 1429 to avoid spamming the log on every request. The warning text at line 1431 is: `"CORS is unrestricted (allow *) because server.unsafe_public_cors = true and no cors_origins are configured."`.

## Implementation Plan

**Step 1: Add `ACCEPT`, `x-request-id`, and `x-roko-session` to `allowed_cors_headers()`**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs`, change `allowed_cors_headers()` at line 1407 from a fixed-size array of 5 to 8:

```rust
fn allowed_cors_headers() -> [HeaderName; 8] {
    [
        CONTENT_TYPE,
        AUTHORIZATION,
        ACCEPT,
        HeaderName::from_static("x-api-key"),
        HeaderName::from_static("x-user-id"),
        HeaderName::from_static("x-user-email"),
        HeaderName::from_static("x-request-id"),
        HeaderName::from_static("x-roko-session"),
    ]
}
```

Add `ACCEPT` to the imports at the top of the file alongside `AUTHORIZATION` and `CONTENT_TYPE`.

**Step 2: Replace `CorsLayer::permissive()` with a wildcard without credentials**

`CorsLayer::permissive()` is a shorthand from `tower-http` that enables `Allow-Credentials: true`. Replace it with an explicit construction that does NOT set credentials:

```rust
if unsafe_public {
    // Log a deduplication warning.
    if UNSAFE_PUBLIC_CORS_WARNING.set(()).is_ok() {
        tracing::warn!(
            "CORS is unrestricted (allow *) because server.unsafe_public_cors = true and no \
             cors_origins are configured. Set cors_origins to limit access."
        );
    }
    return CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(allowed_cors_methods())
        .allow_headers(allowed_cors_headers());
    // NOTE: allow_credentials is NOT set — wildcard origin + credentials is
    // rejected by browsers and enables CSRF.
}
```

**Step 3: Gate `unsafe_public_cors` on auth**

Change the signature of `cors_layer()` to accept `auth_enabled: bool`:

```rust
pub fn cors_layer(cors_origins: &[String], unsafe_public: bool, auth_enabled: bool) -> CorsLayer {
```

In the `unsafe_public` branch, add an auth check before returning the wildcard layer:

```rust
if unsafe_public {
    if !auth_enabled {
        tracing::warn!(
            "server.unsafe_public_cors = true is ignored because serve.auth.enabled = false. \
             Enable auth to use wildcard CORS. Falling through to local-only CORS policy."
        );
    } else {
        // Return wildcard-without-credentials (see Step 2 above).
        ...
    }
}
```

When `unsafe_public` is true but auth is off, fall through to the default local-only policy instead of returning the permissive layer.

**Step 4: Extract a `CorsPolicy` struct**

Create a small struct to group the three CORS-related parameters so the call sites cannot diverge:

```rust
pub(crate) struct CorsPolicy {
    pub cors_origins: Vec<String>,
    pub unsafe_public_cors: bool,
    pub auth_enabled: bool,
}
```

Change `cors_layer()` to accept `&CorsPolicy` instead of three separate arguments. Update all call sites:

- `build_router()` at `routes/mod.rs:249`
- `build_server_router()` at `lib.rs:955`
- `build_server_router()`'s callers at `lib.rs:431` and `lib.rs:888`
- The test helpers at `middleware.rs:2699`, `2815`, `2823`

**Step 5: Update CORS tests**

The existing tests at `middleware.rs:2810-2894` call `cors_layer(&[], false)` and `cors_layer(&[], true)`. Update them to pass a `CorsPolicy` struct. Add new tests:

- `unsafe_public_cors` with `auth_enabled: false` returns local-only policy (not wildcard)
- `unsafe_public_cors` with `auth_enabled: true` returns wildcard without `Access-Control-Allow-Credentials: true`
- Preflight request with `accept` header is allowed by the default policy

## Acceptance Criteria

1. `unsafe_public_cors = true` with `auth.enabled = false` logs a warning and falls through to local-only CORS — the response does not include `Access-Control-Allow-Origin: *`.
2. `unsafe_public_cors = true` with auth enabled returns `Access-Control-Allow-Origin: *` but does NOT include `Access-Control-Allow-Credentials: true`.
3. `allowed_cors_headers` includes `accept`, `x-request-id`, and `x-roko-session`.
4. Both `lib.rs` call sites use the same `CorsPolicy` struct — there is only one place to update CORS parameters.
5. Existing CORS tests in `middleware.rs` updated and passing.
6. New tests cover the auth-gated wildcard behavior.
7. Manual verification: a `curl` preflight from `localhost:5173` to `localhost:6677` with an `Accept` header succeeds with the default config.

## Verification Checklist

- [ ] Start `roko serve` with `unsafe_public_cors = true` and `auth.enabled = false`; verify log warning fires and a preflight from a non-local origin gets no `Access-Control-Allow-Origin` header
- [ ] Start `roko serve` with `unsafe_public_cors = true` and auth enabled; verify `Access-Control-Allow-Origin: *` is present and `Access-Control-Allow-Credentials` is absent
- [ ] Verify a preflight with an `Accept` header is reflected in `Access-Control-Allow-Headers` under the default policy
- [ ] Run `cargo test -p roko-serve` — all tests pass
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs` | Add `ACCEPT`, `x-request-id`, `x-roko-session` to `allowed_cors_headers()` (line 1407); replace `CorsLayer::permissive()` with explicit wildcard-without-credentials (line 1435); add `auth_enabled` parameter; change to accept `CorsPolicy`; update tests at lines 2699, 2815, 2823 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` | Update `build_server_router()` signature (line 942) and both call sites (lines 431, 888) to construct and pass `CorsPolicy` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` | Update `build_router()` call to `cors_layer()` at line 249 to pass `CorsPolicy`; define `CorsPolicy` struct near the `cors_layer` re-export at line 233 |
