+++
id = "gap-2bec05"
kind = "gap"
title = "roko status and roko diagnose don't surface unpriced-call counts, text or JSON"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-02
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "125489b7d"
source = "backlog wave reports 2026-10-02 (PK07 gap-f548c1)"
discovered_from = "gap-f548c1"
anchors = ["crates/roko-cli/src/commands/status.rs", "crates/roko-cli/src/commands/diagnose.rs", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger"]
lane = "rust-cold"
parent = "spec-99d417"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn status_json_reports_unpriced_call_count' crates/roko-cli/ && cargo test -p roko-cli status_json_reports_unpriced_call_count"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T00:31:23Z"
commit = "125489b7d"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T16:06:45Z"
forced = false
evidence = "roko status --json now reports unpriced_calls beside total_cost_usd (SessionStatus field, set in commands/util.rs cmd_status from CostsLog::unpriced_calls over .roko/learn/costs.jsonl); its text cost summary line and SessionStatus text share status::unpriced_calls_summary. roko diagnose text prints \"N of M agent calls unpriced: cost unknown, not in the spend\" in render_text when the run has any (its JSON already had run_state.unpriced_calls). Code in 1b40a30c8, merged with main in 3d71bd283. Proof on that tree: the verify cargo test -p roko-cli status_json_reports_unpriced_call_count passes (cli_fallback test runs the roko binary over priced, free, flagged-unpriced and legacy $0 rows: 0 then 2, total 0.25); cargo test -p roko-cli unpriced passes incl. commands::diagnose::tests::diagnose_reports_unpriced_call_count and status::tests::unpriced_calls_show_beside_the_total; cargo clippy -p roko-cli --no-deps -- -D warnings is clean; a scratch workspace run of the built binary showed unpriced_calls 2 in status --json and the new diagnose line."
+++

## Problem

Correction to the source report: at HEAD, unpriced-call counts are not shown by either `roko status` (text or
`--json`) or `roko diagnose` — the premise that "the text output ... now shows" them does not hold; the count isn't
surfaced anywhere in either command (`grep -rn "unpriced" crates/roko-cli/src/commands/status.rs
crates/roko-cli/src/commands/diagnose.rs` matches nothing). The data exists internally —
`crates/roko-learn/src/telemetry/records.rs:635` (`unpriced_calls: u32`),
`crates/roko-cli/src/graph_task_dispatch/budget.rs` (an `AtomicUsize` counter on the plan budget ledger, snapshotted
around line 501-502), `crates/roko-learn/src/costs_log.rs:27` (`pub unpriced_calls: usize` on a spend struct) — but
neither CLI command reads any of it.

## Why it matters

Goal: truth. An operator running `roko status` or `roko diagnose` to understand why a plan's budget looks wrong
(e.g. spend lower than expected because calls to an unpriced model cost $0) has no visibility into how many of the
plan's calls went unpriced, in either human-readable or machine-readable form.

## Where

- `crates/roko-cli/src/commands/status.rs` and `crates/roko-cli/src/commands/diagnose.rs`: the two command
  implementations; neither reads `unpriced_calls` from any source.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs`: the live per-plan counter
  (`unpriced_calls: std::sync::atomic::AtomicUsize`, snapshot struct field around line 436).
- `crates/roko-learn/src/costs_log.rs:27`: a per-run spend struct with the same field, incremented at line 281.

## Current state

The counter is tracked and used internally for budget-ceiling logic (see the separate plan-ceiling gap, backlog task
2111, already tracked inside gap-f548c1) and recorded into episode extras
(`crates/roko-cli/src/runtime_feedback/episodes.rs:309-312`, key `"helper_unpriced_calls"`), but never read back by
`status` or `diagnose`.

2026-10-26: the source report described gap-f548c1's branch, which merged after this item was written.
`3eed76595` (backlog 2109) made `roko status`'s text print `Unpriced: N calls` in its cost summary
(`CostsLog::unpriced_calls` over `.roko/learn/costs.jsonl`) and gave `roko diagnose`'s JSON
`run_state.unpriced_calls` (the run's `costs.jsonl` rows, counted with `roko_learn::costs_log::is_unpriced`). Still
missing then: `roko status --json` had no count, `roko diagnose`'s text had no line, and no test covered either
command. The budget ledger's counters live only in the running process, so `costs.jsonl` stays the one source both
commands read.

## Plan

1. Pick one existing source of truth for "how many of this plan's calls were unpriced" (the budget ledger's live
   counter for a running plan, or a sum over `costs.jsonl`/`costs_log.rs` rows for a finished one) and read it in
   both `status.rs` and `diagnose.rs`.
2. Add it to both commands' JSON output (a new field, e.g. `unpriced_calls`) and to their text output (a line shown
   when the count is nonzero).
3. Add a test asserting the field/line appears when a fixture run has at least one unpriced call, and is absent or
   zero otherwise.

## Done when

- `roko status --json` and `roko diagnose`'s JSON output both include an unpriced-call count.
- The `[[verify]]` command passes.

## Notes

This is an observability gap, not a budget-enforcement one — the enforcement side (making the plan ceiling actually
account for unpriced calls) is backlog task 2111, already tracked inside gap-f548c1; don't duplicate that work here.
