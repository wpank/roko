+++
id = "gap-62e1b9"
kind = "gap"
title = "PK47 M3 self-model: Claude CLI: re-price every model in modelUsage from the snapshot and keep… (+11 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 47
size = "L"
subsystem = ["roko-learn/self_model"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK47"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/provider/codex_cli/stream.rs", "crates/roko-learn/src/lib.rs", "crates/roko-learn/src/prediction.rs", "crates/roko-learn/src/routing_extras.rs"]
lane = "rust-cold"
parent = "spec-abbc62"
links = { depends_on = ["gap-08120e", "gap-9e3134"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn claude_model_usage_is_repriced_per_model' crates/roko-agent/ && cargo test -p roko-agent claude_model_usage_is_repriced_per_model"

[[verify]]
command = "grep -rqw 'fn codex_cost_uses_snapshot' crates/roko-agent/ && cargo test -p roko-agent codex_cost_uses_snapshot"

[[verify]]
command = "test -f config/prices/2026-09-28.probes.md && grep -q 'reasoning_output_tokens' config/prices/2026-09-28.probes.md && grep -q 'costBasis' config/prices/2026-09-28.probes.md"

[[verify]]
command = "grep -rqw 'fn arm_key_round_trips' crates/roko-learn/src/self_model/ && cargo test -p roko-learn arm_key_round_trips"

[[verify]]
command = "grep -rqw 'fn ingest_joins_open_and_verdict_lines_by_attempt_key' crates/roko-learn/src/self_model/ && cargo test -p roko-learn ingest_joins_open_and_verdict_lines_by_attempt_key"

[[verify]]
command = "grep -rqw 'fn legacy_dedupe_reproduces_the_pinned_audit' crates/roko-learn/src/self_model/ && cargo test -p roko-learn legacy_dedupe_reproduces_the_pinned_audit"

[[verify]]
command = "grep -rqw 'fn murphy_decomposition_matches_brier' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::metrics && ! grep -q 'pub struct RouterCalibration' crates/roko-learn/src/routing_extras.rs"

[[verify]]
command = "grep -rqw 'fn l0_backs_off_to_parent_when_stratum_is_empty' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::prior"

[[verify]]
command = "grep -rqw 'fn l1_logit_recovers_synthetic_weights' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::logit"

[[verify]]
command = "grep -rqw 'fn nig_quantiles_cover_lognormal_costs' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::cost"

[[verify]]
command = "grep -rqw 'fn prequential_ece_below_003_after_isotonic' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::recal"

[[verify]]
command = "grep -rqw 'fn state_round_trips_and_version_tracks_schema' crates/roko-learn/src/self_model/ && cargo test -p roko-learn self_model::model"
+++

## Problem

This package delivers 12 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK47, slice 61xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6105 | S | p2 | Claude CLI: re-price every model in modelUsage from the snapshot and keep total_cost_usd as the vendor figure | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6105-claude-cli-re-price-every-model-in-modelusage-from-the.md` |
| 2 | 6106 | S | p2 | Codex CLI: price turns from the snapshot, with no gpt-5.6-sol fallback for unknown slugs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6106-codex-cli-price-turns-from-the-snapshot-with-no-gpt-5-6-sol.md` |
| 3 | 6107 | S | p2 | Probe Claude Code and Codex CLI costs by hand and file a vendor-versus-snapshot table | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6107-probe-claude-code-and-codex-cli-costs-by-hand-and-file-a.md` |
| 4 | 6108 | S | p2 | Create the roko_learn::self_model module with its shared types and one stub file per part | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6108-create-the-roko-learn-self-model-module-with-its-shared.md` |
| 5 | 6109 | S | p2 | Read S01 run records into labelled self-model units | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6109-read-s01-run-records-into-labelled-self-model-units.md` |
| 6 | 6110 | S | p3 | Normalise the legacy efficiency and episode logs and print an honest data audit | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6110-normalise-the-legacy-efficiency-and-episode-logs-and-print.md` |
| 7 | 6111 | S | p2 | Score forecasts with one metrics module and delete the unused RouterCalibration | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6111-score-forecasts-with-one-metrics-module-and-delete-the.md` |
| 8 | 6112 | S | p2 | Build the L0 hierarchical Beta prior with backoff, forgetting and a lower confidence bound | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6112-build-the-l0-hierarchical-beta-prior-with-backoff-forgetting.md` |
| 9 | 6113 | M | p2 | Add the feature vector and the L1 online Bayesian logistic forecaster with a false-green head | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6113-add-the-feature-vector-and-the-l1-online-bayesian-logistic.md` |
| 10 | 6114 | S | p2 | Forecast attempt cost and latency with a Normal-Inverse-Gamma regression on log scale | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6114-forecast-attempt-cost-and-latency-with-a-normal-inverse.md` |
| 11 | 6115 | S | p2 | Recalibrate forecasts with cross-fitted isotonic regression | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6115-recalibrate-forecasts-with-cross-fitted-isotonic-regression.md` |
| 12 | 6116 | M | p2 | Combine the parts into a SelfModel with forecast, observe, a versioned state file and a cold start | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6116-combine-the-parts-into-a-selfmodel-with-forecast-observe-a.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `config/prices/2026-09-28.probes.md`, `crates/roko-agent/src/claude_cli_agent.rs`, `crates/roko-agent/src/provider/codex_cli/stream.rs`, `crates/roko-learn/src/lib.rs`, `crates/roko-learn/src/prediction.rs`, `crates/roko-learn/src/routing_extras.rs`, `crates/roko-learn/src/self_model/baselines.rs`, `crates/roko-learn/src/self_model/cascade.rs`, `crates/roko-learn/src/self_model/cost.rs`, `crates/roko-learn/src/self_model/features.rs`, `crates/roko-learn/src/self_model/gate.rs`, `crates/roko-learn/src/self_model/ingest.rs`, `crates/roko-learn/src/self_model/legacy.rs`, `crates/roko-learn/src/self_model/logit.rs`, `crates/roko-learn/src/self_model/metrics.rs`, `crates/roko-learn/src/self_model/mod.rs`, `crates/roko-learn/src/self_model/model.rs`, `crates/roko-learn/src/self_model/ope.rs`, `crates/roko-learn/src/self_model/policy.rs`, `crates/roko-learn/src/self_model/prior.rs`, `crates/roko-learn/src/self_model/recal.rs`, `crates/roko-learn/src/self_model/replay.rs`, `crates/roko-learn/tests/fixtures/self_model/legacy/**`, `crates/roko-learn/tests/fixtures/self_model/runs/**`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK12 (gap-08120e), PK13 (gap-9e3134).
- Suggested model: opus.
