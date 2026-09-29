+++
id = "gap-7ee7c2"
kind = "gap"
title = "ViabilityBench verifier CI: reference, stub and gaming solutions, determinism and leak checks (S08.T5)"
status = "open"
triage = "verified"
last_verified = 2026-09-29
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
- **Built 2026-09-29 (wk-bench-ci)**, on branch `work/bug-993e7e` after bug-993e7e: `ci/verify_verifiers.py`,
  `determinism.py`, `leak_check.py`, `test_ci.py` and a README.
  - A task family is any `families/*/` directory with F1's and F4's interface: `gen.py --out DIR --workdir W`,
    `hidden.py --task --workdir --secret-file`, and `reference/solutions.py` with `KINDS` and `apply()`. It is named
    by its directory prefix, so F2–F8 join as they are built.
  - The plan-level slice joins as `pl`: `slicekit.py selftest` runs twice, and a materialized workdir is scanned.
  - `--family NAME=DIR` swaps a family's directory, which is how `test_ci.py` breaks verifiers.
- **Stricter than SC1, on purpose:**
  - the stub must also fail the truth suite;
  - hidden.py must leave the tree it judges unchanged;
  - its verdict must carry `passed`, `checks` and the `gaming` flags;
  - each solved tree is scanned for leaks, as well as the fresh workdir;
  - every solution first gets an agent-style visible run with bytecode on, the bug-993e7e case.
- **Results, all green:**

  | Run | Cells | Time |
  |---|---|---|
  | `--seeds 10`, F1 and F4 | 100 of 100 | 137 s |
  | `--seeds 2` (the verify) | 20 cells | 24 s |
  | `pl --seeds 3` | 18 of 18 | 21 s |

  Load was about 20 on 14 cores.
- **Not done:** no GitHub workflow, because the item does not ask for one. Transcripts and diffs in the leak check
  are left for later, as the plan says.
