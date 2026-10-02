+++
id = "bug-e2b31a"
kind = "bug"
title = "costs_db's Perplexity per-request fees mix search-context tiers"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "40d55e0b7"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-1f81ab"
anchors = ["crates/roko-learn/src/costs_db.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1f81ab"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn perplexity_request_fees_use_the_default_low_tier' crates/roko-learn/src/ && cargo test -p roko-learn --lib perplexity_request_fees"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:17:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:25Z"
forced = false
evidence = "Gate 6f on 9d7b62cbd plus its fixes, re-checked at 41c59176b and merged as 40d55e0b7 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3425, roko-core 1985, roko-learn 1233, roko-gate 700); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_plan_callers and smoke pass; bin 445; scripts/test_run_evidence_graph.py 9/9; all 245 --help pages identical to the pre-split binary once the binary name is normalized; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

costs_db's Perplexity per-request fees mix tiers. sonar uses the low fee ($5 per 1K requests) and sonar-pro the high fee ($14 per 1K), while sonar-reasoning-pro's $8 per 1K matches no tier on the price page ($6, $10 or $14).

## Plan

Pick one search-context tier, record which, and price every model at it from the dated page. Add a test named `perplexity_request_fees_*`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-1f81ab, during the evening close-out round.
- 2026-10-02 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check. The tier is the
  default "low" search context size. The price page (https://docs.perplexity.ai/docs/getting-started/pricing, checked
  2026-10-02; the page carries no date) gives fees in "$ per 1,000 requests (varies by search context size)" and says
  "Low is the default": Sonar $5 / $8 / $12, Sonar Pro and Sonar Reasoning Pro $6 / $10 / $14. `costs_db` now charges
  sonar $0.005 (unchanged), sonar-pro $0.006 (was $0.014, the high tier) and sonar-reasoning-pro $0.006 (was $0.008,
  no tier), with the tier recorded beside the rows. sonar-reasoning is no longer on the page and stays unpriced
  (bug-1f81ab), and sonar-deep-research has no request fee (bug-c0602b). Test
  `perplexity_request_fees_use_the_default_low_tier`; `perplexity_costs` follows the new fees.
- Not changed: the shared registry's `sonar` row has a $0.0625/M cache-read price, but the page lists no cache price
  for any Sonar model (checked 2026-10-02). Reported to the coordinator.
