+++
id = "gap-7ee7c2"
kind = "gap"
title = "ViabilityBench verifier CI: reference, stub and gaming solutions, determinism and leak checks (S08.T5)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/ci"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§1 SC1, §6 T5, §7.1; checklist S08.T5)"
anchors = ["benchmarks/viabilitybench/ci/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-4723ff", "gap-9e7079"], blocks = [], related = ["gap-a8a160"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_always_pass_verifier_turns_ci_red' benchmarks/viabilitybench/ci/test_ci.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py -k test_always_pass_verifier_turns_ci_red -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/ci/verify_verifiers.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f1,f4 --levels 1-5 --seeds 2"
+++

## Problem

A bug in a verifier silently turns into wrong VS labels. Nothing checks, across all families, that:

- reference solutions pass;
- stubs fail;
- planted gaming passes the visible checks and fails the hidden ones;
- two runs give the same verdict;
- materialized workdirs contain no canary and no secret.

## Why it matters

- **It covers S08 SC1** (100% of cells) and half of SC4.
- **It is the first G0 go criterion:** "Verifier CI 100% (F1, F4)".
- **Pilot A must not spend money until it is green.**

## Where

All new, and the paths follow D4. `benchmarks/viabilitybench/ci/` holds:

- `verify_verifiers.py`;
- `determinism.py`;
- `leak_check.py`, which takes `--secret-file`;
- `test_ci.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists. Each family item has its own self-test; this item runs all of them the
same way.

## Plan

1. **`verify_verifiers.py --families f1,f4 --levels 1-5 --seeds N`.**
   - For every cell, it materializes the task and applies the reference, stub and gaming solutions.
   - It runs the visible and hidden checks on each, and prints a table of cells.
   - It exits non-zero on any wrong verdict.
2. **`determinism.py`.** It runs every verdict twice and compares the JSON.
3. **`leak_check.py`.** It scans workdirs for every canary GUID and for the secret, and fails on any hit.
   Transcripts and diffs are added later.
4. **Tests.**
   - A family whose `hidden.py` always passes turns CI red.
   - A canary planted in a workdir is found.

## Done when

- [ ] `--seeds 10` reports 100 of 100 cells green for F1 and F4 (S08 §7.1).
- [ ] Breaking any verifier turns CI red.
- [ ] Both `[[verify]]` commands pass. The second runs 20 cells, so that it finishes within 60 s.

## Notes

- **Later:** `vb ci` (S08 §5.7) will wrap these scripts, and F2–F8 join as they are built.
- **Waits for:** F1 and F4. No hot files.
- **From wk-bench-f4 (gap-9e7079, 2026-09-29):** the F1 and F4 interfaces differ, so the CI scripts must reconcile them, or the families converge first. F4: `gen.py --level L --seed S --out WORKDIR --task-dir PRIVATE`, `hidden.py --task PRIVATE/task.json --workdir TREE --secret-file PATH [--timeout S] [--scratch DIR]`, and `solutions.py::apply_solution(kind, workdir, task)` at the family root. F1 (on `work/gap-4723ff`): `gen.py --level ℓ --seed s --out DIR [--workdir WORKDIR] [--latent v1]`, where `DIR` holds the private `task.json`; `hidden.py --task DIR/task.json --workdir TREE --secret-file PATH`; and `reference/solutions.py::apply(kind, workdir, task)`. The plan slice has its own verifier CI, `families/plan_slice/slicekit.py selftest`, which `ci/verify_verifiers.py` could call.
