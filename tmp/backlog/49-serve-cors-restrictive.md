# Serve CORS Restrictive

**Priority**: P1 — security
**Size**: S (half day)
**Crate**: `crates/roko-serve/src/routes/middleware.rs`, `crates/roko-serve/src/lib.rs`

---

## Problem

The HTTP control plane has a three-tier CORS policy: explicit origins, `unsafe_public_cors`
wildcard, and a default local-only predicate. The default is already restrictive
(localhost-only) as of T3-28/T55, but several residual issues remain:

1. The `unsafe_public_cors` flag bypasses all CORS restrictions with
   `CorsLayer::permissive()`, which also sets `Access-Control-Allow-Credentials: true`.
   This enables CSRF attacks against the roko API from any origin when the flag is set.
   There is no corresponding auth requirement — a user can set `unsafe_public_cors = true`
   with auth off and get a fully open CORS policy on all routes.
2. The `allowed_cors_headers` list includes `x-user-id` and `x-user-email` but not
   `accept` or `x-request-id`, both of which are used by client code. Missing allowed
   headers cause preflight failures for legitimate clients.
3. Two separate call sites in `lib.rs` (lines 434, 891) call `cors_layer()` through
   `build_server_router()`, and a third codepath exists in `run_server_with_state()`.
   These share parameters but the duplication makes it easy for a future change to
   diverge.

---

## Section A: Current State

**A1.** `cors_layer()` at `crates/roko-serve/src/routes/middleware.rs` line 1418 has
three branches: explicit origins (uses `allowed_cors_methods` + `allowed_cors_headers`),
`unsafe_public_cors` (uses `CorsLayer::permissive()`), and default (uses a
local-origin predicate with explicit methods/headers). Tests at lines 2810-2893 cover
the default and unsafe paths.

**A2.** `allowed_cors_methods()` at line 1391 already returns an explicit 6-method list:
`[GET, POST, PUT, DELETE, PATCH, OPTIONS]`. This is correct.

**A3.** `allowed_cors_headers()` at line 1407 returns 5 headers:
`[CONTENT_TYPE, AUTHORIZATION, x-api-key, x-user-id, x-user-email]`. Missing:
`ACCEPT`, `x-request-id`, `x-roko-session`.

**A4.** `build_server_router()` at `crates/roko-serve/src/lib.rs` line 942 takes
`cors_origins` and `unsafe_public_cors` and calls `cors_layer()` directly. It is
called from two sites: `ServerBuilder::start_background` (line 434) and
`run_server_with_state` (line 891). Both pass the same fields from `RokoConfig`.

**A5.** `unsafe_public_cors` has no guard requiring auth. A config with
`unsafe_public_cors = true` and `auth.enabled = false` on a public bind is permitted
if `acknowledge_public_risk` is also set.

---

## Section B: What To Do

**B1.** Gate `unsafe_public_cors` on auth: if `unsafe_public_cors = true` and
`auth.enabled = false`, log a `tracing::warn!` and fall through to the default
local-only policy instead of returning `CorsLayer::permissive()`. This prevents
the combination of open CORS + no auth.

**B2.** When `unsafe_public_cors = true` and auth is enabled, replace
`CorsLayer::permissive()` with an explicit wildcard origin that does **not** set
`Access-Control-Allow-Credentials: true`. Use
`AllowOrigin::any()` + `allowed_cors_methods()` + `allowed_cors_headers()`. The
`permissive()` shorthand sets credentials to `true`, which is both unnecessary and
enables credential-bearing cross-origin requests from any site.

**B3.** Add `ACCEPT`, `x-request-id`, and `x-roko-session` to `allowed_cors_headers()`.
These are used by the demo frontend and the CLI HTTP client.

**B4.** Extract the `cors_origins` + `unsafe_public_cors` + `auth.enabled` triplet
into a small struct (e.g., `CorsPolicy`) so the two call sites in `lib.rs` cannot
diverge. The call sites should construct a `CorsPolicy` once and pass it to
`cors_layer`.

---

## Acceptance criteria

- [ ] `unsafe_public_cors = true` with `auth.enabled = false` logs a warning and falls through to local-only CORS
- [ ] `unsafe_public_cors = true` with auth enabled uses wildcard origin without `Access-Control-Allow-Credentials: true`
- [ ] `allowed_cors_headers` includes `accept`, `x-request-id`, and `x-roko-session`
- [ ] Both `lib.rs` call sites use the same `CorsPolicy` struct
- [ ] Existing CORS tests in `middleware.rs` updated and passing
- [ ] Manual verification: demo frontend on `localhost:5173` can make preflight requests to `localhost:6677` with the default config

### Not in scope
- Adding CORS preflight caching (`Access-Control-Max-Age`)
- Per-route CORS overrides
- Removing the `unsafe_public_cors` config key entirely
