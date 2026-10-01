+++
id = "bug-f62293"
kind = "bug"
title = "A killed Claude Code session's partial cost is labelled cli_usage, because the schema rejects estimated with $0 billed"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/schema", "benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "c68ffc43e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-ccarm's report on gap-c4f364, merged in 157f1d434)"
anchors = ["benchmarks/viabilitybench/schema/validate.py", "benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/schema/run-record.schema.json"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c4f364", "gap-b24517"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_killed_subscription_session_gets_an_honest_cost_label' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_a_killed_subscription_session_gets_an_honest_cost_label -q"

[closed]
at = 2026-09-29
commit = "c68ffc43e"
by = "wk-bench-fix3"
evidence = "Decided option (a), estimated: records.py labels a CLI session priced from its stream (cli.cost_basis stream, killed before its result event) costs.source estimated, which metrics counts as estimated_cost_runs; validate.py accepts estimated with $0 billed only for a run record whose every attempt carries the CLI runner's cli block, so a billed API run still never bills $0. Verify: test_a_killed_subscription_session_gets_an_honest_cost_label passes (driver/test_run_cli.py 11 passed). Left for run_cli.py's owner: its ledger row still says cli_usage (Meter.cost), since ledger rows carry no subscription mark."
+++

## Problem

When the driver kills a subscription session (on a cap or a timeout), the CLI emits no final `result` event. The only cost left is the partial total from the stream's per-message usage.

`run_cli.py` labels that total `costs.source = "cli_usage"` with `cli.cost_basis = "stream"`: "still the CLI's report, but a partial total that misses background calls" (:50). The honest label would be `estimated`, but `schema/validate.py` (:192) rejects any record whose `billed_usd` is 0 while tokens were used and whose `source` is `provider_usage` or `estimated`. That rule exists for billed API arms (:12). A subscription run always bills $0, so the rule forces `cli_usage` onto a number the CLI never reported.

## Why it matters

Pilot benchmark (epic spec-567e52): readers of the report and of `metrics.py` take `cli_usage` as the CLI's own figure. Under that label, a partial stream total looks more reliable than it is. p3, because killed sessions are rare and `cost_basis` still marks them.

## Where

- `schema/validate.py` (:12, :192): the $0-billed rule.
- `driver/run_cli.py` (:44-50, :548): the cost labels.
- `schema/run-record.schema.json`: the `costs.source` enum.

## Current state

At e43d3a033 a killed session's record passes validation only under `cli_usage`.

## Plan

Decide the label. There are three options:

- **(a)** Allow `estimated` with $0 billed when the record comes from a subscription CLI (it has a `cli` block, or `billed` is false).
- **(b)** Add a source such as `cli_partial`. This changes the schema enum and needs S01 alignment.
- **(c)** Keep `cli_usage`, and have the analysis treat `cost_basis = "stream"` as an estimate.

Then document the choice in run_cli.py and the schema, and add `test_a_killed_subscription_session_gets_an_honest_cost_label`.

## Done when

- [ ] A killed subscription session's cost carries a label that says it is partial, and validation accepts it.
- [ ] The `[[verify]]` command passes.

## Notes

- Keep the rule's purpose: a billed API run that used tokens must never record $0.
