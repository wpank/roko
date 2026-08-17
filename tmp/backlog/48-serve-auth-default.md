# Serve Auth Default

**Priority**: P1 — security
**Size**: S (half day)
**Crate**: `crates/roko-core/src/config/serve.rs`, `crates/roko-serve/src/lib.rs`

---

## Problem

The roko HTTP control plane (`roko serve` on port 6677) exposes ~317 API routes including
terminal PTY sessions (full shell access), agent dispatch, config modification, and signal
injection. Any gap in the auth-on-by-default posture or the bind-address safety net can
expose these routes to the network.

The default was hardened in T3-25 (auth enabled, PORT env no longer implies `0.0.0.0`),
but several residual gaps remain:

1. `roko init` writes `serve.auth.enabled = false` into new workspaces (the template
   explicitly opts out of the default). A new user who runs `roko init` then `roko serve`
   has an open API on localhost — fine for local dev, but surprising if they later set
   `server.bind = "0.0.0.0"` without realizing auth is off.
2. `validate_bind_safety` allows a public bind with auth off when
   `serve.acknowledge_public_risk = true`. This escape hatch is undocumented outside the
   code comment and has no expiry or rotation requirement.
3. The `normalize_serve_dispatch_config` function auto-enables auth for non-loopback
   binds only when a Privy credential is found. If no credential exists, it silently
   allows the unauthenticated public bind to proceed to `validate_bind_safety`.
4. No startup banner warns when auth is off even on localhost — the warning only fires
   for non-loopback addresses.

---

## Section A: Current State

**A1.** `ServeAuthConfig::default()` at `crates/roko-core/src/config/serve.rs` line 214
sets `enabled: true`. This is correct.

**A2.** The `roko init` template writes `serve.auth.enabled = false` into the generated
`roko.toml`. Search `roko init` / config template / `init_config` in
`crates/roko-cli/src/` to locate the exact template.

**A3.** `resolve_bind_with_port_env` at `crates/roko-serve/src/lib.rs` line 238 correctly
keeps the configured bind address when `PORT` env is set (port only override). Four unit
tests at lines 3972-4035 cover this.

**A4.** `validate_bind_safety` at `crates/roko-serve/src/lib.rs` line 834 permits a
public bind with auth off if `serve.acknowledge_public_risk = true`. There is no
deprecation schedule or rotation check for this flag.

**A5.** `normalize_serve_dispatch_config` at `crates/roko-serve/src/lib.rs` line ~1014
auto-enables auth only when a stored Privy credential is found and the bind is
non-loopback. Without a credential, the code is a no-op.

---

## Section B: What To Do

**B1.** Change the `roko init` template so the generated `roko.toml` does **not** write
`serve.auth.enabled = false`. Instead, comment out the auth section entirely with a
comment like `# Auth is enabled by default. Uncomment to override:` so the default
`true` takes effect.

**B2.** When auth is disabled (regardless of bind address), emit a `tracing::info!` at
startup: `"HTTP auth is disabled — API routes are unauthenticated"`. This catches
the localhost-with-auth-off case that currently produces no feedback.

**B3.** When `acknowledge_public_risk = true` is used to bypass the public-bind safety
check, upgrade the existing `tracing::warn!` to include an explicit recommendation:
`"Consider enabling auth instead of acknowledge_public_risk"`.

**B4.** Document `acknowledge_public_risk` in the config schema's doc comment and add a
note to `roko doctor` output when it is set.

**B5.** Update any tests that depend on the `roko init` template writing
`auth.enabled = false`.

---

## Acceptance criteria

- [ ] `roko init` no longer writes `serve.auth.enabled = false` into new workspaces
- [ ] Auth is on by default for both new and existing (unconfigured) workspaces
- [ ] Startup log line indicates when auth is disabled, on any bind address
- [ ] `acknowledge_public_risk` usage produces a visible recommendation to enable auth
- [ ] `roko doctor` reports when `acknowledge_public_risk` is set
- [ ] Existing tests pass (update any that assume the init template disables auth)
- [ ] Manual verification: `roko init && roko serve` starts with auth enabled

### Not in scope
- Implementing new auth methods or changing the RBAC model
- Adding OAuth/OIDC
- Removing the `acknowledge_public_risk` escape hatch entirely
- Changing the Privy auto-detection behavior
