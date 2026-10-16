+++
id = "bug-9f6252"
kind = "bug"
title = "Every F8 task repo's README.md opens with \"# honeypot-conflict (F5)\": the agent is told the instance is a honeypot"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["viabilitybench/families"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-ca1de5 worker, 2026-10-16 (rendered F8 workdirs, README.md's first line)"
discovered_from = "bug-ca1de5"
anchors = ["benchmarks/viabilitybench/families/f8_honeypot/gen.py::generate", "benchmarks/viabilitybench/families/f8_honeypot/gen.py::README_TEMPLATE"]
lane = "bench"
links = { depends_on = ["bug-ca1de5"], blocks = ["gap-e9218e"], related = ["gap-394f28", "bug-87ea7f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_task_repo_never_says_honeypot' benchmarks/viabilitybench/families/f8_honeypot/test_f8.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f8_honeypot/test_f8.py -k test_the_task_repo_never_says_honeypot -q"
+++

## Problem

Every F8 task repo tells the agent that it is a honeypot. `gen.generate` writes README.md from
`README_TEMPLATE.format(name=f"honeypot-conflict ({name})", body=...)`, so the README's first line is, for example,
`# honeypot-conflict (F5)`, above the wrapped family's own README (`# sql-migrate`). The agent reads the family's
name, which says the task is a honeypot about a conflict, and the id of the wrapped family. Task 3325's notes say
that agents are never told an instance is a honeypot, and that the README states the conflict convention
generically, as it would for any instance; F8's precise spec (bug-ca1de5) says nothing about a honeypot for the
same reason.

## Why it matters

Goal `proof`. LOG1 block E (gap-e9218e, task 3351) runs the 10 F8 honeypots for H5's detector recall, and S08 §4.3
reports an honest-conflict rate beside the resolve rates: both measure whether an agent notices and flags a
documentation conflict it was not told about. A README that names the honeypot primes the flag and makes the
measurement meaningless.

## Where

- `benchmarks/viabilitybench/families/f8_honeypot/gen.py::generate`: the README it writes over the base's, and its
  fallback body for a base with no README ("This instance also wraps a ... task").
- `benchmarks/viabilitybench/families/f8_honeypot/gen.py::README_TEMPLATE`: the `# {name}` heading.

## Current state

bug-ca1de5 kept every F8 task repo byte-identical on purpose (70 instances' pristine commit, tree and visible test
hashes unchanged), so the heading is still in every instance.

## Plan

1. Keep the wrapped family's README as it is (its own `# api-pager`, `# money-round` or `# sql-migrate` heading)
   and append only the section "This repository's convention"; for a base with no README, write one whose heading
   names the base task, not the family.
2. Bump `GENERATOR_VERSION` (the task repo changes), and re-run the verifier CI for f8 and
   `driver/test_materialize.py -k test_every_log1_f8_honeypot_materializes`.
3. Add `test_the_task_repo_never_says_honeypot` to `families/f8_honeypot/test_f8.py`: for one instance of each
   wrapped family, no file of the workdir mentions "honeypot" (any letter case) and README.md names no family id.

## Done when

The `[[verify]]` command passes and the F8 verifier CI stays green.

## Notes

This changes every F8 task repo, so land it before the pre-registration lock (gap-394f28). Keep the convention
paragraph itself (task 3325). The flag check's own defect (the README's convention line satisfies it) is
bug-87ea7f.
