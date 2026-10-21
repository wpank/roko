+++
id = "bug-bb6b73"
kind = "bug"
title = "A Roko task stopped by SIGTERM books an unknown cost at its full reservation even when the proxy shows all its calls unbilled, eating a budget line's headroom"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B units roko-fixed-s1-3 and roko-fixed-s2-1"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::run_task"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_a_killed_task_with_only_refused_calls_books_nothing' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider -k 'killed_task_with_only_refused_calls'"
+++

## Problem

When `vb run` is stopped with SIGTERM while a Roko task is running, `run_roko.run_task` unwinds before
`read_evidence`. Its `finally` then writes one attempt with `usage_unknown` (`ended_by = "no_attempt_record"`), and
that ledger row's unknown cost counts at its full reservation, the task's worst case of $0.434 on roko_fixed. This
happens even when the metering proxy's log shows that every call of the task was refused, and so cost nothing.

Seen in Pilot B (gap-327242) on 2026-10-20. Two units were stopped while Cerebras answered 402 to every call:
`roko-fixed-s1-3` and `roko-fixed-s2-1`, both now in `PILOT-B.abandoned/`. Each left one such row, so `vb ledger
report` counts $0.868 of BL0 and of the pilot cap that was never billed: Pilot B's books hold $1.79, while its real
billing is $0.92.

## Why it matters

The bound is safe, but it eats a line's headroom with money nobody spent. A stopped run is how an operator meets a
provider outage, so this will happen again.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py::run_task`, in its `finally` (`dispatched and not attempts`).
- `_proxy_rows(ctx)` / `_meter_from_proxy`: the proxy's rows for the task are on disk even after a kill.

## Current state

A killed task's attempt is priced as unknown, so it counts at its reservation.

## Plan

1. In `run_task`'s `finally`, when no attempt was recorded and the proxy metered the task, price the placeholder
   attempt from the task's proxy rows: $0 when every row is unbilled, else their sum. Keep the reservation bound
   only when there is no proxy log.
2. A test: a fake roko that sleeps after one 402, killed with SIGTERM, leaves a $0 row.

## Done when

- [ ] A Roko task killed after only refused calls books $0: the `[[verify]]` test.

## Notes

- Found by gap-327242's worker on 2026-10-20.
