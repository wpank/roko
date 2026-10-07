+++
id = "gap-f548c1"
kind = "gap"
title = "PK07 Attempt ledger: OpenAI-compatible streaming requests ask for usage, so OpenAI spend reaches costs and… (+9 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 7
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "509e3e807"
source = "tmp/backlog/2026-10-02-complete-and-wire PK07"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/streaming.rs", "crates/roko-agent/src/tool_loop/backends/mod.rs", "crates/roko-cli/src/commands/diagnose.rs", "crates/roko-cli/src/commands/util.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/budget.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs", "crates/roko-cli/src/graph_task_dispatch/reflex_credit.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/plan_authoring.rs", "crates/roko-cli/src/runtime_feedback/episodes.rs", "crates/roko-core/src/config/provider.rs", "crates/roko-learn/src/costs_db.rs", "crates/roko-learn/src/costs_log.rs", "crates/roko-learn/src/efficiency.rs", "crates/roko-learn/src/episode_logger.rs", "crates/roko-learn/src/feedback_service.rs", "crates/roko-learn/src/runtime_feedback/episode_helpers.rs", "crates/roko-learn/src/telemetry/records.rs", "crates/roko-neuro/src/episode_completion.rs"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = ["gap-625195"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'include_usage' crates/roko-agent/src/openai_compat_backend.rs"

[[verify]]
command = "grep -rqw 'fn streaming_request_asks_for_usage' crates/roko-agent/src/ && cargo test -p roko-agent streaming_request_asks_for_usage"

[[verify]]
command = "grep -rqw 'fn null_usage_chunk_keeps_its_finish_reason' crates/roko-agent/src/ && cargo test -p roko-agent null_usage_chunk_keeps_its_finish_reason"

[[verify]]
command = "grep -q 'verify_started_at' crates/roko-cli/src/graph_task_dispatch/attempt.rs"

[[verify]]
command = "grep -rqw 'fn verdict_records_first_token_and_verify_times' crates/roko-cli/src/ && cargo test -p roko-cli verdict_records_first_token_and_verify_times"

[[verify]]
command = "grep -q 'verdict.usage' crates/roko-cli/src/graph_task_dispatch/attempt.rs"

[[verify]]
command = "grep -rqw 'fn settle_fills_usage_and_cost_amounts' crates/roko-cli/src/ && cargo test -p roko-cli settle_fills_usage_and_cost_amounts"

[[verify]]
command = "grep -rq 'VerifyStepVerdict' crates/roko-cli/src/graph_task_dispatch/"

[[verify]]
command = "grep -rqw 'fn verdict_records_per_step_results_and_skip_reasons' crates/roko-cli/src/ && cargo test -p roko-cli verdict_records_per_step_results_and_skip_reasons"

[[verify]]
command = "grep -rqw 'fn graph_episode_started_before_completed' crates/roko-cli/src/ && cargo test -p roko-cli graph_episode_started_before_completed"

[[verify]]
command = "grep -q 'skip_reason' crates/roko-learn/src/episode_logger.rs"

[[verify]]
command = "grep -rqw 'fn episode_records_per_rung_verdicts_and_skip_reasons' crates/roko-cli/src/ && cargo test -p roko-cli episode_records_per_rung_verdicts_and_skip_reasons"

[[verify]]
command = "! grep -q 'gate-pass' crates/roko-cli/src/graph_task_dispatch/verification.rs"

[[verify]]
command = "grep -rqw 'fn efficiency_writes_one_keyed_row_per_attempt' crates/roko-cli/src/ && cargo test -p roko-cli efficiency_writes_one_keyed_row_per_attempt"

[[verify]]
command = "grep -rqw 'fn efficiency_prompt_sections_carry_token_counts' crates/roko-cli/src/ && cargo test -p roko-cli efficiency_prompt_sections_carry_token_counts"

[[verify]]
command = "grep -q 'pub priced' crates/roko-learn/src/costs_db.rs"

[[verify]]
command = "grep -rqw 'fn cost_rows_mark_unpriced_calls' crates/roko-cli/src/ && cargo test -p roko-cli cost_rows_mark_unpriced_calls"

[[verify]]
command = "grep -rqw 'fn spend_counts_flagged_unpriced_rows' crates/roko-learn/src/ && cargo test -p roko-learn spend_counts_flagged_unpriced_rows"

