+++
id = "bug-b5eab4"
kind = "bug"
title = "F2, F3, F5, F6 and F7 task repos head README.md with their S08 family name ('# api-pager', '# ts-result'), F6's package is 'ts-result-task', and F8's inherit the heading"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["viabilitybench/families"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d7ff47 worker (claude-agent), 2026-10-17 (every family's gen.py at levels 1-5 x seeds 1-3 with every latent: the workdir files that carry an S08 family name)"
discovered_from = "bug-d7ff47"
anchors = ["benchmarks/viabilitybench/families/f2_apipager/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f3_moneyround/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f6_tsresult/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f6_tsresult/gen.py::PACKAGE_JSON", "benchmarks/viabilitybench/families/f7_rustiter/gen.py::README_TEMPLATE"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d7ff47", "bug-9f6252", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_task_repo_names_its_family' benchmarks/viabilitybench/ci/test_ci.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py -k test_no_task_repo_names_its_family -q"
+++

## Problem

Five of the seven base families head their task repo's `README.md` with the family's own S08 name, the one the specs
and papers use beside its id ("F2 `api-pager`"): F2 `# api-pager`, F3 `# money-round`, F5 `# sql-migrate`, F6
`# ts-result` and F7 `# rust-iter: add` (each family's `gen.py::README_TEMPLATE`). F6's `package.json` also names
the package `"ts-result-task"`. F8 keeps its wrapped base's README heading (bug-9f6252), so every F8 task repo shows
`# api-pager`, `# money-round` or `# sql-migrate`. F1 and F4 do not: F1's README is `# ledgerline` and F4's is
`# tallybook`, product names (F4's CLI is called `kvtool` throughout, as the product it is; its family is
`kvtool-cli`). To see it: render each family with `gen.py --level 1 --seed 1 --out DIR` and read `DIR/repo/README.md`'s
first line. Rendered at levels 1-5 x seeds 1-3 with every latent (165 workdirs), the names appear in README.md in
every F2, F3, F5, F6, F7 and F8 workdir (105) and in every F6 `package.json` (30), and nowhere else once bug-d7ff47's
docstrings are gone; F7's `Cargo.toml` does not name it.

## Why it matters

Goal `proof`. A weaker form of bug-d7ff47 (family ids in module docstrings): these names read as plausible project
names, but they are the benchmark's family names, so an agent that knows the benchmark can tell which family it
faces, and `ts-result-task` calls the repository a task outright. F2, F3, F5, F6, F7 and F8 instances are marked
this way and F1 and F4 instances are not, so the families are not run under the same conditions. F8's honeypot is
not revealed by this (its heading is its base's, the same as an instance of the base family). Changing the headings
changes six families' task repos, so it has to be decided before the pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/families/f2_apipager/gen.py::README_TEMPLATE`, and the same in `f3_moneyround`,
  `f5_sqlmigrate`, `f6_tsresult` and `f7_rustiter`.
- `benchmarks/viabilitybench/families/f6_tsresult/gen.py::PACKAGE_JSON`: `"name": "ts-result-task"`.
- `benchmarks/viabilitybench/families/f8_honeypot/gen.py::render_readme` keeps the base's heading.

## Current state

The headings came with the families (gap-46fd19, gap-eb1aa3). bug-d7ff47 removed the family ids ("(F2 api-pager)")
from F2, F3 and F5's module docstrings and added `ci/test_ci.py::test_no_task_repo_names_a_family_id`, which checks
`\bF[0-9]\b` only. No precise spec names a family (rendered at level 1, seed 1).

## Plan

1. Decide whether a task repo may carry its family's S08 name. If it may not:
2. Give each README a product name, as F1 and F4 have (and F6's package one), and bump `GENERATOR_VERSION` in F2,
   F3, F5, F6, F7 and F8.
3. Add `test_no_task_repo_names_its_family` to `ci/test_ci.py`, beside `test_no_task_repo_names_a_family_id`: no
   workdir file of any task family names a family's S08 name.
4. Re-run the verifier CI for the six families, and show that only the renamed text changes in their task repos.

## Done when

The `[[verify]]` command passes, and `ci/verify_verifiers.py --families f2,f3,f5,f6,f7,f8 --levels 1-5 --seeds 2`
stays green.

## Notes

- Found by the bug-d7ff47 worker, 2026-10-17, while checking what else in a task repo names its family.
- Change nothing else in the task repos: the visible tests, the files in scope and the precise specs stay as they
  are. None of the 165 rendered precise specs names a family.
