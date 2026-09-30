+++
id = "bug-62e3f4"
kind = "bug"
title = "Episodes, costs.json and efficiency.jsonl leave out the three helper calls after each failed gate"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "6f8286d48"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::select_cheap_model_key", "crates/roko-cli/src/graph_task_dispatch.rs:431", "crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/feedback.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-a6e2c3", "gap-e003ec", "bug-f9ae3e", "bug-31438d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn helper_calls_after_a_failed_gate_are_costed' crates/roko-cli/src/ && cargo test -p roko-cli --lib helper_calls_after_a_failed_gate_are_costed"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 6f8286d48. The helper calls after a failed gate are recorded in episodes, costs.json and efficiency.jsonl, tagged role helper. Batch 12b gate on the merged tree (MAIN 6f8286d48 has the same crates and Cargo.lock as gated b0ede92d7): check, nightly fmt and clippy -p roko-cli -p roko-agent -p roko-learn -p roko-serve --no-deps -D warnings clean; lib tests pass: roko-cli 3167, roko-agent 2262, roko-learn 1200, roko-serve 977 (two load flakes, a_timed_out_attempt_reports_the_usage_it_streamed and a_timed_out_attempt_is_resumed_with_an_escalated_timeout, pass alone); cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: helper_calls_after_a_failed_gate_are_costed passes (roko-cli lib)."
+++

## Problem

After each failed gate, Graph dispatch makes three best-effort helper calls on the cheapest dispatchable model: a quality judgement, an error diagnosis and a gate reflection. The model is chosen by `select_cheap_model_key` (`graph_task_dispatch/routing_context.rs:70`: "quality judge, error enrichment, gate reflections"), and the helper dispatch starts at `graph_task_dispatch.rs:431`.

wk-bench-rokoarm counted every call against a loopback fake at 33e107da1:

- episodes and the Graph `costs.json` leave all three helper calls out;
- `.roko/learn/efficiency.jsonl` logged 11 of 12 calls, missing the last helper call.

The benchmark records this in `driver/run_roko.py:40-42`. `arms/roko_fixed.toml:22` budgets 45 calls per task: "3 attempts x (12 turns + 3 auxiliary calls after a failed gate)".

## Why it matters

p1 for the epic "one settled record per attempt": Roko's recorded cost is too low for every task that failed a gate at least once, which is exactly the retried work its thesis is about.

- Cost per verified task, the field rollup and the budget guard all read these records.
- The benchmark can't use Roko's own cost, and has to meter the arm from outside (the proxy, gap-e003ec).

## Where

- The helper dispatch at `graph_task_dispatch.rs:431` and its callers in `verification.rs`, the failed-gate path.
- The cost and efficiency writers in `feedback.rs`.
- `select_cheap_model_key`.

## Current state

At BASE (4315add32), the helper calls run outside the task's usage accounting. Since 33e107da1 only a module split (a729fb911) and unrelated fixes touched this code.

## Plan

1. Give each helper call the same accounting as a task dispatch: a cost row with a class (for example `helper`), the attempt id and the model, plus one efficiency record.
2. Add their cost to the attempt's episode, as a separate `helper_cost_usd` or inside the total with a breakdown, and to the task's `costs.json` and budget.
3. Add `helper_calls_after_a_failed_gate_are_costed`: a task that fails its gate once and then passes records the helper calls' tokens and cost in costs.jsonl, efficiency.jsonl and the episode.

## Done when

- [ ] For a task with a failed gate, the sum of recorded calls equals the calls the provider saw, and the cost includes the helpers.
- [ ] The `[[verify]]` command passes.

## Notes

- gap-a6e2c3 is the same kind of hole for plan generation and revision spend.
- bug-31438d (the provider-reported model) touches the same writers. Fix them together if convenient.
- Implemented on `work/bug-31438d` at `18070c0dc`; cargo verification deferred to the batch check. `helper_calls_after_a_failed_gate_are_costed` (targeted `cargo test` passed at the branch head). Changes:
  - `cheap_agent()` returns a `HelperAgent` that counts each call reaching a provider toward the attempt whose verify steps are settling. A tokio task-local set around `settle_task_verification` on both dispatch paths does this, with no edits to `verification.rs` or `routing_context.rs`.
  - The attempt waits for its background helpers (at most `timeouts.llm_call_secs` + 5 s), then records each call: task spend, the plan's cost ledger (`costs.json`), and a cost row and an efficiency row keyed by the attempt (`role = "helper"`, `attempt_id = <key>/helper-N`).
  - The totals go on the verdict (`helpers`) and the episode (`extra.helper_calls`, `helper_cost_usd`, `helper_tokens_in/out`), outside the agent run's `usage`.
  - Not done: a helper call still running when the wait ends is logged, not recorded. Calls refused for exhaustion during failover are accounted in spend but still have no rows.