[[verify]]
command = "grep -rqE 'fn plan_ceiling_(refuses|charges|counts)_an_unpriced_call' crates/roko-cli/src/graph_task_dispatch/ && cargo test -p roko-cli plan_ceiling_"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T18:37:09Z"
commit = "509e3e807"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T16:02:51Z"
forced = false
evidence = "Gate 3a on work/backlog-batch-3 (merged into main as 509e3e807; main differs from the gated tree only in work/ files): cargo check --workspace --tests, clippy -D warnings, nextest --lib 11,514 passed over 10 crates, golden-path canaries 13/13 (7 targets), roko-agent sse_replay 1/1; every [[verify]] passes (lib tests named in each verify passed; integration tests run by target; static parts rc=0)."
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK07, slice 21xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2101 | S | p1 | OpenAI-compatible streaming requests ask for usage, so OpenAI spend reaches costs and the budget | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2101-openai-streams-ask-for-usage.md` |
| 2 | 2102 | S | p2 | The attempt verdict records its first-token time and its verify start and end | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2102-verdict-records-first-token-and-verify-times.md` |
| 3 | 2103 | M | p1 | Settling an attempt fills the verdict's token classes and the cost amounts the provider reported | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2103-settle-fills-usage-and-reported-cost.md` |
| 4 | 2104 | M | p2 | Each verify step's result, exit code, duration and skip reason reaches the attempt verdict | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2104-verify-steps-reach-the-verdict.md` |
| 5 | 2105 | S | p3 | Graph episodes record when the attempt started, not the moment they were written | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2105-graph-episodes-start-when-the-attempt-started.md` |
| 6 | 2106 | S | p2 | Episodes record a gate verdict per verify step, with exit code, duration and skip reason | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2106-episodes-record-per-step-gate-verdicts.md` |
| 7 | 2107 | M | p2 | The efficiency log gets one settled row per attempt instead of an extra zero row per verify outcome | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2107-one-settled-efficiency-row-per-attempt.md` |
| 8 | 2108 | S | p3 | Efficiency rows record each prompt section's token count instead of 0 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2108-efficiency-prompt-sections-carry-token-counts.md` |
| 9 | 2109 | M | p2 | Cost rows say whether the call was priced, so an unknown cost is no longer read as $0 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2109-cost-rows-say-whether-the-call-was-priced.md` |
| 10 | 2111 | S | p2 | The plan ceiling applies the chosen rule to calls it cannot price | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2111-plan-budget-applies-the-unpriced-call-rule.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2100-the-attempt-ledger-counts-tokens-and-money.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-agent/src/openai_compat_backend.rs`, `crates/roko-agent/src/streaming.rs`, `crates/roko-agent/src/tool_loop/backends/mod.rs`, `crates/roko-cli/src/commands/diagnose.rs`, `crates/roko-cli/src/commands/util.rs`, `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/budget.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`, `crates/roko-cli/src/graph_task_dispatch/reflex_credit.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/plan_authoring.rs`, `crates/roko-cli/src/runtime_feedback/episodes.rs`, `crates/roko-core/src/config/provider.rs`, `crates/roko-learn/src/costs_db.rs`, `crates/roko-learn/src/costs_log.rs`, `crates/roko-learn/src/efficiency.rs`, `crates/roko-learn/src/episode_logger.rs`, `crates/roko-learn/src/feedback_service.rs`, `crates/roko-learn/src/runtime_feedback/episode_helpers.rs`, `crates/roko-learn/src/telemetry/records.rs`, `crates/roko-neuro/src/episode_completion.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK01 (gap-625195).
- Suggested model: opus.

## Progress

Implemented on `work/gap-f548c1`; cargo verification deferred to the batch gate (2026-10-02, claude-agent). The
static part of all 19 `[[verify]]` entries passes at the branch head.

- 2101: implemented at 89b3237e0
- 2102: implemented at 1d34a3c6d
- 2103: implemented at 96cf1a95b
- 2104: implemented at 24d263679
- 2105: implemented at 39b795270
- 2106: implemented at 66f8b03a2
- 2107: implemented at 685c53311
- 2108: implemented at 1f8941c30
- 2109: implemented at 3eed76595
- 2111: implemented at 4add4f8b0
- b54d4ab96: hand-formatting pass over new lines of 2102, 2108 and 2109; no behaviour change.

Notes for the gate:

- 2101: the null-usage guard in `parse_sse_chunk` had already landed with backlog 1110; the task added its test.
  `ProviderConfig.stream_usage` is a new field, so all 131 full literals of `ProviderConfig` across eight crates
  set `stream_usage: None`. The opt-out is also wired in the Gemini OpenAI-compatible adapter.
- 2103: `vendor_usd` is filled only for CLI agents. An API provider's usage cost is roko's own price, not a vendor
  figure (S01 §4.4), so it does not fill `vendor_usd`. `billed_usd` stays null when the usage source is unknown.
- 2104: the steps reach the verdict through `AttemptContext::record_verify_steps`, beside `record_helper_calls`,
  so `Settlement::verified` and `reflex_credit.rs` are unchanged.
- 2107: the unknown time to first token is marked by a `TtftRow` wrapper (`ttft_unknown`), beside `TurnsRow`.
- 2111: plan-level unpriced calls are counted with `Usage::has_known_cost`, as the daily and task ledgers count
  them. A model priced at 0/0 therefore still counts as unpriced for both ceilings; using 2109's `priced` flag
  there would change `record_task_spend`'s six callers.
