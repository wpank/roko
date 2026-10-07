+++
id = "gap-14f08e"
kind = "gap"
title = "Compounding metrics such as playbook_hit_rate are never computed: compute_compounding_metrics has no caller"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-completion-loops 763596768"
anchors = ["crates/roko-learn/src/aggregate.rs::compute_compounding_metrics", "crates/roko-learn/src/aggregate.rs::append_compounding_metrics", "crates/roko-learn/src/aggregate.rs::AutocatalyticMetrics", "crates/roko-cli/src/runtime_feedback/episodes.rs", "crates/roko-cli/src/commands/learn.rs::print_learn_episodes"]
links = { depends_on = [], blocks = [], related = ["find-34a4b5", "reg-3f5969", "gap-1f611b", "bug-86117a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_episodes_json_reports_compounding_metrics' crates/roko-cli/src/commands && cargo test -p roko-cli --bin roko learn_episodes_json_reports_compounding_metrics"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:33Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:59Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`playbook_hit_rate` has no reader, because it is never computed.
`roko_learn::aggregate::compute_compounding_metrics` (`aggregate.rs:45`) produces
`AutocatalyticMetrics`, seven "does learning compound" rates:
- `knowledge_reuse_rate` and `playbook_hit_rate`;
- `cache_hit_rate` and `routing_accuracy`;
- `gate_pass_rate`, `error_dedup_rate` and `cost_per_success`.

Neither it nor `append_compounding_metrics` (`:164`) has a caller outside `aggregate.rs`'s own test.
`.roko/learn/compounding.jsonl` was last written on 2026-09-04, in the Runner-v2 era. Its last row
has `playbook_hit_rate` 0.0 and `knowledge_reuse_rate` 0.0. Nothing reads the file.

## Why it matters

Goal `learning`. Since 763596768 and 189a14e65, Graph episodes carry the inputs:
`runtime_feedback/episodes.rs:120-160` writes `knowledge_used`, `playbook_id`, `playbook_hits`,
`cache_hit` and `initial_model`. Nothing turns them into the rates that would show whether playbooks
and knowledge are being reused.

find-34a4b5's P0-04 row ("playbook hits feed `playbook_hit_rate`") pointed at reg-3f5969. That item
closed today, once playbook ids and outcomes were fixed, but it never computed the rate. The parked
gap-1f611b assumes the metrics are "computed but never used", but they are not computed at all.

## Where

- `crates/roko-learn/src/aggregate.rs`: `compute_compounding_metrics` is pure over `&[Episode]`, and
  `append_compounding_metrics` appends a row to a JSONL file.
- `crates/roko-cli/src/commands/learn.rs`: `print_learn_episodes` (`:1505`) and
  `collect_episodes_json` / `LearnJsonEpisodes` (`:881`, `:658`) already load the episode log.
  `commands/` is compiled into the binary, not the lib.

## Current state

Checked at 33e107da1: `grep -rn 'compute_compounding_metrics\|append_compounding_metrics' crates/`
finds only `aggregate.rs`. A grep for "compounding" finds no reader of `compounding.jsonl` in
`roko-cli`, `roko-serve` or the TUI.

## Plan

1. Compute the metrics on demand in `roko learn episodes` over a bounded recent window (for example
   the last 200 episodes). Print them, and add them to `LearnJsonEpisodes` for `--json`.
2. Either delete `append_compounding_metrics` and the stale `compounding.jsonl` writer path, or
   append one row per plan at the end of `run_one_plan` if a trend is wanted. The second option
   depends on the plan-end hook in q-6b7cca.
3. Until bug-86117a keeps neuro ids and episode ids apart, `knowledge_used` is true whenever the
   prompt surfaced episodes, which inflates `knowledge_reuse_rate`. Say so in the output, or land
   bug-86117a first.

Deleting the metrics instead is acceptable if nobody wants them. If so, record the decision here and
delete the whole `AutocatalyticMetrics` path.

## Done when

- `roko learn episodes` and its `--json` output report `playbook_hit_rate` and the other six rates
  for the recent window.
- The test `learn_episodes_json_reports_compounding_metrics` (binary `roko`, `commands/learn.rs`)
  writes episodes with and without `playbook_id` and asserts the rate. The `[[verify]]` command
  passes.
- If the metrics are computed at plan end instead, the verify moves to a guarded roko-cli lib test.

## Notes

- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  `roko learn episodes` prints the seven rates over the last 200 episodes (`COMPOUNDING_WINDOW`), and `--json` adds
  them as `episodes.compounding`; the test `learn_episodes_json_reports_compounding_metrics` covers both.
- Plan step 2: `append_compounding_metrics` is deleted (no caller, not even a test). A trend can be recomputed from
  `episodes.jsonl`; `.roko/learn/compounding.jsonl` is a stale file nothing writes or reads.
- Plan step 3: 133c02093 (bug-86117a) keeps episode ids out of `knowledge_ids`, so `knowledge_used` no longer means
  "episodes surfaced". But `compute_compounding_metrics` counted the key's presence, and Graph episodes always write
  it (`false` when nothing was injected), so every Graph episode counted as reuse. It now counts only `true`
  (test `knowledge_used_false_is_not_knowledge_reuse`).
