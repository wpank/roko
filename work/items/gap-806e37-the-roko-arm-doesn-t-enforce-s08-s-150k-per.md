+++
id = "gap-806e37"
kind = "gap"
title = "The Roko arm doesn't enforce S08's 150K per-attempt input cap, so S08-SC6 can't hold for that arm"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/arms", "benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-appAB's report)"
anchors = ["benchmarks/viabilitybench/arms/roko_fixed.toml", "benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/faultproxy.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-b7ab99", "bug-c30764", "bug-09fac4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_roko_arm_enforces_the_per_attempt_input_cap' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_the_roko_arm_enforces_the_per_attempt_input_cap -q"
+++

## Problem

S08 §4.10 caps each attempt at 150K input tokens and 12 turns. `arms/roko_fixed.toml` records the cap and says it isn't enforced: `input_tokens_per_attempt = 150000  # not enforced: Roko has no such cap …` (:18). Only the metering proxy's per-task cap of 450K (:20) applies, so one Roko attempt can use far more than 150K input, and S08-SC6 can't hold for this arm.

## Why it matters

Pilot benchmark (epic spec-567e52): the arms must run under the same caps, or cost and success comparisons are unfair. The direct arms enforce per-attempt caps; the Roko arm doesn't.

## Where

The arm file, `run_roko.py` (which passes `max_turns` but no input cap to Roko, :204), and the proxy, which knows the task but not the attempt.

## Plan

Pick one:

- **(a) Roko enforces it.** Add a per-attempt input-token cap to Roko's dispatch (a tier or pipeline setting), and set it from the arm.
- **(b) The proxy enforces it.** Tag proxy traffic with the attempt (bug-09fac4's attempt tagging), and have the proxy refuse the attempt's calls past 150K.

Record which one S08's SC6 relies on. Then add `test_the_roko_arm_enforces_the_per_attempt_input_cap`.

## Done when

- [ ] No Roko attempt's input exceeds 150K tokens, and the record shows the cap.
- [ ] The `[[verify]]` command passes.
