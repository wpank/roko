+++
id = "gap-4e8795"
kind = "gap"
title = "flaky_verify needs a visible-verify wrapper, and no arm has one"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver", "benchmarks/viabilitybench/arms"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "535783782"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/disturb.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-15bb83"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_flaky_verify_is_injected_through_the_visible_verify_wrapper' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_flaky_verify_is_injected_through_the_visible_verify_wrapper -q"

[closed]
at = 2026-09-30
commit = "535783782"
by = "wk-bench-fix1"
evidence = "535783782: a run with flaky_verify installs the visible-verify wrapper (driver/vb_verify.py) in each task's .vb-bin/, and every arm's visible checks go through it: Roko's verify step and rung (`vb-verify '<check>'`, named by PATH), the direct loop (every agent command), and Claude Code (CLAUDE_CODE_SHELL_PREFIX). A run of the check fails as a killed check (137, 'Killed') with the spec's seeded p (HMAC of seed and key:n); p = 0 off the covered positions. Records carry visible.verify_runs, visible.flakes and visible.flake_injected (schema updated); the census's rerun never meets a flake. Verify passes (test_flaky_verify_is_injected_through_the_visible_verify_wrapper: direct arm, p = 1 at position 2, flakes [1, 2] there and none at position 1, both VS = 1). Also green: the real roko binary fails all 3 attempts' gates on flakes (test_real_roko_gate_meets_the_flaky_verify_wrapper), a fake claude's prefixed shell meets the flake at p = 1 and passes at p = 0, and 14 wrapper unit tests. Full bench suite: 365 passed, 2 skipped. The Claude Code prefix form is read from the 2.1.282 bundle; no live session has run it."
+++

## Problem

The `flaky_verify` disturbance (`VB_FLAKE_P`) makes the visible verify fail at random. It needs a wrapper around each arm's visible-verify command to inject the failures, and no arm has one (`driver/disturb.py:34`, :63), so the disturbance is refused.

## Why it matters

Pilot benchmark (epic spec-567e52): H6 can't test how the arms handle a flaky check, one of the disturbances real projects produce most. p3.

## Where

`disturb.py`, and how each arm runs its visible verify (the direct loop, the Claude Code arm and the Roko arm's emitted steps).

## Plan

1. Add a visible-verify wrapper (`vb-verify <command>`) that each arm's visible checks go through, and that fails with probability `VB_FLAKE_P` under the disturbance.
2. Record every injected flake in the run record.
3. Add `test_flaky_verify_is_injected_through_the_visible_verify_wrapper`.

## Done when

- [ ] `flaky_verify` runs on every arm, and the records show the injected failures.
- [ ] The `[[verify]]` command passes.
