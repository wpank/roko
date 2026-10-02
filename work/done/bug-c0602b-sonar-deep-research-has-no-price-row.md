+++
id = "bug-c0602b"
kind = "bug"
title = "sonar-deep-research has no price row and is priced as sonar"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "40d55e0b7"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-2dfd23"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-2dfd23"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn sonar_deep_research_price' crates/roko-core/src/ && cargo test -p roko-core --lib sonar_deep_research_price"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:17:33Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:25Z"
forced = false
evidence = "Gate 6f on 9d7b62cbd plus its fixes, re-checked at 41c59176b and merged as 40d55e0b7 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3425, roko-core 1985, roko-learn 1233, roko-gate 700); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_plan_callers and smoke pass; bin 445; scripts/test_run_evidence_graph.py 9/9; all 245 --help pages identical to the pre-split binary once the binary name is normalized; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

The pricing registry has no sonar-deep-research row, so the prefix rule prices it at sonar's $1/$1, far below its real price.

## Plan

Add the row from Perplexity's dated price page, or leave it unpriced. Add a test named `sonar_deep_research_price`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-2dfd23, during the evening close-out round.
- 2026-10-02 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check. Decided with the
  coordinator: leave it unpriced. The price page (https://docs.perplexity.ai/docs/getting-started/pricing, checked
  2026-10-02; the page carries no date) gives Sonar Deep Research $2/M input and $8/M output, plus citation tokens
  ($2/M), reasoning tokens ($3/M) and search queries ($5 per 1K), and no request fee. `ModelPricing` cannot express the
  last three, which dominate a deep-research call, so a $2/$8 row would understate it, and an unpriced model now has an
  unknown cost (bug-1f81ab).
- bug-1f81ab's snapshot rule already stopped the registry from pricing it as Sonar. This change records the decision at
  the Perplexity rows in `model_registry.rs` and removes `costs_db`'s partial row, including its guessed $0.005
  request fee, so both tables agree. Test `sonar_deep_research_price` (roko-core); `perplexity_costs` (roko-learn) now
  checks that the row is gone.
