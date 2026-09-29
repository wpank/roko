+++
id = "bug-28c193"
kind = "bug"
title = "The CLI error hint blames the API key whenever an error message contains \"401\""
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/diagnose-graph-runs 05f8854ce"
anchors = ["crates/roko-cli/src/main.rs::error_hint", "crates/roko-cli/src/main.rs::format_error_with_hint"]
links = { depends_on = [], blocks = [], related = ["bug-a727ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn error_hint_ignores_401_outside_an_http_status' crates/roko-cli/src/main.rs && cargo test -p roko-cli --bin roko error_hint_ignores_401_outside_an_http_status"
+++

## Problem

Every top-level CLI error goes through `format_error_with_hint`, which appends
`hint: check your API key: set ROKO_API_KEY or run roko config set-secret ROKO_API_KEY <key>` whenever the
lower-cased message contains the substring `"401"`. That also fires on errors that name a path, a plan or run id,
a commit hash or hex id, a millisecond timestamp, a byte count or a PID containing those digits (for example a
worktree for item `gap-e4019c`, or `wrote 14015 bytes`). The user is then sent to fix a credential that has
nothing to do with the error.

## Why it matters

A wrong hint sends a person debugging a failed plan run the wrong way. `ROKO_API_KEY` is also the roko-serve key
(`env_registry.rs:977`, "API key for authenticated serve requests"), so even a real provider 401 gets advice
about the wrong key.

## Where

`crates/roko-cli/src/main.rs::error_hint` (:3513). The auth branch is at :3558-3567 (`lower.contains("401") || …`),
and it is called from `format_error_with_hint` (:3504). Tests are in the binary's `tests` module
(`error_hint_401_triggers_api_key`, :8147).

## Current state

Checked at 33e107da1. bug-a727ff (done) fixed the "authoritative" case by checking state-recovery messages first.
The bare `"401"` substring match is unchanged.

## Plan

1. Match 401 only as an HTTP status, for example `\b401\b` next to `http`, `status` or `unauthorized`, and
   treat any other number that contains 401 as unrelated.
2. When the message names a provider, point at that provider's key (`roko config check-secrets`,
   `roko config providers health`). Keep the `ROKO_API_KEY` / `roko login` hint for serve-client errors.
3. Add `error_hint_ignores_401_outside_an_http_status`. It checks that a path such as
   `/tmp/run-1401/checkpoint.json: No such file or directory` gets no API-key hint, and that `HTTP 401` still gets one.

## Done when

- A digit run containing 401 in a path or id no longer produces the API-key hint, and real HTTP 401s still do.
- The `[[verify]]` command passes.
