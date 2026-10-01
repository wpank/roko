+++
id = "bug-39d15f"
kind = "bug"
title = "roko-gateway still prices an unknown model at Sonnet rates"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-gateway"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ad0d39"
anchors = ["crates/roko-gateway/src/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'sonnet_fallback' crates/roko-gateway/src --include=*.rs"
+++

## Problem

gap-ad0d39 made roko-learn leave unknown models unpriced. The gateway still prices them with `sonnet_fallback`, so gateway cost events disagree with the cost log.

## Plan

Leave unknown models unpriced in the gateway too (cost unknown, warn once), as roko-learn does.

## Done when

- The verify passes, and the gateway's cost tests are updated.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on gap-ad0d39, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `CostTracker::compute_cost` returns an all-zero `CostResult` for a model the table does not price and logs it
  once through roko-learn's `warn_unpriced_model` (now `pub`, so the two log a model once between them);
  `sonnet_fallback` is gone. Test: `cost_track_leaves_an_unknown_model_unpriced` (was
  `cost_track_unknown_model_uses_sonnet_fallback`); docs/v3's gateway cost pages say so. Consequence: the
  gateway's budget preflight can no longer price, so no longer blocks, a call to an unpriced model.
