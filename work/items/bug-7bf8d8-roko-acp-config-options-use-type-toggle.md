+++
id = "bug-7bf8d8"
kind = "bug"
title = "roko acp config options use type toggle; the ACP v1 schema has boolean"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
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
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  The premise was half right: no production option used `Toggle`, since `build_config_options` sends selects with
  string values, but the variant would have serialized as `toggle`. It is now `ConfigOptionType::Boolean`
  (`boolean`, with `toggle` kept as an alias). `config_option_spec_conformance` validates the `session/new` result,
  the `config_option_update` notification and the `set_config_option` result against the vendored schema, now
  widened and renamed to `tests/fixtures/acp-v1-subset.schema.json`. Not changed: `ConfigOptionValue.ready` is a
  roko field at the root of a spec type; the schema accepts it, though the spec reserves root names.
