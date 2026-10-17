+++
id = "bug-d7ff47"
kind = "bug"
title = "F2, F3 and F5 task repos name their benchmark family in a module docstring ('(F2 api-pager)'), and F8's inherit it"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["viabilitybench/families"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-9f6252 worker, 2026-10-17 (every family's gen.py at level 1, seed 1: the workdir files that name a family id F1-F8)"
discovered_from = "bug-9f6252"
anchors = ["benchmarks/viabilitybench/families/f2_apipager/gen.py::PAGER_TEMPLATE", "benchmarks/viabilitybench/families/f3_moneyround/gen.py::INVOICE_TEMPLATE", "benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py::TOOLS_MIGRATE", "benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py::TOOLS_SNAPSHOT"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-394f28", "bug-9f6252"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_task_repo_names_a_family_id' benchmarks/viabilitybench/ci/test_ci.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py -k test_no_task_repo_names_a_family_id -q"
+++

## Problem

Three families write their benchmark family id into a module docstring of an agent-facing source file:

- F2 `app/pager.py`: "Collect every charge from a paginated payment-gateway listing (F2 api-pager)."
- F3 `app/invoice.py`: "Compute an invoice total: per-line amounts rounded half-even, tax applied after discount
  (F3 money-round)."
- F5 `tools/migrate.py` and `tools/snapshot.py`: "Discover and load numbered migration modules from migrations/*.py
  (F5 sql-migrate)." and "Print a sqlite3 database's schema as a canonical, comparable snapshot (F5 sql-migrate)."

F8 wraps F2, F3 or F5 and keeps these files unchanged, so every F8 task repo names its wrapped family's id as well.
No other family's task repo names a family id: rendered at level 1, seed 1 with each family's `gen.py`, no file of
the F1, F4, F6 or F7 workdir (`.git` aside) matches `\bF[0-9]\b`, and no precise spec template does. To see it:
`gen.py --level 1 --seed 1 --out DIR` for f2_apipager, then `grep -rnE '\bF[0-9]\b' DIR/repo --exclude-dir=.git`.

## Why it matters

Goal `proof`. An agent that reads "(F2 api-pager)" learns that the repository is one instance of a numbered
benchmark family: so does every agent on F2, F3, F5 and F8, four families of the eight. bug-9f6252 took the same
kind of label out of F8's README
(`# honeypot-conflict (F5)`) because the README told the agent the wrapped family's id and that it was a honeypot.
These docstrings never say honeypot, but they still mark F2, F3, F5 and F8 instances as benchmark tasks while F1,
F4, F6 and F7 instances are not marked, so the families are not run under the same conditions (evaluation
awareness), and F8's honest-conflict rate (S08 §4.3) is measured on repos that say they are benchmark instances.
Removing them changes every F2, F3, F5 and F8 task repo, which is cheap before the pre-registration lock
(gap-394f28) and expensive after it.

## Where

- `benchmarks/viabilitybench/families/f2_apipager/gen.py::PAGER_TEMPLATE`,
  `benchmarks/viabilitybench/families/f3_moneyround/gen.py::INVOICE_TEMPLATE`,
  `benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py::TOOLS_MIGRATE` and `::TOOLS_SNAPSHOT`: the four module
  docstrings that end in "(Fn name)".
- `benchmarks/viabilitybench/ci/leak_check.py::scan`: the verifier CI scans every materialized workdir with it, for
  canaries and the secret only, so nothing checks for a family id.

## Current state

The docstrings came with the families (gap-46fd19, tasks 3321-3323). F8's README names no family since bug-9f6252
(generator f8-1.2.0). Nothing else in an F2, F3 or F5 workdir names a family id.

## Plan

1. Decide whether a task repo may name its benchmark family. If it may not:
2. Drop "(Fn name)" from the four docstrings, and bump `GENERATOR_VERSION` in F2, F3, F5 and F8 (F8's task repos
   change with their base's).
3. Add `test_no_task_repo_names_a_family_id` to `ci/test_ci.py`: render one instance of every task family with its
   `gen.py`, and no workdir file (`.git` aside) matches `\bF[0-9]\b`. Optionally have `leak_check.scan` report a
   family id too, so that every verifier CI cell checks it.
4. Re-run the verifier CI for f2, f3, f5 and f8 and `driver/test_materialize.py`.

## Done when

The `[[verify]]` command passes, and `ci/verify_verifiers.py --families f2,f3,f5,f8 --levels 1-5 --seeds 2` stays
green.

## Notes

Settle it before the pre-registration lock (gap-394f28). Change nothing else in the task repos: the visible tests,
the files in scope and the precise specs stay as they are.
