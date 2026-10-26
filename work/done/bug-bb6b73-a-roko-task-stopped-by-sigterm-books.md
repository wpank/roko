+++
id = "bug-bb6b73"
kind = "bug"
title = "A Roko task stopped by SIGTERM books an unknown cost at its full reservation even when the proxy shows all its calls unbilled, eating a budget line's headroom"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "bcdc1de57"
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B units roko-fixed-s1-3 and roko-fixed-s2-1"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::run_task"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_a_killed_task_with_only_refused_calls_books_nothing' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider -k 'killed_task_with_only_refused_calls'"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T11:12:13Z"
commit = "bcdc1de57"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T10:26:23Z"
forced = false
evidence = "bcdc1de57: run_task's finally now resolves proxy_rows = _proxy_rows(ctx) and, when it is not None (a proxy log exists, even an empty one for this task), calls the new _meter_killed_attempt(placeholder, proxy_rows, ctx.snapshot), which prices the placeholder the way _meter_from_proxy already prices a normal attempt whose window was all unbilled: $0 (known, provider_usage) when every row is unbilled, else the sum of what was billed, priced by the model that served it; without a proxy log at all the placeholder keeps its full reservation as an unknown cost, unchanged. New test test_a_killed_task_with_only_refused_calls_books_nothing: a fake roko makes one call, gets a 402 through the real metering proxy (StubServer + --proxy), then sleeps 60s; a background thread sends the test process a real SIGTERM after 2.5s; vb.main(['run', ...]) raises SystemExit (secret.tripwire's handler, code 128+SIGTERM) with the previous SIGTERM handler restored afterward. The one ledger row written by run_task's own finally prices at api_equiv_usd 0.0, source provider_usage, billed_usd 0.0, zero usage, and no records.jsonl is ever written for the task (the kill unwinds inside run.runner.run_task before _run_one can build one). Verified red/green by hand: reverting just the finally-block edit made the test fail (api_equiv_usd None), confirming it exercises the bug; restored, it passes. benchmarks/viabilitybench/driver/ (276 tests): 261 passed, 15 skipped (all 'no roko binary at target/debug/roko', the pre-existing real_roko/shakedown skips), 0 failed."
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
