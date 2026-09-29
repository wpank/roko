+++
id = "bug-a49003"
kind = "bug"
title = "A killed subscription session's ledger row still says cli_usage; it needs an estimated cost and a subscription marker"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix3's report on bug-f62293)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/ledger.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["bug-f62293", "gap-c4f364"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_killed_sessions_ledger_row_is_an_estimate' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_a_killed_sessions_ledger_row_is_an_estimate -q"
+++

## Problem

bug-f62293 gave a killed subscription session's run record an honest cost label. The session's ledger row still says `cli_usage`: `Meter.cost()` in `driver/run_cli.py` (:229-235) returns `ledger.Cost(…, "cli_usage")` for every priced session, including one killed before its `result` event, whose cost is only a partial total from the stream (:58).

## Why it matters

Pilot benchmark (epic spec-567e52): the ledger is the spend record the caps and the reconciliation read. A partial estimate labelled as the CLI's own report looks more reliable than it is. p3, because killed sessions are rare.

## Where

- `Meter.cost` in `run_cli.py`.
- `ledger.py`: `Ledger.append` (:274) and `_subscription` (:757), which recognises a subscription row by its $0 bill.

## Current state

At 7fa54b873 the run record and the ledger row disagree about the label.

## Plan

1. When the session was killed, have `Meter.cost` return source `estimated`.
2. Give ledger rows an explicit subscription marker, so the validator's rule that an estimated or provider-billed cost can't bill $0 doesn't reject them, and `_subscription` no longer has to infer it.
3. Add `test_a_killed_sessions_ledger_row_is_an_estimate`.

## Done when

- [ ] A killed session's ledger row and run record carry the same honest label.
- [ ] The `[[verify]]` command passes.
