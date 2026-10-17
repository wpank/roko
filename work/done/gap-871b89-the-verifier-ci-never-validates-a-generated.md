+++
id = "gap-871b89"
kind = "gap"
title = "The verifier CI never validates a generated task.json against vb.task/1 (S08 §4.2 (3)), so F6 still emits no recoverability"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["viabilitybench/ci"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "e0fc8fa05"
source = "bug-f90bd0 worker, 2026-10-16 (materialize F6-l1-0001: invalid manifest)"
discovered_from = "bug-f90bd0"
anchors = ["benchmarks/viabilitybench/ci/verify_verifiers.py", "benchmarks/viabilitybench/ci/test_ci.py", "benchmarks/viabilitybench/families/f6_tsresult/gen.py::generate"]
lane = "bench"
links = { depends_on = ["bug-ca1de5"], blocks = [], related = ["bug-f90bd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_invalid_manifest_turns_ci_red' benchmarks/viabilitybench/ci/test_ci.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py -k test_an_invalid_manifest_turns_ci_red -q"

[[verify]]
command = "grep -qw 'def test_f6_manifest_is_a_valid_task' benchmarks/viabilitybench/families/f6_tsresult/test_f6.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f6_tsresult/test_f6.py -k test_f6_manifest_is_a_valid_task -q"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T20:50:00Z"
commit = "e0fc8fa05"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:33Z"
forced = false
evidence = "e0fc8fa05: ci/verify_verifiers.py runs schema/validate.py's validate('task', ...) on every cell's manifest, as driver/materialize.py does, and an invalid one makes the cell red with a CellError ('the manifest is not vb.task/1: ...') before any kind is judged. F6's gen.py lists recoverability (one entry per truth-suite check, citing spec.precise.md#AC1-AC4 and README.md, src/domain/errors/, the visible test and tsconfig.json); generator f6-1.1.0, its task repos and spec byte-identical across 40 instances (both latents). Both verifies pass: test_an_invalid_manifest_turns_ci_red (an F4 copy whose gen drops recoverability: cell F4-l1-0001 red, no kind judged; main's CI judges the same copy green) and test_f6_manifest_is_a_valid_task (v1 and v2 validate, every cited criterion in the spec and every file in the workdir; main's F6 fails with missing required field 'recoverability'). F6-l1-0001 and F6-l5-0002 now materialize, both latents. verify_verifiers on every family: f1,f2,f3,f4,f5,f8 levels 1-5 seeds 2 (the workflow's job) 80/80, f6 levels 1-5 seeds 2 (node, no tsc) 20/20, pl seeds 3 18/18, f7 levels 1-5 seeds 2 through the cargo limiter 10/10. test_ci.py 9 passed; bench pytest 772 passed, 16 skipped (no roko binary, opt-in sweeps, analysis stack)."
+++

## Problem

S08 §4.2 (3): "The generator emits `recoverability[]` and CI fails an instance without it." `ci/verify_verifiers.py`
reads each cell's `task.json` but checks only that `visible_verify` and `visible_test_hashes` are present; it never
runs `schema/validate.py` on it. So family manifests drifted from `vb.task/1` with the verifier CI green: F2, F3, F5
and F7 until bug-f90bd0, F8 still (bug-ca1de5), and F6, whose manifest has a `spec` but no `recoverability`:
`materialize()` refuses `F6-l1-0001` with `invalid manifest: $: missing required field 'recoverability'`.

## Why it matters

Goal `proof`. The driver validates every manifest before a run (`driver/materialize.py`), so an invalid one is found
only when a campaign reaches it. F6 is in no P1 stream (S08 §4.3: a transfer probe only), so nothing in LOG1 is
blocked by it today; the missing CI check is what let five families drift unnoticed.

## Where

- `benchmarks/viabilitybench/ci/verify_verifiers.py`: the per-cell manifest checks after `gen.py` runs.
- `benchmarks/viabilitybench/families/f6_tsresult/gen.py::generate`: the manifest, with no `recoverability`.

## Current state

`schema/validate.py` implements the subset of JSON Schema the bench's schemas use, and the driver already calls it
(`validate.validate("task", manifest)`). Since bug-f90bd0, F1-F5 and F7 emit valid manifests; F6 and F8 do not.

## Plan

1. In `verify_verifiers.py`, validate each cell's manifest with `validate.validate("task", ...)` and fail the cell on
   any error (a `CellError`). Add `test_an_invalid_manifest_turns_ci_red` to `ci/test_ci.py`: an F4 copy
   (`f4_copy(wrap="gen")`) whose wrapper drops a required field from the manifest it wrote.
2. Give F6 a `recoverability` list: each hidden requirement, with the spec AC or the workdir file that states it.
3. Run `verify_verifiers.py` over every family. F8 turns green only after bug-ca1de5, which this item waits on.

## Done when

The verifier CI fails a cell whose manifest is not `vb.task/1`, F6's manifests validate, and both `[[verify]]`
commands pass.

## Notes

The GitHub workflow (`.github/workflows/viabilitybench-ci.yml`) runs `verify_verifiers.py --families
f1,f2,f3,f4,f5,f8`, so step 1 turns that job red until bug-ca1de5 lands; merge this after it. F6's job needs Node and
TypeScript.
