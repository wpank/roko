+++
id = "gap-c71dbb"
kind = "gap"
title = "No converter from a family generator's output to the manipulation-check module's [[task]] shape"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-6 follow-up reports 2026-10-03 (PK24 gap-e120a1)"
discovered_from = "gap-e120a1 (backlog task 3234)"
anchors = ["benchmarks/viabilitybench/specops/manipulation_check.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-46fd19", "gap-76fd1c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_full_48_instance_h3_report_runs_from_generators' benchmarks/viabilitybench/specops/tests/test_manipulation.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py -k test_full_48_instance_h3_report_runs_from_generators -q"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:47Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T09:03:20Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes. specops/from_generator.py converts F1's generator output to the manipulation check's [[task]] shape; the other families' coverage is gap-76fd1c."
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
delivered, consuming a `[[task]]`-shaped input. `benchmarks/viabilitybench/specops/from_generator.py` (new,
gap-c71dbb) is the converter, for F1 only — see its Progress note below for why the other five H3 families
don't convert yet.

## Current state

4 fixtures pass by hand, and so does one real, freshly generated F1 instance, converted by
`specops/from_generator.py` and scored by the real `manipulation_check.check_pair`/`check_many` (gap-c71dbb).
No converter covers F2, F3, F4, F5 or F7 yet (see Progress). S08.T8/S08.T9 (still not confirmed landed under
those names) are named as the tasks the full 48-instance report depends on.

## Plan

1. Confirm whether S08.T8/S08.T9 have backlog task numbers and their current status.
2. Write the converter: each family's `gen.py`/`render_spec` output mapped into the manipulation-check module's
   `[[task]]` shape.
3. Run the full 48-instance H3 report once the converter and the remaining family generators (F2, F3, F5, if still
   missing) are in place.

## Done when

- A family generator's real `generate()` output converts into the manipulation-check module's `[[task]]` shape
  and scores a real gap through `check_pair`/`check_many`, for at least one family (gap-c71dbb narrowed this
  from the full 48-instance, 6-family report to one family, round-tripped — see Progress).
- The `[[verify]]` command passes.

## Notes

- Depends on F2/F3/F5 family generators landing if they haven't (check `benchmarks/viabilitybench/families/` for
  their current state before starting). They have landed (2026-10-05), but F2, F3 and F5 render no
  `spec.precise.md` at all, by design (gap-46fd19) — landing was not sufficient; see Progress.
- 2026-10-05 (wave-19 follow-up): on `work/gap-6e7a86` (not yet merged), this item's converter
  is implemented for F1 only; F2/F3/F5/F7 render no `spec.precise.md` by design (`gap-46fd19`,
  now linked above) and F4's template has different section headers. The remaining coverage
  (F4's converter, and a product decision for the README-only families) is filed separately as
  `gap-76fd1c` (now linked above too) rather than left only as that branch's own Progress note.

## Progress

- gap-c71dbb: implemented, scoped to one family (F1) per this wave's assignment, not the full 48-instance,
  6-family report the item's own "Done when" originally asked for. Checked all six H3 families
  (`benchmarks/viabilitybench/families/f{1,2,3,4,5,7}_*/gen.py`) before writing anything: F2, F3 and F5 now have
  `gen.py` (landed since this item was filed on 2026-10-03), but none of the three renders a `spec.precise.md`
  at all — each skips it on purpose (F5's own module docstring: "this family also skips a separate `template/`
  directory and `spec.precise.md` generation (gap-46fd19's package note)"; F2 and F3 carry the identical
  sentence). Their task description lives only in the instance's rendered `README.md`, free prose with no
  separable acceptance/context/scope/non-goals sections to parse. F7 is the same (`README_TEMPLATE`, no
  `render_spec`). F4 does render a `spec.precise.md`, but from its own template with different section headers
  ("## Constraints", "## Visible check", no "## Non-goals", no explicit `max_loc`) — a second, separate parser
  this item did not scope time for. Only F1's template matches the "S07 TSS v1" layout
  `manipulation_check.py`'s own docstring describes (confirmed by rendering a real instance and reading the
  output directly, not by inspecting source alone).
- Wrote `benchmarks/viabilitybench/specops/from_generator.py`: `parse_precise_spec(text)` reads that layout's
  six parts (title, goal, description, acceptance, verify, context, scope, non-goals) out of the rendered
  markdown with one regex per part, and `task_from_instance(out_dir)` adds the manifest's `instance_id` as the
  task's `id` and cross-checks the parsed file list against the manifest's own `files_in_scope` (`render_spec`
  always builds the "Files in scope" bullets from exactly that list, so a mismatch means the parser mis-read
  the layout, not that the files genuinely differ) — this cross-check is what caught the load-bearing-revert
  test below.
- Added two tests to `specops/tests/test_manipulation.py`: a focused unit test
  (`test_task_from_instance_round_trips_a_fresh_f1_generator_output`) asserting the exact parsed fields and
  that the converted task clears a real `check_pair` gap, and the item's own named
  `test_full_48_instance_h3_report_runs_from_generators`, which — given the finding above — generates 8 fresh
  F1 instances (not the four hand-built fixtures), converts each, and runs the real `check_many` over them,
  asserting every pair passes with `report.ok`. Its docstring says plainly that this is one family at the
  stream's per-family count, not six families at 48 total, and why. Also corrected the stale claim in
  `manipulation_check.py`'s own module docstring and `test_manipulation.py`'s file docstring that said no
  converter existed yet.
- Verified load-bearing: temporarily changed `task_from_instance`'s parsed `files` to always be `[]`; both new
  tests failed immediately, at the manifest cross-check (`ConversionError: parsed Scope files [] !=
  manifest files_in_scope [...]`), not at some unrelated assertion; restored from a backup and re-confirmed
  green. Suites run in the benchmark venv: `specops/` full (55 passed), `families/f1_pyconv/test_f1.py`
  (bundled into a 65-passed combined run with `specops/`), `driver/test_run_cli.py` +
  `driver/test_run_roko.py` (45 passed, 3 skipped — unaffected by this item but share `driver/records.py` with
  gap-6e7a86), plus both items' named `[[verify]]` commands run directly together.
- Left for a future item (not filed separately; this note is the record): a converter for F4's layout, and a
  second conversion path for README-only families (F2/F3/F5/F7) if the full 48-instance H3 report is still
  wanted — the latter needs a product decision (give those families a `spec.precise.md` after all, reversing
  gap-46fd19, or teach `check_pair`/`degrade` to work from unstructured README prose), not just more code here.
