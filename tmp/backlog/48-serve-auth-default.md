# 48 — Serve Auth Default

**Priority**: P1 — security: new workspaces start with auth disabled, hiding the exposure behind a comment
**Size**: S (half day)
**Crates**: `crates/roko-cli` (`src/commands/init.rs`), `crates/roko-serve` (`src/lib.rs`)
**Depends on**: None

---

## Background

The roko HTTP control plane (`roko serve`) exposes approximately 317 API routes including terminal PTY sessions (full shell access), agent dispatch, config modification, and signal injection. `ServeAuthConfig::default()` correctly sets `enabled: true`, so any deployment that does not explicitly configure auth will require an API key.

The problem is that `roko init` — the command new users run first — explicitly writes `serve.auth.enabled = false` into every new workspace's `roko.toml`. This means a new user who runs `roko init && roko serve` has an open, unauthenticated API on localhost. This is noted in a generated comment as a "local development" choice, but:

1. The comment is easy to miss and does not appear in startup logs.
2. If the user later changes `server.bind = "0.0.0.0"` for deployment, the `validate_bind_safety` check will either block them (if they also haven't set `acknowledge_public_risk`) or allow unauthenticated public access if they have.
3. There is no startup banner warning when auth is off on loopback — the existing warning only fires for non-loopback binds.

The fix is small: stop writing `enabled = false` in the init template, add a startup log line when auth is off regardless of bind address, and improve the `acknowledge_public_risk` warning message.

## Current State

1. `ServeAuthConfig::default()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/serve.rs:214` sets `enabled: true`. This is correct and must not change.

2. `render_init_template()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs:15` sets `config.serve.auth.enabled = false` at line 28, then calls `annotate_auth_disabled()` (line 33) which prepends a comment explaining the choice.

3. `annotate_auth_disabled()` at line 140 in `init.rs` searches for the anchor string `[serve.auth]\nenabled = false` and prepends a multi-line comment. If the schema serializer ever changes key ordering, the annotation is silently skipped and the raw `enabled = false` is written without the explanatory comment.

4. `validate_bind_safety()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs:834` returns `Ok(())` immediately for loopback addresses — so no warning fires for the common `roko init && roko serve` case even when auth is off.

5. When `serve.acknowledge_public_risk = true` is set (line 839), the existing `warn!` macro at line 841 says: `"binding to a public address without authentication; all routes will be network-accessible"`. There is no recommendation to enable auth instead.

6. `normalize_serve_dispatch_config()` at line 937 delegates to the core loader's `normalize_and_validate_dispatch_models` — it does not inspect auth state. The Privy auto-enable logic (mentioned in the original backlog) is not in the current code; that path was apparently removed or not implemented.

7. There are no tests that assert the init template writes `serve.auth.enabled = false`. The test coverage for `render_init_template` in `init.rs` is minimal (the function has `#![allow(dead_code)]` at the top, suggesting it may not even be called from production paths currently — verify before changing).

## Implementation Plan

**Step 1: Remove `enabled = false` from the init template**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs`:

Remove lines 28-33:
```rust
config.serve.auth.enabled = false;

let mut rendered = config
    .to_toml_pretty()
    .context("serialize default v2 roko.toml")?;
rendered = annotate_auth_disabled(&rendered);
```

Replace with just:
```rust
let rendered = config
    .to_toml_pretty()
    .context("serialize default v2 roko.toml")?;
```

Also delete the `annotate_auth_disabled()` function at line 140-149 (it is only used by this path).

The generated `roko.toml` will no longer contain a `[serve.auth]` section. The library default (`enabled: true`) takes effect, which means a new workspace requires an API key for `roko serve`. Add a comment to the generated config near the `[serve]` section explaining how to configure auth:
```
# Auth is enabled by default. To disable for local development only:
# [serve.auth]
# enabled = false
```

**Step 2: Add a startup log line when auth is disabled**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs`, locate `run_server_with_state()` (line 861) and `ServerBuilder::start_background()` (line 293). Both paths call `validate_bind_safety()`. Add a check immediately after the bind safety validation:

```rust
if !roko_config.serve.auth.enabled {
    tracing::info!(
        "HTTP auth is disabled — API routes are unauthenticated. \
         Set `serve.auth.api_key` or `[[serve.auth.api_keys]]` to enable."
    );
}
```

This fires for loopback binds with auth off, closing the gap identified in the original issue.

**Step 3: Improve the `acknowledge_public_risk` warning**

In `validate_bind_safety()` at line 839-844, update the `warn!` message to include a concrete recommendation:

```rust
warn!(
    addr = %addr,
    "binding to a public address without authentication; all routes will be \
     network-accessible. Consider enabling auth: set `serve.auth.enabled = true` \
     and provision an API key via `serve.auth.api_key` or `[[serve.auth.api_keys]]` \
     instead of using `acknowledge_public_risk`."
);
```

**Step 4: Add `acknowledge_public_risk` to `roko doctor` output**

Search for the `doctor` command in `crates/roko-cli/src/commands/`. Find where server configuration is checked and add a warning item: if `serve.acknowledge_public_risk = true`, emit a `DoctorItem::warn("serve.acknowledge_public_risk is set — this allows unauthenticated public binds")`.

**Step 5: Update tests**

Search for any test that calls `render_init_template` or asserts the content of the generated `roko.toml`. Update those tests to not expect `enabled = false`. If there are integration tests that run `roko init` and then `roko serve` without providing an API key, they will need to either add an API key or set `auth.enabled = false` explicitly in the test config.

## Acceptance Criteria

1. `roko init` no longer writes `serve.auth.enabled = false` into new workspace `roko.toml` files.
2. Auth is on by default for both new and unconfigured workspaces (covered by the existing `ServeAuthConfig::default()` returning `enabled: true`).
3. When `roko serve` starts with `auth.enabled = false` (any bind address), the startup log includes an info-level message indicating auth is disabled.
4. When `acknowledge_public_risk = true` is used to bypass the public-bind check, the warning log includes a recommendation to enable auth.
5. `roko doctor` reports when `acknowledge_public_risk` is set.
6. `cargo test --workspace` passes.
7. Manual verification: `roko init && roko serve` starts successfully (requires auth provisioning or explicit opt-out in the generated toml).

## Verification Checklist

- [ ] Run `roko init` in a temp directory and check the generated `roko.toml` has no `enabled = false` in the `[serve.auth]` section
- [ ] Run `roko serve` with `auth.enabled = false` and observe the startup info log line
- [ ] Run `roko serve` with `acknowledge_public_risk = true` and observe the improved warning
- [ ] Run `roko doctor` with `acknowledge_public_risk = true` and observe the warning item
- [ ] Run `cargo test --workspace` — all tests pass

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/init.rs` | Remove `config.serve.auth.enabled = false` (line 28), remove `annotate_auth_disabled()` call (line 33) and function definition (lines 140-149); add commented auth section to generated output |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` | Add `tracing::info!` when auth is disabled in `run_server_with_state()` and `ServerBuilder::start_background()`; improve `acknowledge_public_risk` warning in `validate_bind_safety()` (line 841) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/` | Add `acknowledge_public_risk` doctor check in the appropriate doctor command file |
