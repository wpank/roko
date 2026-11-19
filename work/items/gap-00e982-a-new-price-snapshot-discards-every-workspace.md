+++
id = "gap-00e982"
kind = "gap"
title = "A new price snapshot discards every workspace's learned self-model: the predictor version hashes the snapshot id, so load_or_new sets the state aside instead of re-pricing it"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-learn/self_model"]
created = 2026-10-28
updated = 2026-10-28
last_verified = 2026-10-28
source = "round v7 lead (bb36bc6f), on the gap-ceffb3 merge, 2026-10-28"
anchors = ["crates/roko-learn/src/self_model/model.rs::predictor_version", "crates/roko-learn/src/self_model/model.rs::load_or_new"]
links = { depends_on = [], blocks = [], related = ["gap-ceffb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_new_price_snapshot_keeps_the_learned_state' crates/roko-learn/src && cargo test -p roko-learn --lib a_new_price_snapshot_keeps_the_learned_state"
+++

## Problem

A new price snapshot throws away every workspace's learned self-model. `predictor_version`
(`crates/roko-learn/src/self_model/model.rs`) hashes the model class, the feature schema and the price snapshot id, and
`SelfModel::load_or_new` sets aside a saved state of another version and starts a fresh model. So when the built-in
snapshot changes (gap-ceffb3 made `prices-2026-10-27` the built-in copy on 2026-10-28), every workspace that prices from
the built-in copy (`[pricing] snapshot = ""`) loses its outcomes, L1 weights and calibration window on its next load,
and the model goes back through its shadow warm-up (`SHADOW_OUTCOMES`, 30 outcomes).

## Why it matters

Snapshots will change every few weeks: gap-ceffb3's notes already list a promotional price that ends 2026-11-21
(gpt-5.6-sol) and a model retiring by December (kimi-k2.6). Each change would restart every workspace's learning,
although a price change alters cost forecasts, not what the model learned about which arm passes. Round v7's lead had
to archive Undertow's state by hand before the 10-28 merge (`tmp/hermes-roko-v7/lead/undertow-self-model-archive-20261028/`).

## Where

- `crates/roko-learn/src/self_model/model.rs::predictor_version`: the version includes `snapshot_id`.
- `crates/roko-learn/src/self_model/model.rs::load_or_new`: a state of another version is set aside, not migrated.
- `crates/roko-learn/src/self_model/model.rs::the_version_changes_with_the_snapshot_id`: the test that pins today's
  behaviour, which the fix changes.
- Any other learner keyed by `snapshot.id()` in `crates/roko-learn/` (check with grep).

## Current state

Unchanged since the self-model landed; first raised by round v7's lead on 2026-10-28 when gap-ceffb3 was merged.

## Plan

1. Keep the price snapshot out of the predictor version: version over model class and feature schema only, so only a
   feature-schema change sets state aside.
2. On load under another snapshot, keep outcomes, weights and calibration, and re-price cost forecasts at the current
   snapshot (`load_or_new` already prices a loaded model "at `snapshot` again"). Record the snapshot each stored cost
   was priced at, so costs from different snapshots are never mixed silently.
3. Replace `the_version_changes_with_the_snapshot_id` with a test of the new rule, and keep a test that a new feature
   schema still starts fresh.

## Done when

`a_new_price_snapshot_keeps_the_learned_state` passes: a model trained on some outcomes and saved under one snapshot
loads under another snapshot id with its outcomes and pass forecasts unchanged and its cost forecasts at the new prices;
and a feature-schema change still sets the state aside.

## Notes

- Raised by round v7's lead (session bb36bc6f) in reply to the gap-ceffb3 merge question, 2026-10-28.
