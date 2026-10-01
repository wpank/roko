+++
id = "bug-2fdfbd"
kind = "bug"
title = "roko acp may fail to parse spec embedded-resource and resource_link prompt blocks"
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
anchors = ["crates/roko-acp/src/types.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-426c9d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib prompt_resource_blocks_parse"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:50:43Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

Inbound prompt `resource` blocks deserialize into a `ResourceRef` that expects `{type:"file",uri}`. The spec's EmbeddedResource is `{uri, text|blob, mimeType}`, and `resource_link` blocks are not handled at all, so spec clients' embedded context probably fails to parse. Unverified.

## Plan

Accept the spec's EmbeddedResource and resource_link shapes, and test them against the vendored schema. Add a test named `prompt_resource_blocks_parse_*`.

## Done when

- `cargo test -p roko-acp --lib prompt_resource_blocks_parse` passes.

## Notes

- Reported on 2026-10-01 by wk-specq, working on bug-426c9d, during the evening close-out round.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  Confirmed: an embedded resource failed to parse, which made the whole `session/prompt` invalid, even though roko
  advertises `embeddedContext`. A `resource_link` became `Unknown`, which `unsupported_prompt_content` refuses,
  though agents must accept links. `ResourceRef` now also has `Text` and `Blob` (manual serde; roko's
  `{"type":"file"}` form still parses), and `ContentBlock::ResourceLink` exists. File links and embedded text feed the
  prompt context on both context paths; links also appear in the prompt text. Tests: `prompt_resource_blocks_parse_*`
  (the samples are validated against the vendored schema).
