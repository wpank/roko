+++
id = "bug-8f8704"
kind = "bug"
title = "${VAR} is expanded only in provider fields, so serve.auth.api_key = \"${X}\" loads as a literal key"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/gap-e9660f at 6820f1c2d)"
anchors = ["crates/roko-core/src/config/schema.rs", "crates/roko-core/src/config/loader.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["gap-e9660f", "bug-524a3b", "gap-ed511d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_auth_api_key_expands_env_references' crates/roko-core/src/ && cargo test -p roko-core --lib serve_auth_api_key_expands_env_references"
+++

## Problem

`RokoConfig::interpolate_env_vars` (`crates/roko-core/src/config/schema.rs:1043`, called from the loader at `loader.rs:760`) expands `${VAR}` only inside `providers`: `api_key_env` and headers (:1052-1068). Anywhere else, a `${VAR}` reference stays literal. With `[serve.auth] api_key = "${ROKO_SERVE_KEY}"`, serve's key is the string `${ROKO_SERVE_KEY}`, and anyone who reads the config knows it.

## Why it matters

Release blockers: now that secrets must stay out of `roko.toml` (gap-e9660f, bug-524a3b), `${VAR}` is the natural way to reference them. It silently does the wrong thing for every non-provider secret.

## Where

`interpolate_env_vars` and `interpolate_env_vars_with`.

## Plan

1. Expand `${VAR}` in every secret-bearing string field (`serve.auth.api_key`, `server.auth_token`, `privy_*`, webhook secrets), or in every string. Fail loudly when a referenced variable is unset.
2. Add `serve_auth_api_key_expands_env_references`.

## Done when

- [ ] `api_key = "${X}"` loads the value of `X`, or fails with a clear error when `X` is unset.
- [ ] The `[[verify]]` command passes.
