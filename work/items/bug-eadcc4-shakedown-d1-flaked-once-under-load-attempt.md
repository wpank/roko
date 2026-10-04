+++
id = "bug-eadcc4"
kind = "bug"
title = "Shakedown D1 flaked once under load: attempt 2's proxy row likely misattributed to attempt 1's window"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-14 follow-up reports 2026-10-04 (gate 14b full bench run)"
discovered_from = "gate 14b bench run (flaked once; passed alone and at gates 13c/14a/15a)"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::settle", "benchmarks/viabilitybench/driver/test_shakedown.py::test_shakedown_d1_blank_answer_does_not_isolate_the_task"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_delayed_episode_write_does_not_misattribute_a_later_proxy_row' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -k test_delayed_episode_write_does_not_misattribute_a_later_proxy_row"
+++

## Problem

Shakedown D1 (`test_shakedown_d1_blank_answer_does_not_isolate_the_task`) flaked once in gate
14b's full bench run: attempt 2 (the retry that should solve the task after attempt 1's blank
answer) came back `no_proxy_traffic: attempt 2: the metering proxy saw no request`, turning the
task `infra_error` instead of `completed`. It passed 3/3 run alone, and passed the full-suite
runs at gates 13c, 14a and 15a — a genuine, load-correlated race, not a deterministic bug.

The attempt-to-proxy-row matching this check depends on
(`benchmarks/viabilitybench/driver/run_roko.py:757-813`) assigns each proxy row to the first
attempt whose own *episode* timestamp (`stamps = [episode.get("completed_at") or
episode.get("timestamp") ...]`, line 759; `ends = [clock(stamp) for stamp in stamps]`, line 762)
is at or after the row's own `ts`. `windows[1]` (attempt 2's) is empty, and the attempt isn't
exempted by the same-end shortcut (`ends[position] != ends[position - 1]`, line 811), so the
flag fires. Two concrete mechanisms could produce that, both consistent with "a race, worse
under load":

1. **Episode-write delay, not request delay.** `ends[0]` is attempt 1's *episode* timestamp —
   stamped when Roko finishes writing the episode, which the module's own docs say waits for
   helper calls first ("The attempt waits for them before it writes its episode," line 76). If
   that write is delayed under load (disk I/O contention, scheduling pressure from a full bench
   run's many parallel processes), `ends[0]` can land *after* attempt 2's real network request
   already happened and was logged by the proxy — misattributing that row into attempt 1's
   window (which already has its own row) and leaving attempt 2's window empty, even though the
   real-world order was attempt 1 finished, then attempt 2 ran.
2. **Silent precision downgrade.** `clock` is `_instant` (microsecond compare, the fix from
   `bug-09fac4`) only `if all(_subsecond(stamp) ...)` across *every* episode and proxy
   timestamp for the task (line 761); `_subsecond` is a bare string check for a `.` in the ISO
   time (line 1070-1072). If any single stamp in this task happens to serialize with no
   fractional-second component (a timestamp landing exactly on a whole second, or a formatter
   edge case), the *entire* comparison silently downgrades to whole-second granularity
   (`_second`, line 1060), reviving the same-second ambiguity `bug-09fac4` fixed — but only for
   that one unlucky task.

## Why it matters

Severity p2: the shakedown is the instrument check run before every paid real-model bench pass
(`PK36`'s note in the project memory: "the instrument check before any paid Roko-arm run"). A
flake here that isn't understood risks being misread as a real regression during a paid run, or
worse, masking one — "it's probably just the known flake" becomes an excuse to ignore a real
`no_proxy_traffic` failure later.

## Where

- `benchmarks/viabilitybench/driver/run_roko.py:741-813` (`settle`'s window-matching: `stamps`,
  `ends`, `clock`, `_subsecond`, the `no_proxy_traffic` flag).
- `benchmarks/viabilitybench/driver/test_shakedown.py::test_shakedown_d1_blank_answer_does_not_isolate_the_task`
  (the flaking test).
- `work/done/bug-09fac4-...md` (the related, already-fixed same-second ambiguity; read first,
  since this may be a residual case of the same family rather than a new mechanism).

## Current state

Flaked once under load (gate 14b's full bench run); not reproduced in isolation or in three
other full-suite runs. No diagnostic captures the raw `ends[]`/proxy-row `ts` values when
`no_proxy_traffic` fires, so there is no data yet to confirm which of the two hypotheses above
(or another) actually happened.

## Plan

1. When `no_proxy_traffic` fires, log (or attach to the flag) the attempt's own episode
   timestamp, the neighboring attempts' episode timestamps, and every proxy row's raw `ts` for
   the task — enough to tell, after the fact, whether a row was misattributed to the wrong
   window or truly missing.
2. Re-run the full bench suite enough times under comparable load to try to reproduce; if
   reproduced, use the new diagnostics to confirm which mechanism (episode-write delay vs.
   precision downgrade) is responsible.
3. Fix accordingly: either widen the matching window's tolerance (e.g. prefer the proxy row's
   own `ts` ordering over episode-write completion time), or make `_subsecond` enforcement per
   missing-precision stamp rather than all-or-nothing for the task, whichever the data points
   to.

## Done when

- The mechanism is confirmed (not just hypothesized), and either fixed or shown to be
  acceptably rare with a documented reason it can't fully be eliminated.
- A regression test exercises the confirmed mechanism directly (e.g. an episode timestamp
  deliberately delayed past a later proxy row's `ts`) rather than relying on reproducing the
  flake under real load.

## Notes

- 2026-10-04 (wave-14/gate-14b follow-up, bench shakedown flake): confirmed at main HEAD
  `deba5c6f8` by reading the exact matching algorithm the failure message comes from. Not
  reproduced; this item records the mechanism analysis so a future occurrence (or a deliberate
  repro attempt) has concrete hypotheses to check against, per the diagnostics in the Plan.
