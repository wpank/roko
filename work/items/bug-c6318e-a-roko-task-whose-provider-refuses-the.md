+++
id = "bug-c6318e"
kind = "bug"
title = "A Roko task whose provider refuses the account (HTTP 402 payment required) is recorded as Roko's own failure, not infra_error"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B unit roko-fixed-s1-2, Cerebras 402 from 15:17Z"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::account_refusals", "benchmarks/viabilitybench/driver/test_run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_a_task_the_provider_refused_for_the_account_is_an_infra_error' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider -k 'refused_for_the_account'"
+++

## Problem

When the provider refuses the account (HTTP 402 payment required, 401 bad key), the Roko arm records the task as
Roko's own `failed`, not `infra_error`. Roko reports the refusal as a failed attempt ("provider: network error: http
402 ..."), and `run_roko._status` passes a failed checkpoint through as `failed`.

Seen in Pilot B (gap-327242) on 2026-10-20. Cerebras answered every call from 15:17:44Z on with `402 {"message":
"Payment required to access this resource. Visit your billing tab.", "type": "payment_required_error", "param":
"quota", "code": "payment_required"}`. The proxy forwarded 25 such calls. The last 9 tasks of unit `roko-fixed-s1-2`
(F4-l4-0001 onwards) were recorded `failed`, 8 of them after 0 turns.

## Why it matters

A `failed` record is a VS = 0 outcome of the arm; an `infra_error` record is excluded and counted (S05 §0, S09 §4.7
G2: re-run infra errors). A provider account problem would therefore lower the Roko arm's VS rate and pass^k, and
G2's infra-error share would hide it.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py`: `run_task` and `_status`.
- The metering proxy's log (`proxy.jsonl`): each forwarded call's `status`.

## Current state

Fixed on `work/gap-327242`. `account_refusals(evidence.proxy_rows)` names the task's forwarded calls that got 401
or 402 (`ACCOUNT_STATUSES`). A task that then failed or timed out becomes `infra_error`, with the reason
`provider_account: the provider refused N call(s) with HTTP 402`. A task that completed keeps its outcome: a routed
arm's ladder drops a refused rung and can still pass (the shakedown's D7, which a first version of this fix broke).
Injected faults and the proxy's own refusals are never forwarded, so they do not count.

## Plan

1. Done: classify on the proxy's forwarded statuses, with an end-to-end test (a fake roko through the proxy against
   a stub that answers 402).
2. Not done: the direct loop and the other runners. Check whether `mini_loop` and `run_msa` already end such a task
   `infra_error`. `run_cli` has no proxy: the Claude Code arm's account errors arrive as the `result` event's
   `is_error`, which `_status` already makes `infra_error`.

## Done when

- [x] A Roko task whose provider answers 402 is `infra_error`: the `[[verify]]` test.

## Notes

- Pilot B's unit `roko-fixed-s1-2` ran before this fix, so its 9 records stay `failed`. It is to be moved to
  `PILOT-B.abandoned/` and seed 1 re-run once Cerebras answers again (campaign redo: see the companion item).
