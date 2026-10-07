+++
id = "gap-76b41c"
kind = "gap"
title = "validate.py's KINDS omits vs-label, and the ViabilityBench README doesn't document audit/"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK28 gap-c06ff3)"
discovered_from = "gap-c06ff3 (backlog task 7107)"
anchors = ["benchmarks/viabilitybench/schema/validate.py::KINDS", "benchmarks/viabilitybench/audit/__init__.py::label_errors", "benchmarks/viabilitybench/README.md"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '\"vs-label\"' benchmarks/viabilitybench/schema/validate.py && grep -q 'audit/' benchmarks/viabilitybench/README.md"
+++

## Problem

`benchmarks/viabilitybench/schema/validate.py`'s `KINDS` tuple is
`("task", "feature", "run-record", "metric-record", "price-snapshot", "ledger", "experiment")` — it does not include
`"vs-label"`, even though `benchmarks/viabilitybench/schema/vs-label.schema.json` exists and
`benchmarks/viabilitybench/audit/__init__.py::label_errors` is the row-level validator the `audit` package actually
uses in production (`label_errors(row)` checks the schema and the extra rules the schema's JSON subset can't
express). Passing `"vs-label"` as `validate.py`'s `kind` CLI argument today raises
`ValueError: unknown kind 'vs-label'; expected one of task, feature, run-record, metric-record, price-snapshot,
ledger, experiment` (confirmed by reading `validate.py:93-94`, `:170`).

Separately, `benchmarks/viabilitybench/README.md` never mentions the `audit/` package at all (confirmed: no
occurrence of the literal string `audit/` in the file) even though `audit/` is now a real, tested subsystem
(estimators, lottery, replay — task 7107 / gap-c06ff3) alongside `driver/`, `families/`, `analysis/`, `streams/`,
`schema/`, which the README does describe.

## Why it matters

`validate.py` is the one general-purpose schema linter for every row kind ViabilityBench emits
(`schema/*.schema.json` + the kind-specific extra rules). Leaving `vs-label` out means anyone linting a labels
file from the CLI (`python -m schema.validate <file> vs-label`, the natural command to reach for) gets a
confusing `ValueError` instead of real validation, and has to already know to use `audit.label_errors` directly
instead. The README gap means a newcomer reading it to understand the benchmark's layout has no idea `audit/`
(the M4 deep-audit estimators) exists.

## Where

- `benchmarks/viabilitybench/schema/validate.py::KINDS` (line 43) and its two uses at `:93-94` (the `ValueError`)
  and `:170` (the CLI's `choices=KINDS`).
- `benchmarks/viabilitybench/audit/__init__.py::label_errors` (the real validator `vs-label` rows should be routed
  to, or whose logic `validate.py` should call for this kind).
- `benchmarks/viabilitybench/README.md` (the package-layout section that lists `driver/`, `families/`, `analysis/`,
  etc.).

## Current state

`label_errors` already exists and is exercised by `audit/tests/test_estimate.py` (`test_valid_vs_label_rows`,
`test_vs_label_mutants_fail`, `test_an_audit_row_needs_a_green_verdict_and_false_green_reads_unknown_both_ways`), so
the validation logic itself is not missing — only `validate.py`'s own `KINDS` registry and the README's package
list are out of date relative to it.

## Plan

1. Add `"vs-label"` to `validate.py`'s `KINDS`, and wire its validation path to call (or reuse the same rules as)
   `audit.label_errors` rather than duplicating schema-subset checks — `validate.py` already loads schemas
   generically, so this is likely a small branch plus an import, not a new rule set.
2. Add a short paragraph (or one bullet in the existing layout list) to `benchmarks/viabilitybench/README.md`
   describing `audit/` (estimators, lottery, replay — false-green rate estimation, S05 deep audits).

## Done when

- `python -m schema.validate <file> vs-label` validates against the real rules instead of raising `ValueError`.
- `benchmarks/viabilitybench/README.md` mentions `audit/`.
- The `[[verify]]` command passes.

## Notes

- No LLM spend; this is a small, self-contained fix in two files.
- 2026-10-02 (filer-grpD): filed from backlog wave reports (PK28 gap-c06ff3), confirmed directly against
  `validate.py` and `README.md` at HEAD (both checks below return nonzero today).
