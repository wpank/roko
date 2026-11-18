+++
id = "bug-80c642"
kind = "bug"
title = "roko plan status's SPENT can fall mid-run: around a try's settle its spend counts in the committed ledger and in its open try's live cost, then once (7.3: $0.17 then $0.14 as task 2 settled); and a frame between the plan check's end and its delivery record shows no plan check row"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-18
updated = 2026-11-18
last_verified = 2026-11-18
source = "round v8 guide-v8: phase 7.3's terminal capture (run 0d8e8a2c), frames 02:55:21-02:55:31 UTC and 02:58:45"
discovered_from = "agent:guide-v8 (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::RunReadings", "crates/roko-cli/src/graph_checkpoint.rs::CommittedHold", "crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptHold", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-serve/src/routes/runs.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7a62d9", "gap-9bfb4e", "bug-addd24"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_spent_never_falls_as_a_try_settles' crates/roko-cli/src && grep -rqw 'fn a_settling_try_counts_once' crates/roko-serve/src && cargo test -p roko-cli --lib -- plan_status_spent_never_falls_as_a_try_settles plan_status_shows_what_is_running a_plan_without_a_ceiling_commits_its_attempts_spend_too a_running_try_reports_its_spend_per_model_call && cargo test -p roko-serve --lib -- a_settling_try_counts_once"
+++

## Problem

`roko plan status`'s SPENT can go down mid-run, which the shared readings rule (the design system's §4.8) doesn't
allow. guide-v8's terminal capture of round v8's phase 7.3 (Undertow `p7-3-the-playing-step-row-as-designed`, run
0d8e8a2c, frames 2 s apart, in `captures/three-surfaces/terminal-raw/p7-3-the-playing-step-row-as-designed/`) shows
it:

| UTC | Header | Task 2 |
|---|---|---|
| 02:55:21.78 | 0/3 · 1m 33s · $0.07 · 0/10 | running · 1m 04s |
| 02:55:23.78 | 0/3 · 1m 35s · $0.14 · 0/10 | running · 1m 06s |
| 02:55:25.78 | 0/3 · 1m 37s · $0.17 · 0/10 | running · 1m 08s |
| 02:55:27.78 | 1/3 · 1m 39s · $0.14 · 3/10 | ✓ 1 try 1m 08s $0.04 |
| 02:55:31.78 | 1/3 · 1m 43s · $0.15 · 3/10 | |

The run's records agree on every try's cost. T2's last `call_cost.attempt_cost_usd` (0.0396 at 02:55:18.4), its
`attempt_settled` (02:55:26.349) and its verdict's `billed_usd` are all $0.0396. T1's calls went from 0.0851
(02:55:22.4) to 0.0955 (02:55:24.0) to 0.1071 (02:55:30.6).

SPENT is the checkpoint's committed spend plus each open try's latest `attempt_cost_usd` (bug-7a62d9). Subtracting
T1's and T2's live costs from the shown figures puts the committed part near $0.015 at 23.78, near $0.035 at 25.78
(T2 still open), and $0.045 at 27.78 (T2 closed). So in the seconds before T2's verdict, its spend seems to count in
the committed part as well as in its open try's live cost. It drops out of the live part when the verdict closes the
try.

There is also one transient frame at 02:58:45, "3/3 · 4m 57s · $0.17 · 9/10", with no plan check row, just before
"done just now". The plan check had finished and its delivery wasn't yet recorded.

## Why it matters

Goal tooling. SPENT should only grow while a run goes. A figure that falls makes a person wonder what got refunded.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::run_readings`: committed spend plus the open tries' live cost.
- The checkpoint's committed holds (`graph_checkpoint.rs::CommittedHold`), which record each open attempt's finished
  calls, and the order of the try's ledger commit and its verdict. A plan without a ceiling kept no holds
  (`graph_task_dispatch/budget.rs::try_reserve`), so its checkpoint had no record of a try's calls before it settled.
- `crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback`: the try's `cost_usd` row went out after its
  `attempt_settled` and named no try. roko serve's run summary (`routes/runs.rs::summarize_events`) and the portal
  dropped the try's calls at `attempt_settled` and added its cost a moment later (2 ms in 7.4's run).

## Plan

1. Replay the commit sequence around a try's settle (ledger commit, verdict line, `attempt_settled`) in a test, and
   find when the committed spend holds a try that is still open.
2. Count each try once: an open try whose spend the ledger already settled adds nothing live. Or SPENT while running
   is the settled spend plus each open try's latest call, taken from the holds.
3. Fill the plan check's gap: while the delivery is recording a passed check, show the check as passed.

## Done when

A replay of 7.3's commits shows SPENT never falling, the hub's events count a settling try once, and the plan check
reads as it ended until its delivery is recorded; the `[[verify]]` passes.
