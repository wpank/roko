+++
id = "gap-76fd1c"
kind = "gap"
title = "F4's converter and a product decision for F2/F3/F5/F7 are still needed for the full 48-instance H3 report"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-05
updated = 2026-10-15
last_verified = 2026-10-05
source = "wave-19 follow-up reports 2026-10-05 (gap-c71dbb, work/gap-6e7a86)"
discovered_from = "gap-c71dbb (open, work/gap-6e7a86; own Progress note recorded this but deliberately did not file it)"
anchors = ["benchmarks/viabilitybench/specops/manipulation_check.py", "benchmarks/viabilitybench/families/f4_kvtool/gen.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_full_48_instance_h3_report_runs_from_generators' benchmarks/viabilitybench/specops/tests/test_manipulation.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py -k test_full_48_instance_h3_report_runs_from_generators -q"
+++

## Problem

The generator-to-`[[task]]` converter `gap-c71dbb` built (on `work/gap-6e7a86`, not yet
merged) covers F1 only, so the full 6-family, 48-instance H3 manipulation-check report is still
blocked, for two separate reasons `gap-c71dbb`'s own Progress note found:

- **F2, F3, F5 and F7 render no `spec.precise.md` at all, by design.** `gap-46fd19` (done)
  deliberately skipped it for these families when they were built: F2/F3/F5 "skip a separate
  `template/` directory and `spec.precise.md` generation (`gap-46fd19`'s package note)," and F7
  has only a `README_TEMPLATE`, no `render_spec` at all. Their task description lives only in
  free-prose `README.md`, with no separable acceptance/context/scope/non-goals sections the
  converter's parser (`specops/from_generator.py::parse_precise_spec`) could read even if it
  tried — this isn't a missing converter, it's missing structured input.
- **F4 does render a `spec.precise.md`, but from a different template.** Different section
  headers ("## Constraints", "## Visible check," no "## Non-goals," no explicit `max_loc`) than
  the "S07 TSS v1" layout `parse_precise_spec` was written against (F1's layout). This needs its
  own parser, not a product decision — just unwritten code.

`gap-c71dbb`'s own Notes already flagged this explicitly as deliberately not filed separately:
"Left for a future item (not filed separately; this note is the record): a converter for F4's
layout, and a second conversion path for README-only families (F2/F3/F5/F7) if the full
48-instance H3 report is still wanted — the latter needs a product decision..., not just more
code here."

## Why it matters

Goal: proof, same goal as `gap-c71dbb`/`gap-e120a1`. The headline 48-instance H3 report S08/S09
depend on still can't run end to end from real generator output on 5 of 6 families — F1 alone,
even correctly converted and scored, is 8 of the 48 instances the spec calls for.

## Where

- `benchmarks/viabilitybench/specops/from_generator.py::parse_precise_spec`,
  `task_from_instance` (F1's converter; F4 needs a sibling parser).
- `benchmarks/viabilitybench/families/f4_kvtool/gen.py` (F4's differently-headed template).
- `benchmarks/viabilitybench/families/f{2,3,5,7}_*/gen.py` (the README-only families;
  `gap-46fd19`'s deliberate simplification).

## Plan

1. Write a second parser (or extend `parse_precise_spec` with a layout variant) for F4's
   `spec.precise.md` headers, converting it the same way F1's is converted.
2. For F2/F3/F5/F7: a product decision is needed first — either give these families a
   `spec.precise.md` after all (reversing part of `gap-46fd19`'s simplification), or teach
   `manipulation_check.check_pair`/`degrade` to work from unstructured README prose instead of
   a parsed, sectioned spec.
3. Once both are resolved, run the full 48-instance, 6-family H3 report end to end and confirm
   it matches `gap-c71dbb`'s original "Done when."

## Done when

- F4 converts and scores through the real `check_pair`/`check_many`, the same way F1 does.
- F2/F3/F5/F7 either gain a convertible spec, or a documented alternative scoring path; Will's
  product decision is recorded either way.
- The full 48-instance, 6-family H3 report runs end to end from real generator output.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-19 follow-up, gap-c71dbb, work/gap-6e7a86 not yet merged): confirmed
  directly on the branch. `gap-c71dbb`'s own Progress note described this remaining work and
  deliberately did not file it separately at the time ("this note is the record"); filing it now
  per the instruction so it doesn't go stale once that item closes on its narrowed, F1-only
  scope.

2026-10-15 (status-quo assessment at 7b6a5af44, findings 28 and 41; tmp/status-quo/2026-10-15/backlog/A03-31-32-defaults-and-specs.md row 3234, work-items/C1-*.md line 12): still open and still the only item for backlog 3234's full 48-instance H3 manipulation-check report, which has never run (the check has run on the four fixture families and on 8 fresh F1 instances only). Its verify is now vacuous: `test_full_48_instance_h3_report_runs_from_generators` exists and passes, but it converts and checks only 8 F1 instances (its own docstring says so), so this item's static verify passes while its Done when does not hold. Replace the verify with a test that needs all 48, for example `grep -qw 'def test_h3_manipulation_report_covers_all_48_instances' benchmarks/viabilitybench/specops/tests/test_manipulation.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py -k test_h3_manipulation_report_covers_all_48_instances -q` (48 pairs from the instances `streams/p1_h3.toml` names, each with gap >= MIN_GAP). The product decision for F2/F3/F5/F7 is shared with a new item from finding 26 (LOG1 block C's vague stream cannot materialize those 32 instances either): rendering a `spec.precise.md` in F1's layout for those families would serve both, and both should land before the pre-registration lock (gap-394f28), which hashes the streams. `benchmarks/viabilitybench/.venv` does not exist in the main checkout at HEAD, so the Python half of this verify cannot run there yet.

- 2026-10-16: the premise above changed, in exactly the direction the 2026-10-15 note anticipated. `bug-f90bd0`
  (closed; merged as `980678432`, "LOG1 block C's vague H3 stream cannot materialize 32 of its 48 instances"),
  fixing a different, `driver/materialize.py`-side problem, gave F2, F3, F5 and F7 a `spec.precise.md` each --
  new `spec/precise.template.md` files plus matching `gen.py` changes -- reversing part of `gap-46fd19`'s
  README-only simplification. Checked directly: that new markdown is in exactly F1's own TSS v1 section layout
  ("## Acceptance criteria", "## Verify", "## Context", "## Scope", "## Non-goals"), so
  `specops.from_generator.task_from_instance` converts real F1, F2, F3, F5 and F7 `generate()` output unchanged,
  with no further code or product decision needed for those four. Only F4's converter was still missing --
  confirmed by generating real instances of all six families and running each through `task_from_instance`
  before writing any new code. Built here: `parse_precise_spec_f4`, for F4's own layout ("## Goal",
  "## Constraints", "## Visible check"; no "## Non-goals"; no "Change at most N lines" sentence, so `max_loc` is
  a fixed constant -- unused by `check_pair`/`degrade`'s actual scoring, confirmed by grep), dispatched from
  `task_from_instance` on the manifest's own `family` field. `test_full_48_instance_h3_report_runs_from_generators`
  now generates and converts 8 real instances of each of the 6 H3 families (48 total, F1-F5 and F7) and runs them
  through `check_many`: `report.ok` is true, every pair's gap clears `MIN_GAP` (observed minimum 53.17 against a
  bar of 30). The full 48-instance H3 report runs end to end from real generator output.
