+++
id = "bug-9dd205"
kind = "bug"
title = "Supply-chain audit fails on new rustls advisories and unmaintained unsafe-libyaml"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["ci"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "pr-87-ci-2026-10-19"
anchors = ["Cargo.lock", "deny.toml"]
links = { depends_on = [], blocks = [], related = ["find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo deny --locked check advisories"
+++

## Problem

The "Supply-chain audit" check (`cargo deny check`, `.github/workflows/deny.yml`) fails on PR #87, which integrates
local main through ee4b53c5e. Nothing in the PR caused it; advisories published since the last green run did:

- rustls 0.23.45 has four: RUSTSEC-2026-0346 (`Debug` impls for client session values expose TLS secrets),
  RUSTSEC-2026-0347 (buffered data sent to the outer server after an ECH rejection), RUSTSEC-2026-0348 (Brotli
  certificate decompression can allocate up to 1 GiB before authentication) and RUSTSEC-2026-0349 (an
  unauthenticated ServerHello can downgrade later connections to classical key exchange). The fix is rustls 0.23.46.
- unsafe-libyaml 0.2.11 is unmaintained (RUSTSEC-2024-0449). It has no known vulnerability and no patched release.

## Why it matters

The audit fails on every PR and push until it is fixed, so the release PR can't go green, and the rustls advisories
are real TLS issues for roko's HTTP clients.

## Where

- `Cargo.lock`: rustls 0.23.45.
- `deny.toml` `[advisories] ignore`: reviewed exceptions, each with the path that pulls the crate in.
- unsafe-libyaml comes only through serde_yaml_ng 0.10 in roko-cli, which parses local YAML: an agent contract
  (`crates/roko-cli/src/chat_session.rs`) and plan front matter (`crates/roko-cli/src/orchestrator/plan_discovery.rs`).

## Current state

`cargo deny check advisories` fails with the five errors above (PR #87's run 39227305352).

## Plan

1. `cargo update -p rustls` (0.23.45 to 0.23.46; it changes only that package).
2. Add RUSTSEC-2024-0449 to `deny.toml`'s ignore list, with its reason, as the other unmaintained crates are.

## Done when

`cargo deny --locked check advisories` passes.

## Notes

Related: find-8cc7ac (workflows failing on main). Replacing serde_yaml_ng with a YAML parser that has no
unmaintained dependency would remove the ignore entry; it is not needed for the audit.
