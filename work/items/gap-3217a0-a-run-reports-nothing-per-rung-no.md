+++
id = "gap-3217a0"
kind = "gap"
title = "A run reports nothing per rung: no tries, passes, time or cost per rung and model, so a slow or misordered ladder shows only in a by-hand analysis (Undertow mid: 7-46 min tries, $0.198 per verified pass vs strong $0.121)"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/routes"]
created = 2026-10-20
updated = 2026-10-20
source = "round-v7-roko-speed#design-note-rungs"
discovered_from = "agent:roko-speed (round v7 design note, approved by the lead 2026-10-20)"
anchors = ["crates/roko-learn/src/telemetry/report.rs::RunRecords", "crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord", "crates/roko-serve/src/routes/runs.rs::summarize_run", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
links = { depends_on = [], blocks = [], related = ["gap-74893d", "bug-f0007b", "gap-6a319c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rung_rows_measure_each_rung_and_flag_a_misordered_ladder' crates/roko-learn/src && cargo test -p roko-learn --lib rung_rows_measure_each_rung_and_flag_a_misordered_ladder && cargo test -p roko-serve --lib run_summary_lists_its_rungs"
+++

## Problem

A run's records say, try by try, which rung and model ran, how it ended, what it cost and how long it took
(`roko.verdict/1`). Nothing adds them up per rung. So a person can't see that a rung is slow or misordered:
- In Undertow's phase 1 (run `0bed5c58`), glm-4.7 on mid took 7 to 46 minutes a try, 96–99% of it model time,
  while gpt-5.4-mini on strong passed in 44 s for $0.02.
- Over Undertow's four runs, mid cost $0.198 and 1,013 s per verified pass, and strong $0.121 and 209 s.
  hermes-pilot had to compute both from the records by hand (`$V7/pilot/tooling/trytimes.py`).

The closest readouts aren't per rung:
- `GET /api/learn/model-scorecard` gives one row per model, merged from `efficiency.jsonl`, a per-model-call latency
  registry and the router's counts. It has no rung, no time per try and no cost per verified pass.
- `roko learn telemetry route-report` counts pass and fail per decision source.
- `roko learn self-model fit` reports calibration only.

A verdict also doesn't say that its try stopped at a rung's bound; only the `attempt_settled` event does.

## Why it matters

The ladder is roko's main lever for "cheap models first, climb when the checks fail". A rung that is slower and
dearer than the one above it wastes the person's time and money on every task, and only a by-hand analysis shows it
today. The lead wants to decide Undertow's ladder order for phase 3 from phase 2's numbers.

## Where

- `crates/roko-learn/src/telemetry/`: `RunRecords` reads a run's records. The new per-rung rows go next to the
  route report.
- `crates/roko-learn/src/telemetry/records.rs`: `AttemptVerdictRecord`, which gains `bound`.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs`: `settle` writes the verdict.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the end-of-run summary.
- `crates/roko-serve/src/routes/runs.rs`: `summarize_run`.
- `crates/roko-serve/src/routes/learning/`: the learn routes.

## Plan

1. **Per-rung rows** from a run's verdicts (and its open lines for the role), one per (role, rung, model):
   - tries, verified passes (learning label 1), agent failures, other, and tries that stopped at their time bound;
   - median and p90 wall time per try;
   - median time per verified pass (the time the task spent on the rung to reach the pass);
   - model-time share, from `timing.phases`;
   - cost per try, cost per verified pass and total cost.
2. **A misordered-ladder line.** When a higher rung has both lower cost per verified pass and lower median time per
   verified pass than a lower rung, and each has at least 3 verified passes, say so in one line with the numbers.
3. **`rung_slow`, a live hub event,** once per run per rung. It fires when a try reaches its time bound, or when the
   rung's tries average more than `[routing.ladder] slow_call_secs` (default 60) per model call over at least 5 calls.
4. **Where it shows:**
   - `GET /api/runs/{id}/summary` gets `rungs`, `misordered` and `rung_slow`;
   - `GET /api/learn/rungs` covers the workspace's runs;
   - `roko learn rungs` prints the table;
   - `roko plan run` ends with its rows, in text and JSON.
5. **`bound` in `roko.verdict/1`:** `turns`, `spend` or `time` for a try that ended at its rung's bound.

## Done when

A run's summary lists its rungs with those numbers, flags a misordered ladder, and names the rungs that ran slow. A
$0 run with a slow, time-bounded rung shows all three. Checked by the tests in `[[verify]]`.

## Notes

- From roko-speed's design note in `tmp/hermes-roko-v7/findings/roko-speed.md` (2026-10-20). The lead approved points
  1+2 as one gap, with the misordered line and `rung_slow` added.
- The self-model's start rule weighing latency stays out of this item: gap-74893d, after phase 2's data.
- Phase 2 of Undertow is the live test: its summary with `rungs[]` and any `rung_slow`, next to hermes-pilot's table.
- **One readout, not a fourth** (the lead's condition, 2026-10-20). `roko learn rungs` and `GET /api/learn/rungs` are
  the per-try readout, built from the verdicts. Here is how the other two relate to it:
  - `roko learn telemetry route-report` stays as it is. It audits whether routing is logged honestly (decision
    sources, ι, ε_honest), which is a different question.
  - `GET /api/learn/model-scorecard` overlaps. Its tries, pass rate and cost per model come from `efficiency.jsonl`,
    which has one or more rows per try. Those columns should come from this item's fold, summed per model. Its
    per-call latency (TTFT, tokens per second) and the cascade router's trials stay, since nothing else measures
    them. That folding is gap-c0fe41.
- **Which runs serve's summary covers.** It reads a run's records under the run's own id. That is where a run that
  serve started keeps them, which is the Hermes path. A CLI `plan run` keeps them under its plans' run ids
  (`graph-<plan>-<uuid>`), which its event index doesn't name, so serve's summary of a CLI run has no rows. The CLI
  prints them itself at the end of the run, and `GET /api/learn/rungs` and `roko learn rungs` read every run.
