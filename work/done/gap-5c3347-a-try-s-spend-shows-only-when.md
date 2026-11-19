+++
id = "gap-5c3347"
kind = "gap"
title = "A try's spend shows only when it settles: the page and chat read $0.000 through a 10-20 minute try (costs.jsonl and the run summary get nothing until settle), so nobody sees a try approach its cap"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "d41c377e2"
source = "round-v7-hermes-pilot#C-p0-run1"
discovered_from = "agent:hermes-pilot (round v7 phase 0, run 6988f0b8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs"]
links = { depends_on = [], blocks = [], related = ["gap-6a319c", "bug-f0007b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_running_try_reports_its_spend_per_model_call' crates/ && cargo test -p roko-cli --lib -- a_running_try_reports_its_spend_per_model_call"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T17:15:23Z"
commit = "d41c377e2"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-19T16:22:26Z"
forced = false
evidence = "Each model call that roko's tool loop makes now reaches the hub as a call_cost event when it returns: model, tokens, cost_usd (null when unpriced) and attempt_cost_usd, the try's spend so far. It goes through a metered CallAllowance that wraps the attempt's own allowance on both dispatch paths; PricedCalls passes the model and usage through record_model_call. Serve's run summary adds the spend of tries that have not settled (metrics.live_cost_usd) to cost_usd until each settles. Verify a_running_try_reports_its_spend_per_model_call passes: 3 scripted calls give 3 call_cost events at $1/$2 per million, the total grows with each, and all arrive before attempt_settled. a_running_try_s_spend_reaches_the_run_summary also passes, as do 139 roko-cli, 39 roko-core and 22 roko-serve targeted tests; clippy is clean. In the $0 fixture run t-5c3347 (stand-in coder, 2 s turns), T1's calls landed at +19.98 s, +22.07 s and +24.22 s with $0.00146, $0.00293 and $0.00439 so far, and attempt_settled followed at +26.49 s with $0.00439. Not covered: an agent that runs its own loop (the Claude CLI) reports its spend at the end, as before."
+++

## Problem

A try's spend shows up only when the try settles.

Round v7, Undertow phase 0, run `6988f0b8` (roko 3b34aed92, plugin 0.6.0):
- T1's try ran 9.6 minutes on glm-4.7. Through all of it, the Hermes roko page's "SPEND · THIS RUN" read $0.000 and
  `.roko/learn/costs.jsonl` had no row for the run. The $0.1185 appeared at once when the try settled.
- T3's second try ran 20.8 minutes; its $0.23 also arrived only at settle.
- The chat's `roko_status` at 8 minutes said "no spend reported yet".

## Why it matters

The cap is the person's main guard, and the pitch is "watch it work, with the spend against its cap". A run that
reads $0.000 for 20 minutes, then jumps, shows neither. It also hides a slow, expensive try (bug-f0007b's case)
until it's over.

## Where

- The cost path: per-call usage is known when each model call returns (the tool loop's usage per iteration), but
  the efficiency event and `costs.jsonl` row are written per settled try.
- The page reads the run's spend from those rows and events.

## Plan

1. Publish each model call's usage and cost as it returns: an event such as `call_cost` with run, task, attempt,
   model, tokens and `cost_usd`. Accumulate it into the run's live spend in serve's run summary.
2. Keep the settled try's total as the record of truth.
3. The plugin's meter can then move during a try.

## Done when

During a running try, serve's run summary (and the hub's events) report a spend above $0 that grows with each model
call. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/shots/r01-run-page.png` (SPEND $0.000 at minute 9 of T1's try) and
  `evidence/p0-run1/`.
- Related:
  - gap-6a319c: per-try timing telemetry;
  - bug-f0007b: one slow try eats the cap.
