+++
id = "bug-f90bd0"
kind = "bug"
title = "LOG1 block C's vague H3 stream cannot materialize 32 of its 48 instances: only F1 and F4 have a vague spec variant, and 3329's verify samples only those two"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["viabilitybench/driver", "viabilitybench/families"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-26"
anchors = ["benchmarks/viabilitybench/driver/materialize.py::_add_vague_spec", "benchmarks/viabilitybench/driver/materialize.py::VAGUE_FAMILIES", "benchmarks/viabilitybench/streams/log1_p1_h3_vague.toml", "benchmarks/viabilitybench/driver/test_materialize.py::test_h3_variants_pass_the_manipulation_check", "benchmarks/viabilitybench/families/f2_apipager/gen.py", "benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py"]
lane = "bench"
links = { depends_on = [], blocks = ["gap-394f28"], related = ["gap-76fd1c", "gap-daeaa9", "gap-394f28", "gap-c71dbb", "gap-4a5109", "gap-46fd19"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_every_h3_vague_instance_materializes' benchmarks/viabilitybench/driver/test_materialize.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_materialize.py -k test_every_h3_vague_instance_materializes -q"
+++

## Problem

LOG1 block C (S09 §4.3) runs the 48 H3 instances under their vague spec:
`benchmarks/viabilitybench/streams/log1_p1_h3_vague.toml` lists them, 8 each of F1, F2, F3, F4, F5 and F7. The
materializer builds a vague variant only for F1 and F4: `VAGUE_FAMILIES = ("F1", "F4")`
(`driver/materialize.py:74`), and `_add_vague_spec` refuses any other family with "has no vague variant yet"
(`:172-173`). So block C would fail materialization on the 32 F2, F3, F5 and F7 instances. F2, F3, F5 and F7 render
no `spec.precise.md` at all, by design (their `gen.py` module docstrings, gap-46fd19), so there is nothing for the D-v1
operator to degrade.

Backlog 3329 ("All 48 instances have both variants, pass the manipulation check, and satisfy the recoverability
rule") was closed with PK25 (gap-daeaa9), whose progress note says "F1 and F4 only". Its verify,
`test_h3_variants_pass_the_manipulation_check`, samples only F1 and F4, so it passes. Status-quo assessment
2026-10-15 at 7b6a5af44, finding 26 (`tmp/status-quo/2026-10-15/backlog/A04-33-proof-bench.md`, row 3329).

## Why it matters

Goal `proof`. H3 (vague vs precise specs) is one of the pre-registered hypotheses. The pre-registration lock
(gap-394f28) hashes the streams and experiments, so the stream and the families it needs must be right before the
lock is taken; fixing them afterwards means a new lock. Will's 2026-10-15 decision 1 defers the lock until the pilots
report, which leaves time to fix this first.

## Where

- `benchmarks/viabilitybench/driver/materialize.py`: `VAGUE_FAMILIES`, `_add_vague_spec`,
  `recoverability_violations`, `render_vague_markdown`.
- `benchmarks/viabilitybench/families/{f2_apipager,f3_moneyround,f5_sqlmigrate,f7_rustiter}/gen.py`: render a README
  task statement, no `spec.precise.md`.
- `benchmarks/viabilitybench/streams/log1_p1_h3_vague.toml` and `streams/p1_h3.toml`: the 48 instances.

## Current state

F1 and F4 vague variants work, with the recoverability check. The other four families have generators, truth suites
and ladders (verifier CI green on 2026-10-07) but no precise spec file.

## Plan

Options:

1. **(recommended)** Give F2, F3, F5 and F7 a rendered `spec.precise.md` in F1's section layout (title, goal,
   acceptance, context, scope, verify, non-goals), built from what each generator already knows, then add them to
   `VAGUE_FAMILIES` and check recoverability per family. This also gives gap-76fd1c (the 48-instance manipulation-check report) the structured
   input it lacks.
2. A second vague path that degrades the README-only statement directly.
3. Cut block C to F1 and F4 and change S09 §4.3's cell counts. That changes the pre-registered design, so it is Will's
   call.

Then add `test_every_h3_vague_instance_materializes`: every instance in `log1_p1_h3_vague.toml` materializes with
`spec_variant="vague"` and passes `recoverability_violations`.

## Done when

The `[[verify]]` command passes, with all 48 instances materializing (not a sample).

## Notes

This must land before gap-394f28 takes the lock: the coordinator should link it as blocking gap-394f28. The verify
needs `benchmarks/viabilitybench/.venv`, which does not exist in the main checkout at HEAD (create it with the bench's
requirements first). Don't edit the stream's instance list: `experiments/test_log1.py` checks that it matches
`p1_h3.toml`.
