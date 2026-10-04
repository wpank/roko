+++
id = "bug-38a412"
kind = "bug"
title = "roko-agent's chain feature doesn't enable roko-std/chain, so its own tool catalog test fails in isolation"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-agent"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (gate 6b)"
discovered_from = "gate 6b"
anchors = ["crates/roko-agent/Cargo.toml", "crates/roko-agent/tests/provider_parity.rs::catalog_count_matches_expected_default"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-c89168", "gap-425d9e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '^chain = \\[\\]$' crates/roko-agent/Cargo.toml"
+++

## Problem

`cargo test -p roko-agent --features chain --test provider_parity catalog_count_matches_expected_default` fails:
it expects 52 tools in the catalog but gets 35. `crates/roko-agent/Cargo.toml:21` defines `chain = []` — an empty
feature with no propagation — so enabling `roko-agent`'s own `chain` feature does not turn on `roko-std/chain`,
and the 17 chain tool handlers `roko-std/chain` would add to the catalog are absent. Running with
`--features chain,roko-std/chain` explicitly, all tests pass (52 tools).

By contrast, `crates/roko-cli/Cargo.toml:21-27`'s `chain` feature correctly propagates: `chain = ["dep:roko-chain",
"roko-std/chain", "roko-agent/chain", "roko-serve/chain", "roko-agent-server/chain"]`. Before PK79's work
(gap-425d9e) turned `roko-cli`'s `chain` default off, both `roko-std/chain` and `roko-agent/chain` were enabled
together as part of `roko-cli`'s default feature set, which is why this gap in `roko-agent`'s own feature
definition never surfaced as a test failure until a test exercises `roko-agent` with `chain` enabled in isolation.

## Why it matters

A workspace member's own feature flag that doesn't propagate to the dependency it names is a latent trap: anyone
building or testing `roko-agent --features chain` on its own (not through `roko-cli`'s umbrella feature) gets a
catalog that silently lacks the chain tools, with no compile error — only a test (when run) catches it.

## Where

`crates/roko-agent/Cargo.toml:21` (`chain = []`); contrast `crates/roko-cli/Cargo.toml:21-27`'s correctly
propagating `chain` feature; the failing test `crates/roko-agent/tests/provider_parity.rs:559`
(`catalog_count_matches_expected_default`).

## Current state

`cargo test -p roko-agent --features chain --test provider_parity catalog_count_matches_expected_default` fails
(35 vs expected 52) at HEAD. `--features chain,roko-std/chain` passes (confirmed by gate 6b).

## Plan

Change `crates/roko-agent/Cargo.toml:21` from `chain = []` to `chain = ["roko-std/chain"]`, mirroring how
`roko-cli`'s own `chain` feature propagates to its dependencies.

## Done when

- `cargo test -p roko-agent --features chain --test provider_parity catalog_count_matches_expected_default`
  passes without also passing `roko-std/chain` explicitly.
- The `[[verify]]` command passes.

## Notes

- Severity p3 per the reporter (gate 6b): this only bites when `roko-agent` is built/tested with `chain` in
  isolation, not through `roko-cli`'s umbrella feature (the common case).
- Related: `spec-c89168` (open, the broader chain/Alloy feature-gating epic) and `gap-425d9e` (done, PK79 — turned
  `roko-cli`'s `chain` default off, which is what let this gap go unnoticed until a test exercised it directly).

## Progress

- 2026-10-04 (w4-length): implemented on `work/bug-a3f005` at cc52b2729; cargo verification deferred to the
  batch gate. `crates/roko-agent/Cargo.toml` now has `chain = ["roko-std/chain"]`. roko-serve/chain forwards the
  same `roko-std/chain` (plus roko-agent-server/chain); it does not forward roko-agent/chain, which only gates
  roko-agent's own test assertions. roko-cli/chain forwards both. roko-agent has no other chain-gated code. The
  static verify passes.
