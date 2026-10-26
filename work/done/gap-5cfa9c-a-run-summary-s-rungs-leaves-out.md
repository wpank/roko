+++
id = "gap-5cfa9c"
kind = "gap"
title = "A run summary's rungs[] leaves out the calls a rung's model made before failover refused them, so the rungs don't add up to cost_usd (run 7b8cb121: $0.7009 on rungs, $0.9085 cost_usd; mid's glm-4.7 spent $0.2076 in calls refused before T2's failover)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-learn/telemetry"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "d653565eb"
source = "round-v7 guide-v7 via the lead (run 7b8cb121's summary)"
discovered_from = "agent:guide-v7 (round v7)"
anchors = ["crates/roko-learn/src/telemetry/rung_stats.rs::RungReport", "crates/roko-serve/src/routes/runs.rs::summarize_run"]
links = { depends_on = [], blocks = [], related = ["bug-11e54b", "gap-1430ea", "gap-3217a0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_rungs_spend_counts_its_models_refused_calls' crates/roko-learn/src && cargo test -p roko-learn --lib -- a_rungs_spend_counts_its_models_refused_calls"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T11:31:18Z"
commit = "d653565eb"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-21T10:44:39Z"
forced = false
evidence = "d653565eb. roko-learn telemetry/rung_stats: RungRow::refused_usd, what the calls a rung's model made and its provider refused before failover moved their tries on cost; refused_calls reads the role failover_refused cost rows of the runs read from learn/costs.jsonl (attempt key under the run, model, cost), and add_refused counts each on the row of its try's role and its model (the row with a rung and the most tries, else a new row with no rung); a try's own cost_usd is unchanged. RungReport::of_with_costs reads the costs, of keeps the old readout; RungRow::line adds \"$x in calls refused before failover\". The run summary's rungs (serve summarize_run), GET /api/learn/rungs, roko learn rungs and the end of roko plan run read learn/costs.jsonl. Run 7b8cb121's records hold exactly one failover_refused row, $0.2076 of glm-4.7 for T2's try, which now counts on mid. Tests: a_rungs_spend_counts_its_models_refused_calls (7b8cb121's shape: mid glm-4.7 cost 0.6655 plus refused 0.2076, strong gpt-5.4-mini 0.0354, a refused model with no try on a row of its own; another run's rows and a helper row are not counted; without the costs file the readout is as before), run_summary_lists_its_rungs (the summary's mid row carries refused_usd 0.2076, strong 0.0). On main 85e89014b's tree plus the change: fmt and clippy -D warnings clean; lib suites roko-agent 2397, roko-cli 3993, roko-core 2047, roko-learn 1395, roko-runtime 213, roko-serve 1081, all passing."
+++

## Problem

A run summary's `rungs[]` doesn't add up to its `cost_usd`: the calls a rung's model made before failover refused
them are on no rung. guide-v7 noticed it in run 7b8cb121:

| | USD |
|---|---|
| mid, glm-4.7 (`rungs[]`) | 0.6655 |
| strong, gpt-5.4-mini (`rungs[]`) | 0.0354 |
| **rungs together** | **0.7009** |
| refused glm-4.7 calls before T2's failover (`side_call_usd.failover_refused`) | 0.2076 |
| `cost_usd` | 0.9085 |

mid's model spent the $0.2076 on mid's try, but `rungs[]` counts only each settled try's own cost, so mid looks
$0.21 cheaper than it was. plugin-wow's ladder panel reads `rungs[]`.

## Where

- `crates/roko-learn/src/telemetry/rung_stats.rs::RungReport::of`: a row's `cost_usd` is the sum of its settled
  tries' costs, from their `attempt_settled` figures.
- A refused call's cost is in its side-call cost row and its `efficiency_event` (`side_call_usd.failover_refused`,
  with the try's `attempt`, gap-1430ea). The try's verdict names the refused model in
  `executed.failover_refusals` (`called: true`).
- `crates/roko-serve/src/routes/runs.rs::summarize_run` puts the report's rows in the summary as `rungs`.

## Plan

Count a refused call's spend on the row of the rung and model that made it, as a separate `refused_usd`, so a try's
own cost stays its own. The rungs' `cost_usd` plus `refused_usd`, plus the other side calls, then equal the run's
`cost_usd`. Tell plugin-wow the field when it lands.

## Done when

Run 7b8cb121's records give mid a `refused_usd` of $0.2076, and the rungs plus the other side calls equal `cost_usd`.
Checked by the test in `[[verify]]`.
