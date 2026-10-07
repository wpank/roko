+++
id = "bug-9d77f1"
kind = "bug"
title = "The Sonar registry row has a cache-read price that Perplexity does not publish"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e2b31a"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e2b31a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn sonar_has_no_cache_price' crates/roko-core/src/ && cargo test -p roko-core --lib sonar_has_no_cache_price"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:54Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

The registry's sonar row sets `cache_read_per_m` 0.0625, and a comment says only Sonar has a cache price, but Perplexity's pricing page lists no cache price for any Sonar model.

## Plan

Drop the cache price (or cite where it comes from), fix the comment, and add a test named `sonar_has_no_cache_price`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-cfg, working on bug-e2b31a, during the overnight close-out round.
- 2026-10-02 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check. Source found:
  ae18f6c31 (bug-3de629) took $0.0625/M as Sonar's "published cached-input price", but on Perplexity's site that rate
  belongs to the Agent API's `perplexity/sonar` ($0.25 input, $2.50 output, $0.0625 cache;
  https://docs.perplexity.ai/docs/agent-api/models), another model from the Sonar API's `sonar` ($1/$1). The Sonar
  API price page (https://docs.perplexity.ai/docs/getting-started/pricing, checked 2026-10-02) lists no cache price
  for any Sonar model. So the row now prices a cache read at the input rate, as `sonar-pro` and `sonar-reasoning-pro`
  already did, and the Perplexity comment says so and names the source of the old number.
- Test `sonar_has_no_cache_price` (roko-core) covers all three Sonar rows. roko-learn's
  `cache_read_rates_match_each_providers_price_page` now pins `sonar` at 1.0x.
