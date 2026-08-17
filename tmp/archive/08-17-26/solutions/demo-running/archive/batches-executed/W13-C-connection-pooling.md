# W13-C: Connection Pooling Documentation and Observability

**Wave**: 13 -- Speed & Reliability
**IMPROVEMENTS ref**: 2.5
**Priority**: P2 -- documentation and observability improvement (pooling already works)
**Effort**: 15-30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

The IMPROVEMENTS document flags connection pooling as missing -- a new `reqwest::Client`
created per dispatch. However, investigation reveals this is **already solved** in the
codebase:

- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` lines 96-113
  define a process-wide `LazyLock<reqwest::Client>` called `SHARED_HTTP_CLIENT` with
  `pool_max_idle_per_host(10)`, `pool_idle_timeout(90s)`, `tcp_keepalive(30s)`.
- `shared_http_client()` returns a clone of this static client.
- `ReqwestPoster::new()` and `OpenAiCompatLlmBackend` both call `shared_http_client()`.

## Root Cause

No bug exists. The IMPROVEMENTS document was based on an earlier audit that missed the
`SHARED_HTTP_CLIENT` static. The connection pooling is already in place with good defaults.

## What This Batch Does

This batch makes the pooling more visible by adding a User-Agent header (useful for
debugging in provider dashboards) and documenting the pool settings in the doc comment.

### File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs`

#### Change 1: Add User-Agent and improve documentation

**Find this code** (lines 91-113):
```rust
/// Process-wide shared HTTP client with pooled connections.
///
/// A single `reqwest::Client` keeps TCP and TLS connections warm across all
/// provider adapters, avoiding redundant handshakes when new backends are
/// constructed for the same process.
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build shared HTTP client")
});

/// Return the process-wide shared HTTP client.
///
/// All production HTTP posters should use this client so requests can reuse
/// pooled connections instead of paying a fresh TLS handshake per backend.
#[must_use]
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT.clone()
}
```

**Replace with:**
```rust
/// Process-wide shared HTTP client with pooled connections.
///
/// A single `reqwest::Client` keeps TCP and TLS connections warm across all
/// provider adapters, avoiding redundant handshakes when new backends are
/// constructed for the same process.
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        // Connection pool: keep up to 10 idle connections per host.
        // Most plan runs hit 1-2 LLM API hosts, so 10 is generous.
        .pool_max_idle_per_host(10)
        // Drop idle connections after 90s of inactivity.
        .pool_idle_timeout(Duration::from_secs(90))
        // TCP keepalive probes every 30s to detect dead connections early.
        .tcp_keepalive(Duration::from_secs(30))
        // Fail fast on connection attempts (DNS + TCP + TLS).
        .connect_timeout(Duration::from_secs(10))
        // User-Agent for debugging and provider dashboards.
        .user_agent(concat!("roko-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build shared HTTP client")
});

/// Return the process-wide shared HTTP client.
///
/// All production HTTP posters should use this client so requests can reuse
/// pooled connections instead of paying a fresh TLS handshake per backend.
/// The client is configured with:
/// - `pool_max_idle_per_host(10)` -- reuse connections across dispatches
/// - `pool_idle_timeout(90s)` -- drop stale connections
/// - `tcp_keepalive(30s)` -- detect dead peers early
/// - `connect_timeout(10s)` -- fail fast on unreachable hosts
#[must_use]
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT.clone()
}
```

## Verification

```bash
# Compile check
cargo check -p roko-agent 2>&1 | head -10

# Verify the user-agent is set
grep -n "user_agent" crates/roko-agent/src/provider/mod.rs
```

## Agent Prompt

```
You are implementing W13-C: Connection Pooling Documentation and Observability.
This is a small documentation/observability improvement -- the core pooling already works.

## Changes to make (1 file)

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs`, update the
`SHARED_HTTP_CLIENT` static and `shared_http_client()` function.

Find this code (lines 91-113):
```rust
/// Process-wide shared HTTP client with pooled connections.
///
/// A single `reqwest::Client` keeps TCP and TLS connections warm across all
/// provider adapters, avoiding redundant handshakes when new backends are
/// constructed for the same process.
static SHARED_HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build shared HTTP client")
});

/// Return the process-wide shared HTTP client.
///
/// All production HTTP posters should use this client so requests can reuse
/// pooled connections instead of paying a fresh TLS handshake per backend.
#[must_use]
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT.clone()
}
```

Replace with the same code but:
1. Add inline comments explaining each pool setting
2. Add `.user_agent(concat!("roko-agent/", env!("CARGO_PKG_VERSION")))` before `.build()`
3. Expand the `shared_http_client()` doc comment to list the pool settings

No new imports needed. `concat!` and `env!` are built-in macros.

Do NOT run cargo build/test/clippy/fmt -- compilation is deferred.
```

## Commit

This batch is committed with all Wave 13 batches together. Do not commit individually.

## Checklist

- [ ] User-Agent header added to shared HTTP client
- [ ] Inline comments explain pool configuration values
- [ ] Doc comment on `shared_http_client()` documents the pool settings
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed
