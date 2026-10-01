+++
id = "bug-0c0747"
kind = "bug"
title = "Other built-in price tables still carry the old Opus and Haiku rates and disagree with BUILTIN_PRICING"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-3de629"
anchors = ["crates/roko-core/src/provider_catalog.rs", "crates/roko-learn/src/costs_db.rs", "crates/roko-compose/src/enrichment/estimate.rs", "crates/roko-agent/src/provider/codex_cli/stream.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-3de629"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib price_tables_agree"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:05Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:29Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

bug-3de629 corrected `BUILTIN_PRICING` against the providers' price pages, with dates. Other tables still carry old rates: `provider_catalog.rs` (Opus $15/$75), the `costs_db.rs` overrides (anthropic/claude-opus-4-6 at $15/$75, and Gemini rows with no cache prices), `estimate.rs`'s fallback ladder (Opus $15/$75, Haiku $0.80/$4) and `codex_cli/stream.rs` (gpt-5.6-sol $2/$0.50/$8). Configured models with no cache prices also get different defaults depending on the path: `CostTable::from_config` and task_runner price cache reads at 0.5x input, while `Usage::fill_cost_from_pricing` uses 0.1x.

## Plan

Derive the other tables from `BUILTIN_PRICING`, or delete them. Use one cache-read default everywhere. Add a test named `price_tables_agree_*`.

## Done when

- `cargo test -p roko-learn --lib price_tables_agree` passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-3de629, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  Each table now derives from `BUILTIN_PRICING` or drops its copy: the provider catalog lists no price for a model
  the registry prices, so `roko config providers add` writes no cost keys for it (it wrote Opus 4.6 at $15/$75 and
  Gemini 2.5 Flash at $0.15/$0.60); costs_db keeps the registry's Gemini 2.5 rows (adding 2.5 Pro's >200K tier) and
  drops `anthropic/claude-opus-4-6`; estimate.rs prices an unregistered Claude slug at its family's row; the Codex
  stream prices turns from the registry. One cache default (roko-core `DEFAULT_CACHE_READ_MULTIPLIER` 0.1x,
  `DEFAULT_CACHE_WRITE_MULTIPLIER` 1.25x) serves `Usage::fill_cost_from_pricing`, the learning `CostTable` and
  task_runner (reads were 0.5x there); the gateway's cost tracker prices cache writes at the table's rate. Anchor
  corrected to roko-compose's estimate.rs. Not changed: `builtin_pricing`'s prefix rule still prices `o3-mini` as
  `o3` and `gemini-2.5-flash-lite` as 2.5 Flash (reported).
