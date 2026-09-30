+++
id = "gap-93fdf4"
kind = "gap"
title = "26-HTTP-API says serve auth is off by default, though serve.auth.enabled defaults to true, and docs/v1 and docs/v2 still put secrets in roko.toml"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs/v1", "docs/v2", "docs/v3"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4cf2e329b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["docs/v3/26-HTTP-API.md", "crates/roko-core/src/config/serve.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ed511d", "bug-524a3b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqE 'api_key *= *\"' docs/v1 docs/v2 && grep -qiE 'on by default|enabled by default' docs/v3/26-HTTP-API.md"
+++

## Problem

- `docs/v3/26-HTTP-API.md` presents API-key authentication as opt-in ("# Enable API key authentication", around :95). But `ServeAuthConfig::default()` enables it (`crates/roko-core/src/config/serve.rs:278-285`), and `roko serve` mints a key.
- docs/v1 and docs/v2 still show secrets in `roko.toml`, for example `docs/v1/INTEGRATION-GUIDE.md`, `docs/v1/12-interfaces/05-http-api-roko-serve.md` and `docs/v1/19-deployment/10-secret-management.md`. roko now refuses those. gap-ed511d covers docs/v3 and RAILWAY.md.

## Why it matters

Release: the docs describe an insecure default that isn't the real one, and old docs lead users to configs roko refuses. p3.

## Plan

1. Say in 26-HTTP-API §3 that auth is on by default, and how to turn it off for local use.
2. Add a deprecation banner to the docs/v1 and v2 pages that put secrets in `roko.toml`, pointing to `.roko/.env`, or fix their examples.

## Done when

- [ ] Both are true.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise held at 4cf2e329b; also, 26-HTTP-API's quick start used `roko serve --api-key`, a flag `roko serve` does not have.
- Changes: 26-HTTP-API section 3 says auth is on by default, with the launch token `roko serve` prints on a loopback address, how to set a key and how to turn auth off; its diagram and config example agree. docs/v1 and docs/v2 examples no longer show `api_key`, a webhook secret, the chain wallet key or the Railway token as literals: each points at its `ROKO__` variable in `.roko/.env`, provider keys use `api_key_env`, and v2 gives `serve.auth.enabled`'s default as true. The `ServeAuthConfig::enabled` doc notes the default.
- To check: `enabled` has a field-level `#[serde(default)]`, so a `[serve.auth]` table that leaves it out probably reads auth as off, unlike the struct default.
- The `[[verify]]` (greps) passes. Implemented on `work/bug-7830f5` at `8e7cebc97`; the `serve.rs` doc comment waits for the batch cargo check.
