+++
id = "bug-ba8d42"
kind = "bug"
title = "A [serve.auth] table without `enabled` turns serve auth off: the field's serde default is false while the struct default is true"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-core/config", "roko-serve/auth"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-guard2 report (2026-09-30); confirmed by the coordinator at 7490cb94b"
anchors = ["crates/roko-core/src/config/serve.rs::ServeAuthConfig"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-524a3b", "bug-8f8704", "gap-e9660f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_auth_table_without_enabled_keeps_auth_on' crates/roko-core/src/ && cargo test -p roko-core --lib serve_auth_table_without_enabled_keeps_auth_on"
+++

## Problem

`ServeAuthConfig` (`crates/roko-core/src/config/serve.rs`) declares `#[serde(default)] pub enabled: bool`, and its `impl Default` sets `enabled: true` (secure by default). serde's field-level `default` uses `bool::default()`, which is false. So a `[serve.auth]` table that sets anything without `enabled`, for example only `api_key`, `privy_app_id` or `jwks_providers`, deserializes with auth off.

This matters more now. bug-524a3b and gap-e9660f moved the API key to `ROKO__SERVE__AUTH__API_KEY` in `.roko/.env`. If the env overlay builds a `[serve.auth]` table that holds only that key, then setting the key the recommended way switches auth off.

## Why it matters

Release goal, security: `roko serve` could run unauthenticated in exactly the setup the docs now recommend. p1.

## Where

`crates/roko-core/src/config/serve.rs::ServeAuthConfig` (the `enabled` field and `Default`), plus the config loader's env overlay.

## Current state

Confirmed by reading the code at 7490cb94b. Not yet shown by a test.

## Plan

1. Give `enabled` a field default of true (`#[serde(default = "default_true")]`), so a partial table keeps auth on unless it says `enabled = false`.
2. Tests (`serve_auth_table_without_enabled_keeps_auth_on`):
   - a `[serve.auth]` table with only `api_key` → enabled;
   - only the `ROKO__SERVE__AUTH__API_KEY` env var set, no table → enabled;
   - `enabled = false` → disabled.
3. Check the other config structs for the same field-default-versus-struct-default mismatch on security-relevant booleans, and list any found in the Notes.

## Done when

- [ ] A partial `[serve.auth]` table, and the env-only key, both keep auth on.
- [ ] The `[[verify]]` command passes.
