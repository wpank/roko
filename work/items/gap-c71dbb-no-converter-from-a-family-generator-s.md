+++
id = "gap-c71dbb"
kind = "gap"
title = "No converter from a family generator's output to the manipulation-check module's [[task]] shape"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK24 gap-e120a1)"
discovered_from = "gap-e120a1 (backlog task 3234)"
anchors = ["tmp/backlog/2026-10-02-complete-and-wire/3234-manipulation-check-vague-30-below-precise.md", "benchmarks/viabilitybench/specops/manipulation_check.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_full_48_instance_h3_report_runs_from_generators' benchmarks/viabilitybench/specops/tests/test_manipulation.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py -k test_full_48_instance_h3_report_runs_from_generators -q"
+++

## Problem

`gap-e120a1`'s own Progress notes (PK24, done) say verbatim: a manipulation-check gap test "passes on the four
3233 fixtures: gaps of 62-68 points, far past the 30-point bar. The full 48-instance H3 report needs a converter
from each family's own `gen.py`/`render_spec` into this module's `[[task]]` shape, and was not built here"
(mirrored in backlog task 3234, whose own text defers the full 48-instance pass "once S08.T8 and S08.T9 land").
No converter from a ViabilityBench family generator's output (`gen.py`/`render_spec`) to `specops/manipulation_check.py`'s
(or whatever module hosts the `[[task]]` shape) expected input exists yet.

## Why it matters

Without the converter, H3 (the manipulation-check hypothesis) can only be evaluated on the 4 hand-built 3233
fixtures, not the full 48-instance set the spec calls for — the headline H3 report S08/S09 depend on cannot be
produced end to end.

## Where

`benchmarks/viabilitybench/` family generators (`gen.py`, `render_spec` — F1 and F4 exist per earlier research;
F2, F3, F5 were noted as not yet built in task 3234's own text); the manipulation-check module task 3234/PK24
delivered, consuming a `[[task]]`-shaped input.

## Current state

4 fixtures pass by hand. No generator-to-`[[task]]` converter exists; S08.T8/S08.T9 (not yet confirmed landed) are
named as the tasks the full report depends on.

## Plan

1. Confirm whether S08.T8/S08.T9 have backlog task numbers and their current status.
2. Write the converter: each family's `gen.py`/`render_spec` output mapped into the manipulation-check module's
   `[[task]]` shape.
3. Run the full 48-instance H3 report once the converter and the remaining family generators (F2, F3, F5, if still
   missing) are in place.

## Done when

- The 48-instance H3 report runs end to end from family generator output, not hand-built fixtures.
- The `[[verify]]` command passes.

## Notes

- Depends on F2/F3/F5 family generators landing if they haven't (check `benchmarks/viabilitybench/families/` for
  their current state before starting).
