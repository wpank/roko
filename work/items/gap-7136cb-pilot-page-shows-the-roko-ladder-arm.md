+++
id = "gap-7136cb"
kind = "gap"
title = "Pilot page shows the roko_ladder arm and the V7 bar, task 3316"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 3316 (blocked in wave 7, PK36)"
discovered_from = "gap-31c0e8"
anchors = ["benchmarks/viabilitybench/analysis"]
lane = "bench"
links = { depends_on = ["gap-d9e9fe"], blocks = [], related = ["gap-31c0e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pilot_page_shows_the_ladder_arm_and_the_v7_bar' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_pilot_page_shows_the_ladder_arm_and_the_v7_bar -q"
+++

## Problem

Task 3316 of PK36 (gap-31c0e8, merged in wave 7) makes the pilot page show the `roko_ladder` arm beside the other
arms, with the ratios the V7 claim is about. Its premise doesn't hold yet: the pilot page itself
(`analysis/pilot_page.py`, `report.py --pilot`) is gap-d9e9fe's deliverable, and gap-d9e9fe is open.

## Why it matters

Pilot C's result (3315) is read on this page; without the ladder arm on it the V7 comparison isn't visible.

## Where

`benchmarks/viabilitybench/analysis/pilot_page.py` and `report.py --pilot` (gap-d9e9fe), and
`analysis/test_analysis.py`.

## Current state

Neither file exists at main after wave 7. PK36's worker stopped at this task.

## Plan

1. Once gap-d9e9fe lands the pilot page, add the ladder arm and the V7 bar as task 3316 says, with the test the
   verify names.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3316-pilot-page-shows-ladder-arm-and-v7-bar.md`.
- It doesn't need 3315's data: the test can use a fixture bundle.
