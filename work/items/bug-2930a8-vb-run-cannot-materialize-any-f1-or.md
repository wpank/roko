+++
id = "bug-2930a8"
kind = "bug"
title = "vb run cannot materialize any F1 or F4 instance: materialize.py expects the .vb/ layout both families dropped"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b5da763b9"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["benchmarks/viabilitybench/driver/materialize.py::materialize", "benchmarks/viabilitybench/driver/testdata/toy_family/gen.py", "benchmarks/viabilitybench/families/f1_pyconv/gen.py", "benchmarks/viabilitybench/families/f4_kvtool/gen.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-9e7079", "gap-4723ff", "gap-28ebea", "gap-7ee7c2", "gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_materialize_renders_real_family_instances' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_materialize_renders_real_family_instances -q"

[closed]
at = 2026-09-29
commit = "b5da763b9"
by = "wk-bench-fix1"
evidence = "b5da763b9: materialize.py runs gen.py --out <private>/.vb --workdir <workdir>, reads task.json, spec and pristine.json from that private dir, and checks the workdir's tree and HEAD against the pristine base; the toy family uses the same interface. Verify passes (test_materialize_renders_real_family_instances: vb materialize on F1-l1-0001 and F4-l1-0001, workdir holds no manifest or canary). vb run with the stub provider reached a run record for F1-l3-0001 and F4-l3-0001; full viabilitybench suite green."
+++

## Problem

The reporter found that F4's `gen.py` required `--task-dir`, which `materialize.py` never passes. That interface is gone, but the break it caused remains, and it now affects both families.

- **The families' interface.** Since c1c5db22b (the gap-9e7079 follow-up), F4 uses F1's interface: `gen.py --level L --seed S --out DIR [--workdir WORKDIR]`.
  - DIR receives the private `task.json` (the `vb.task/1` manifest with its canary), `spec.*.md`, `pristine.bundle` and `pristine.json`.
  - The task repo goes to WORKDIR, which defaults to `DIR/repo`.
  - F4's gen.py docstring (:13) says this layout replaces S08 §5.2's `DIR/.vb/`.
- **What materialize.py expects.** It still implements S08 §5.2. It runs `gen.py --level --seed --out <workdir>` (:71-72), expects `<workdir>/.vb/task.json` (`MANIFEST_DIR = ".vb"`, :39), and then moves `.vb/` out of the agent's reach.

Checked at BASE (4315add32) by calling `materialize.materialize` in a scratch directory for F4-l3-0001, F4-l1-0002 and F1-l3-0001. All three raise `MaterializeError: … gen.py wrote no .vb/task.json into …/work`. The generator wrote `task.json`, the spec, the bundle and `repo/` into the path materialize had meant as the agent's workdir.

The driver's tests pass only because they use `driver/testdata/toy_family/gen.py`, which still writes `DIR/.vb/` (:65).

## Why it matters

Pilot benchmark (epic spec-567e52): `streams/pilot.toml` names F1 and F4 instances, so `vb materialize` and `vb run` can't start a single pilot task. Pilot A (gap-c33709) and the Roko arm's live runs are blocked.

## Where

- `driver/materialize.py::materialize` (:60-120): the generator call, the manifest move and the checks.
- The families' `gen.py` `main` functions (F1 :581-585, F4 :284-288).
- The toy family that the tests use.

## Current state

- The families agree with each other. The driver and its toy family still follow the old contract.
- gap-7ee7c2 (verifier CI) records the same F1/F4 interface, but only for the CI scripts.

## Plan

1. **Call the new interface.** Run `gen.py --level ℓ --seed s --out <private_dir>/task --workdir <workdir>`. Read `task.json` from DIR, and resolve `spec.<variant>.path` relative to DIR.
2. **Pristine base.** Take the pristine bundle and `pristine.json` from DIR, since the families already call `repo.init_task_repo`. Check that the workdir's HEAD matches instead of re-initializing it.
3. **Keep every check:** schema, instance id, spec sha256, visible test hashes, and no canary anywhere in the workdir.
4. **Toy family.** Move it to the same interface, so the tests exercise the real contract.
5. **Docs.** Update the module docstring, the README and S08 §5.2 (tmp) to the DIR-plus-WORKDIR layout.
6. **Test.** Add `test_materialize_renders_real_family_instances`, which materializes at least F1-l1-0001 and F4-l1-0001 with the real families.

## Done when

- [ ] `vb materialize --stream pilot --instance F4-l1-0001 --out DIR` and the same for an F1 instance succeed, and the agent's workdir holds no manifest and no canary.
- [ ] The `[[verify]]` command passes.

## Notes

- No model calls are needed: materialization is offline.
