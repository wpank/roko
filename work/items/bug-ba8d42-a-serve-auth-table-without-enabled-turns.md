+++
id = "bug-ba8d42"
kind = "bug"
title = "A [serve.auth] table without `enabled` turns serve auth off: the field's serde default is false while the struct default is true"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-core/config", "roko-serve/auth"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "669fc7274"
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

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-ba8d42` at `669fc7274`: `ServeAuthConfig.enabled` is `#[serde(default = "default_true")]`, so auth stays on unless a config says `enabled = false`. Test `config::loader::tests::serve_auth_table_without_enabled_keeps_auth_on` covers serde alone, a `.roko/config.toml` key file holding only `api_key`, three partial `roko.toml` tables (an `api_key` reference, `privy_app_id`, `enforcement_mode`), `ROKO__SERVE__AUTH__API_KEY` with no table and over a partial table (a real process-env run through `load_config_file`), and `enabled = false`.
- 2026-09-30 (wk-serve-sec): Cargo, in the worktree's own APFS clone of `roko-batch-target` (cloned once `GATE-BUSY` was absent, 32 GB free, `CARGO_BUILD_JOBS=4`, `CARGO_INCREMENTAL=0`; deleted afterwards). `cargo test -p roko-core --lib serve_auth_table_without_enabled_keeps_auth_on` passes (118 s including the rebuild). Negative control: with the old bare `#[serde(default)]` the same test fails at the serde assertion (a table with only `api_key`). `cargo test -p roko-core --lib`: 1954 passed, 0 failed (187 s). The env-only key was already safe before the fix, because the hierarchical overlay serializes the loaded config, `enabled` included, before setting the key. The env key over a partial table was not.
- 2026-09-30 (wk-serve-sec): Scan of every config struct with a manual `Default` (roko-core `config/` and roko-cli `config.rs`) for bare `#[serde(default)]` fields whose `Default` value differs: no other security-relevant boolean. `GatesConfig.mode` and `diff_scope` agree with their enum defaults. Two non-security mismatches remain, not fixed here: `DeployConfig.worker_image` (serde `None`, `Default` `Some("ghcr.io/nunchi-trade/roko-worker:latest")`) and roko-cli `Config.gates` (serde empty, `Default` one `shell:true` gate).
- 2026-09-30 (wk-serve-sec): Merged the working branch (`705c0da0f`) in `4d936e77e`. The one conflict, the `enabled` field in `serve.rs`, was resolved keeping both sides: guard2's "On by default" doc and this branch's `default_true`. Re-run on the merged tree in a fresh clone (taken once `GATE-BUSY` cleared, `CARGO_BUILD_JOBS=4`, deleted afterwards): `cargo test -p roko-core --lib serve_auth_table_without_enabled_keeps_auth_on` passes, and `cargo test -p roko-core --lib config::` gives 392 passed, 0 failed.
