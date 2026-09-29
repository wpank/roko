+++
id = "bug-09fac4"
kind = "bug"
title = "The metering proxy stamps whole seconds, so an attempt that ends in the same second as the one before gets no usage"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix1's report)"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/faultproxy.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e90ebd", "bug-62e3f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_attempts_ending_in_the_same_second_get_their_own_usage' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_attempts_ending_in_the_same_second_get_their_own_usage -q"
+++

## Problem

`driver/run_roko.py` assigns the proxy's request rows to attempts by `ts`, which is in whole seconds, in `ordinal` order. An attempt owns the requests up to its episode's end (:57-59). An attempt that ended in the same second as the one before can't be told apart from it: its requests count toward the earlier attempt, and its own window is empty but isn't flagged (:395-396). That attempt's usage stays unknown, so the task's attempt-level cost is null, even though the proxy metered every call. wk-bench-fix1 made the proxy cross-check fill the task total, so what remains is the per-attempt attribution.

## Why it matters

Pilot benchmark (epic spec-567e52): per-attempt usage and cost feed the retry analysis. p3, because fast consecutive attempts are uncommon.

## Where

- The attempt-window assignment in `run_roko.py`.
- The proxy's row stamps in `faultproxy.py`.

## Plan

1. Tag proxy rows with the attempt, not just the task: `configure(task=…, attempt=…)` whenever Roko starts an attempt, if the runner can observe that. Or stamp rows with sub-second time and use Roko's own sub-second episode times.
2. Add `test_attempts_ending_in_the_same_second_get_their_own_usage`.

## Done when

- [ ] Every attempt gets its own metered usage, whatever its timing.
- [ ] The `[[verify]]` command passes.
