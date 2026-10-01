+++
id = "bug-7bf8d8"
kind = "bug"
title = "roko acp config options use type toggle; the ACP v1 schema has boolean"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-426c9d"
anchors = ["crates/roko-acp/src/handler.rs", "crates/roko-acp/src/session.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-426c9d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib config_option_spec_conformance"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:18Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:50:53Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
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
