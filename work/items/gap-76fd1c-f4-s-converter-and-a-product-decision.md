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
updated = 2026-10-05
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
