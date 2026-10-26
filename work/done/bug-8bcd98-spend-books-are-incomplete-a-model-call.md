+++
id = "bug-8bcd98"
kind = "bug"
title = "Spend books are incomplete: a model call that fails writes no call_cost row, a verdict's billed_usd leaves out the calls refused before its failover (7b8cb121 T2#1: $0.0354 of $0.2430), and a try killed mid-call has no cost row (6059db44: the plugin's Build pad showed $0.04 of $0.0627)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "ae712ef2e"
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask; plugin-wow's finding on 6059db44 (tmp/hermes-roko-v7/findings/plugin-wow.md)"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::run_inner", "crates/roko-cli/src/graph_task_dispatch/call_cost.rs::MeteredCalls", "crates/roko-learn/src/telemetry/records.rs::AttemptCost", "crates/roko-learn/src/telemetry/records.rs::plus_refused", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::call_refused", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::record_refused_spend", "crates/roko-learn/src/feedback_service.rs::model_call_cost_record"]
links = { depends_on = [], blocks = [], related = ["gap-36f3fb", "bug-dceafd", "gap-5cfa9c", "gap-1ba0ac", "gap-ece700"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'refused_usd' crates/roko-learn/src/telemetry/records.rs && grep -rqw 'fn a_call_that_fails_writes_its_call_cost_row' crates/roko-cli/src && cargo test -p roko-cli --lib a_call_that_fails_writes_its_call_cost_row"

[[verify]]
command = "grep -rqw 'fn a_failed_over_try_s_verdict_counts_its_refused_calls' crates/roko-cli/src && cargo test -p roko-cli --lib a_failed_over_try_s_verdict_counts_its_refused_calls && cargo test -p roko-learn --lib a_try_s_spend_counts_its_refused_calls"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:24:08Z"
commit = "ae712ef2e"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T11:59:05Z"
forced = false
evidence = "Part 2 merged as eab74f5b2 (the verdict's cost.refused_usd; chain45b green). Part 1, ae712ef2e: CallFacts.outcome (call_outcome: timeout, rate_limited, refused, error), the tool loop records a call that fails past its retries, streamed or not, with what it streamed (else nothing), its wall time and how it ended, and MeteredCalls publishes its call_cost row with outcome (ok on success). chain47 (main 7675b37f7 plus part 1): fmt and clippy clean (with roko-std and roko-graph), the verify list passed (a_call_that_fails_writes_its_call_cost_row: a provider that starts its answer then stalls gives the try one row of 0 tokens, $0, timeout and >= 300 ms; a_failed_calls_outcome_names_how_it_failed; a_call_cut_off_mid_stream_records_what_it_streamed; a_failed_over_try_s_verdict_counts_its_refused_calls, now with the 429-ended call's rate_limited row; a_running_try_reports_its_spend_per_model_call; a_gpt5_tool_loop_reasons_on_the_responses_api), lib suites: roko-agent 2415, roko-core 2049, roko-learn 1404, roko-runtime 213, roko-serve 1084 green; roko-cli 4029 passed and 2 failed, frozen_learning_run_writes_no_learned_state and frozen_learning_switch_reaches_config_and_manifest, which fail on main itself since find-697944 (9d68ed77f) added audit_off/holdout_off to ablation_flags without updating them. Part 3 (killed tries) was roko-recovery's bug-b7f6a8 (merged 5e9af6682)."
+++

## Problem

roko keeps spend in several books:
- **per call:** the `call_cost` events, which the run summary and the live spend sum;
- **per try:** the verdict's `cost` in `attempts.jsonl`, and the `learn/costs.jsonl` rows written from
  `FeedbackEvent::ModelCall`;
- **per plan:** the budget ledger, `state/graph/<plan>/costs.json`.

In round v7 they disagree in three ways.

1. **A model call that fails without a usage report writes no `call_cost` row.** This covers response timeouts, 429s
   and HTTP 4xx refusals.
   - bdc4461e T1#1 and T1#2 each lost a call to the 120 s response wait. 4 min of their model time has no call row.
   - 16fc88ae's four tries were refused (HTTP 400) and left no row.
   - The per-call book loses the call, its time and its outcome.
   - A call cut off mid-stream is booked through the stream tally (bug-72d775). A call billed with no usage report
     would be lost.
2. **A verdict's `cost.billed_usd` leaves out the routed model's calls that a failover refused.**
   - 7b8cb121 T2#1 billed $0.0354, but its calls cost $0.2430. The difference, $0.2076, was glm-4.7's work before the
     z.ai 429 failover.
   - 20c780ae's tries leave out $0.027.
   - The run summary and `learn/costs.jsonl` (role `failover_refused`) carry the refused calls; the verdict doesn't.
     gap-5cfa9c is the same gap on the run summary's `rungs[]` side.
3. **A try killed mid-call has no verdict and no `learn/costs.jsonl` row.** This was 6059db44 T1#1, killed by the
   2026-10-26 reboot.
   - `FeedbackEvent::ModelCall` is sent only when the dispatch returns.
   - The try's $0.0221 is in the run summary (its `call_cost` events) and in the plan ledger ($0.0627 for the run).
   - plugin-wow found the Build pad, which reads the cost rows, showing $0.04 for phase 4.1.
   - roko-recovery (round v7, `tmp/hermes-roko-v7/findings/roko-recovery.md`) is characterizing the kill case
     (gap-36f3fb, bug-dceafd). This item names it and leaves its fix there.

Expected: every call a try makes, failed or refused, appears in each book with its outcome, and the books agree.

## Why it matters

Truth: every number in the reports and every learning signal reads these books.
- The round's accounting had to re-price each try from its `call_cost` rows to match the pilot's ledger, because the
  verdicts don't add up to the run's spend.
- A failed call's missing time makes model time look like it has no calls.
- The cost rows feed the self-model and the plugin's spend.

## Where

- `roko-agent/src/tool_loop/mod.rs::run_inner`: a call's `Err` path. It records usage only when the stream tally has
  some (`budget.record_turn`).
- `graph_task_dispatch/call_cost.rs::MeteredCalls`: publishes `call_cost` from `record_model_call`, for calls that
  return usage.
- `roko-learn/src/telemetry/records.rs::AttemptCost`: the verdict's `cost`.
- `roko-learn/src/feedback_service.rs::model_call_cost_record`: the `learn/costs.jsonl` row, from
  `FeedbackEvent::ModelCall`.

## Current state

run-analyst checked it over round v7 (`tmp/hermes-roko-v7/run-analyst`).
- **Per run:** the sum of `call_cost` equals the plan ledger and the pilot's ledger to the cent, for every run.
- **Per try:** the verdict differs wherever a failover refused calls.
- **Cost rows:** every try except the killed one has a `learn/costs.jsonl` row. Provider errors and cancelled tries
  included.

## Plan

1. **Failed calls:** on a call's failure, report it with its outcome and wall time, and zero usage unless the
   provider billed some. For example, give `TurnBudget::record_turn` an outcome, or add a
   `record_failed_call(outcome, wall_ms)`. `MeteredCalls` then publishes a `call_cost` row with `outcome` (ok,
   timeout, rate_limited, refused, error) and `wall_ms`.
2. **The verdict:** add `cost.refused_usd`, what the try's refused calls cost, beside `billed_usd`. A try's spend is
   billed plus refused. The verdict's writer and every reader of `billed_usd` that means "the try's spend" use the
   sum.
3. **Killed tries:** leave them to roko-recovery's item. It is expected that a resume or serve's settling writes the
   abandoned try's cost row from its `call_cost` events.

## Progress (roko-speed, 2026-10-26)

- **Part 2 (the verdict), built:**
  - A verdict's `cost.refused_usd` is what the try's calls cost that a provider refused part-way before failover
    moved the try on, or before it called the same model again. Older rows lack it, and it is absent when no call was
    refused.
  - The attempt's progress sums each refused call as `run_bridge_with_failover` settles it, so the error paths count
    it too. The attempt records the sum on settle.
  - `AttemptCost::plus_refused` gives the try's spend. The attempt_settled event's `cost_usd`, the audit lottery's
    run spend and the TUI's attempt metric use it.
  - `rung_stats` leaves it out on purpose: refused calls count on the refused model's row, from the cost rows
    (gap-5cfa9c).
  - Test: `a_failed_over_try_s_verdict_counts_its_refused_calls`. The routed model reads a file ($0.102), then 429s
    past the loop's retries; a backup finishes ($0.011). Billed plus refused equals the try's `call_cost` sum and
    the plan's spend.
- **Part 1 (failed calls' rows), built on gap-1ba0ac's `CallFacts`:**
  - `CallFacts` gains `outcome`, and `call_outcome(&LlmError)` names it:
    - `timeout`: a response wait or stream stall;
    - `rate_limited`: a 429 past the retries;
    - `refused`: a 4xx refusal, a credential or model the provider won't serve, or spent usage or balance;
    - `error`: anything else.
  - The tool loop tells its budget of a call that fails past its retries: streamed or not, with what it streamed
    (else no tokens), how long it took and how it ended.
  - `MeteredCalls` publishes its `call_cost` row with `outcome`, `ok` for a call that returned a response. A failed
    call's row costs what it streamed, else $0, so the call is still counted.
  - Test: `a_call_that_fails_writes_its_call_cost_row`. A provider answers 200 and then streams nothing; the try's
    one row has no tokens, $0, `timeout`, and a wait of at least the 300 ms TTFT limit.
  - Also `a_failed_calls_outcome_names_how_it_failed`. The failover test now sees the 429-ended call's row.
  - A non-streamed call whose retries all failed returns `RetriesExhausted` and reads `error`. A try's calls stream.
- **Part 3** is roko-recovery's, as bug-b7f6a8.

## Done when

- A call that times out writes a `call_cost` row with outcome `timeout` and its `wall_ms`. Test:
  `a_call_that_fails_writes_its_call_cost_row`.
- A verdict whose try failed over carries `cost.refused_usd`, and billed plus refused equals the try's `call_cost`
  sum.
- The `[[verify]]` passes.

## Notes

- Not here: `reasoning_tokens` and `finish_reason` on `call_cost`. roko-effort takes those with gap-1ba0ac.
- The time fields on every call (`wall_ms`, `ttft_ms`, provider waits) are in gap-ece700.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, "How the numbers were made" and telemetry gaps T1 and T8.

- 2026-10-26 (roko-recovery, round v7): **part 3 (killed tries) moves to bug-b7f6a8 (p1).**
  - On resume, the dead attempt is settled `abandoned` with its `call_cost` spend, the call in flight estimated, in
    `attempts.jsonl`, `learn/costs.jsonl` and the plan ledger.
  - The seam, as the lead set it: roko-recovery takes part 3, and roko-speed takes parts 1 and 2.
  - Where a dead try's spend lands. Kill matrix on 4d22b94e8 (`tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8`,
    a call costs $0.041):

  | kill point | dead try's calls | verdict row | plan ledger `costs.json` | `learn/costs.jsonl` | serve summary of the dead run |
  |---|---|---|---|---|---|
  | a: model stream | $0.041 | none | no ($0 spent, $1.00 reserved) | no | yes |
  | b: tool call | $0.082 | none | no (reserved) | no | yes |
  | c': hand-off check | $0.082 | none | no (reserved) | no | yes |
  | c: gate verify after the hand-off | $0.082 | none | **yes** (committed at the agent's end) | no | yes |
  | d: a settled try, killed before its cost row | T1#1 settled ($0.164) | yes | yes | **no** (the row comes after the verdict) | yes |
  | d'' / d3: just after T1's settle (serve run) | none open | yes | yes | yes | **no: $0 and 0 tasks** (its index lacks the rows after `ev.update`; not traced further) |

  - After a resume (c), the plan ledger counts dead and new spend ($0.287). `learn/costs.jsonl` and the resumed serve
    run's summary count only the new ($0.205).
  - After a start-over (a, a', b, c', d3), the old ledger is set aside and the new one starts at $0. Only the
    `call_cost` events and the dead run's summary keep the dead try.
  - The pilot's live P4.1 numbers match row c: the plan ledger has $0.0627, and the cost rows have $0.0406.
