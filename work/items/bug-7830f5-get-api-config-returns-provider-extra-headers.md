+++
id = "bug-7830f5"
kind = "bug"
title = "GET /api/config returns provider extra_headers in the clear"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/config"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4cf2e329b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-serve/src/routes/config.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add", "bug-5a6636"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_route_redacts_provider_extra_headers' crates/roko-serve/src/ && cargo test -p roko-serve --lib config_route_redacts_provider_extra_headers"
+++

## Problem

serve's config route returns each provider's resolved `extra_headers`, values included. A test in `crates/roko-serve/src/routes/config.rs` (:1906-1914) asserts that the effective config holds `authorization = "literal-file-secret"`, a header resolved from a file secret. Header values often carry credentials (`Authorization`, API-key headers).

## Why it matters

Release: any token that can read the config route reads provider credentials.

## Where

The response construction in `routes/config.rs`, and the redaction it applies to other secret fields.

## Plan

1. Redact `extra_headers` values in every config response (or all but a known-safe allowlist), as `roko config show` does.
2. Add `config_route_redacts_provider_extra_headers`.

## Done when

- [ ] No config response contains a header value.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise held at 4cf2e329b: `mask_secret_fields`, which `GET /api/config`, `GET /api/config/toml` and the PUT response use, masked five named fields and a provider `api_key` that no longer exists, not `extra_headers`.
- Change: each non-empty header value shows as `***`, as `roko config show` redacts headers; header names stay, and an empty value stays empty. No allowlist: a header's name does not say whether its value is a credential.
- The static part of the `[[verify]]` passes. Implemented on `work/bug-7830f5` at `cefc346af`; cargo verification deferred to the batch check.
