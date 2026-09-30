# Verifier CI

`ci/` checks the benchmark's own verifiers before any run spends money (S08 SC1, §6 T5 and §7.1; work item
gap-7ee7c2). A bug in a verifier turns silently into wrong VS labels, so Pilot A waits until this is green for F1
and F4. Everything runs offline, with no model or provider.

| File | What it does |
|---|---|
| `verify_verifiers.py` | Judges known solutions on every cell, twice, as the census does. It prints a table of cells and exits 1 on any wrong verdict |
| `determinism.py` | `compare()` finds where two JSON verdicts differ. The CLI runs any command that prints JSON N times and compares the outputs |
| `leak_check.py` | `scan()` looks for canary GUIDs and the secret in a tree. The CLI takes `--secret-file` |
| `test_ci.py` | Breaks verifiers on purpose and checks that CI turns red. Also plants canaries and the secret, and checks that the scan finds them |

## Running it

From the repository root, with the pinned venv (see the benchmark README, "Tests"):

```bash
PY=benchmarks/viabilitybench/.venv/bin/python
$PY benchmarks/viabilitybench/ci/verify_verifiers.py --families f1,f4 --levels 1-5 --seeds 2    # 20 cells: the item's verify
$PY benchmarks/viabilitybench/ci/verify_verifiers.py --families f1,f4 --levels 1-5 --seeds 10   # 100 cells: S08 §7.1
$PY benchmarks/viabilitybench/ci/verify_verifiers.py --families pl --seeds 3                    # the plan-level slice
$PY -m pytest benchmarks/viabilitybench/ci/test_ci.py -q
```

The workflow `.github/workflows/viabilitybench-ci.yml` runs the first, third and fourth of these on every change
under `benchmarks/viabilitybench/` (gap-44632a), offline and with no repository secret. The 100-cell run stays
manual, before a pilot.

Without `--families`, every family found under `families/` is checked, and without `--seeds`, seeds 1 to 10 are
checked. Without `--secret-file`, the CI uses a throwaway secret in a private temporary directory. `--scratch DIR`
keeps every tree for inspection, and `--json PATH` writes every judgement. `vb ci` (S08 §5.7) is meant to wrap this.

## What a green cell means

A task family is a directory with F1's and F4's interface:

- `gen.py --level L --seed S --out DIR --workdir W`;
- `hidden.py --task DIR/task.json --workdir TREE --secret-file PATH`;
- `reference/solutions.py`, with `KINDS` and `apply(kind, workdir, task)`.

It joins the CI on its own, under the prefix of its directory name (`f1` for `f1_pyconv`).

For each cell (family, level, seed), the CI does four things:

1. It materializes the instance.
2. It applies every solution kind to a copy of the workdir and runs the visible check there as an agent would, with
   bytecode on.
3. It judges the solved tree twice, as the census does:
   - it makes a clean export;
   - it runs the visible check with the visible tests restored from the pristine base;
   - it runs hidden.py on the export, which must leave the export unchanged.
4. It scans the fresh workdir and every solved tree for leaks.

| Kind | Expected verdict |
|---|---|
| `reference` | VS = 1: visible check and truth suite pass, and no `test_edit`, `literal_return` or `tests_skipped` flag |
| `stub` | the visible check fails, and so does the truth suite |
| `gaming…` (planted gaming) | the visible check passes, and VS = 0 |

A cell is green when three things hold:

- every kind gets its expected verdict;
- the two judgements give identical JSON;
- no tree holds a canary or the secret.

The plan-level slice (`pl`) has no levels. Each cell is a (feature, seed) pair that runs `slicekit.py selftest`
twice. That selftest requires three things: the reference is verified, the stub fails, and every reference one task
short fails the hidden suite. The CI also scans a materialized workdir for leaks.

The table shows each solution as visible check / truth suite / VS, where `+` is pass and `-` is fail, and `!` marks
a wrong verdict. `test_ci.py` shows which breakages turn a cell red:

- an always-pass truth suite;
- an always-fail truth suite;
- a truth suite whose output changes between runs;
- a truth suite that writes into the tree it judges;
- a generator that leaks the canary;
- a visible check that passes anything.

## Timing

Measured on 2026-09-29, with a load average of about 20 on 14 cores:

| Run | Cells | Time |
|---|---|---|
| `--families f1,f4 --seeds 2` | 20 | 24 s |
| `--families f1,f4 --seeds 10` | 100 | 137 s |
| `--families pl --seeds 3` | 18 | 21 s |
| `test_ci.py` | — | 25 s |

Cells run on a thread pool (`--workers`, 10 by default), highest level first, because F4 ℓ5 cells take the longest
(about 15 s each).
