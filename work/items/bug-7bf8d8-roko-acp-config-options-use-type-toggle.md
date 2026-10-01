+++
id = "bug-7bf8d8"
kind = "bug"
title = "roko acp config options use type toggle; the ACP v1 schema has boolean"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-426c9d"
anchors = ["crates/roko-acp/src/handler.rs", "crates/roko-acp/src/session.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-426c9d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib config_option_spec_conformance"
+++

## Problem

roko's config options (`handler.rs`, `session.rs`) use `type: toggle` with a free-form currentValue. The ACP v1 schema has `boolean` with a bool currentValue, so spec clients may reject roko's `config_option_update` and the `session/new` configOptions.

## Plan

Emit the spec shape, and validate it against the vendored ACP v1 schema subset, as bug-426c9d does for session updates. Add a test named `config_option_spec_conformance`.

## Done when

- `cargo test -p roko-acp --lib config_option_spec_conformance` passes.

## Notes

- Reported on 2026-10-01 by wk-specq, working on bug-426c9d, during the evening close-out round.
