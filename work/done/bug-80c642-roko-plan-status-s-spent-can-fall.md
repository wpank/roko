+++
id = "bug-80c642"
kind = "bug"
title = "roko plan status's SPENT can fall mid-run: around a try's settle its spend counts in the committed ledger and in its open try's live cost, then once (7.3: $0.17 then $0.14 as task 2 settled); and a frame between the plan check's end and its delivery record shows no plan check row"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-18
updated = 2026-11-18
last_verified = 2026-11-18
last_verified_rev = "ad8558c15"
source = "round v8 guide-v8: phase 7.3's terminal capture (run 0d8e8a2c), frames 02:55:21-02:55:31 UTC and 02:58:45"
discovered_from = "agent:guide-v8 (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::RunReadings", "crates/roko-cli/src/graph_checkpoint.rs::CommittedHold", "crates/roko-cli/src/graph_task_dispatch/budget.rs::AttemptHold", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-serve/src/routes/runs.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7a62d9", "gap-9bfb4e", "bug-addd24"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_spent_never_falls_as_a_try_settles' crates/roko-cli/src && grep -rqw 'fn a_settling_try_counts_once' crates/roko-serve/src && cargo test -p roko-cli --lib -- plan_status_spent_never_falls_as_a_try_settles plan_status_shows_what_is_running a_plan_without_a_ceiling_commits_its_attempts_spend_too a_running_try_reports_its_spend_per_model_call && cargo test -p roko-serve --lib -- a_settling_try_counts_once"

[closed]
at = 2026-11-18
at_ts = "2026-11-18T11:56:15Z"
commit = "ad8558c15"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-18T10:57:09Z"
forced = false
evidence = "ad8558c15 on work/bug-80c642: a settling try counts once for every reader of its records. Terminal: roko plan status (and roko diagnose's header) read SPENT from one checkpoint commit, the settled spend plus each hold's finished calls (CheckpointInspection.held_micro_usd), not the settled spend plus each open try's latest call_cost: the cost ledger settles a try as its agent's call ends, before its checks and its verdict line, so the try counted twice in between (7.3: T2 for 0.32 s at $0.17 for $0.14, T1 for 0.28 s at $0.29 for $0.17, T3 for 0.37 s at $0.18 for $0.17). Ledger: a plan without a ceiling now keeps holds that reserve nothing (AttemptHold.tracks_only), so its checkpoint commits each try's calls too and a resume settles them; their allowance refuses no call and sets no limit. Hub: emit_feedback publishes the try's token usage and cost row, now naming its attempt, before its attempt_settled (before, serve's summary and the portal dropped the try's calls at attempt_settled and added its cost 2 ms later on 7.4), and roko serve's run summary counts the try's cost in place of its calls from that row. Plan check gap (7.3's 02:58:45 frame): the event index's last plan_check_finished gives the plan check row and its part of CHECKS until the delivery is recorded. Red first: plan_status_spent_never_falls_as_a_try_settles read $0.16 $0.18 $0.21 $0.18 $0.18; a_plan_without_a_ceiling_commits_its_attempts_spend_too found no allowance; a_running_try_reports_its_spend_per_model_call's cost row named no try; a_settling_try_counts_once read 0.05 0.08 0.05; plan_status_shows_what_is_running had no plan check row once the check ended. Replay of 7.3's own commits on an APFS clone of ~/dev/undertow (tmp/hermes-roko-v8/roko-recovery/repro/replay_73.py, replay-73-item17.txt): roko-item15b falls 3 times in 52 steps, roko-item17 never; gap_73.py at 02:58:46: 9/10 checks and no row before, the plan check row (tools/check.sh --plugin, 1m 16s) and 10/10 after. Branch checks: nightly fmt; roko-cli lib 4163 passed, bin 484 passed; roko-serve lib 1104 passed; 15 roko-cli integration test files that run Graph plans with budgets and checkpoints passed; workspace clippy -D warnings clean; binary bin/roko-item17. The portal needs no change; Hermes' 7.4 dip is the plugin's own run_spend fallback double count, sent to plugin-dash-v8."
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
