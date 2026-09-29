+++
id = "gap-b3fa0a"
kind = "gap"
title = "Red-on-base checker: prove each task's verify step fails on a clean base (S07.2)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/execution/checklist.json (S07.2); tldr/04 design rule 3"
anchors = ["benchmarks/viabilitybench/speclint/dynamic.py", "benchmarks/viabilitybench/speclint/tests/test_dynamic.py"]
lane = "bench"
parent = "spec-e57870"
links = { depends_on = ["gap-1cd8d3"], blocks = [], related = ["find-70edcb", "gap-d14a43", "gap-46ab3f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_verify_passing_on_base_is_hf3' benchmarks/viabilitybench/speclint/tests/test_dynamic.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_dynamic.py -k test_verify_passing_on_base_is_hf3 -q"

[[verify]]
command = "grep -qw 'def test_removes_only_its_own_worktrees' benchmarks/viabilitybench/speclint/tests/test_dynamic.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/speclint/tests/test_dynamic.py -k test_removes_only_its_own_worktrees -q"
+++

## Problem

A verify step that already passes before the change proves nothing, and nothing checks for this:

- 19.2% of current tasks (41 of 214) have only grep or `test` verify steps (research note B4).
- Dogfood found verify steps that passed on an untouched repo (find-70edcb, R-1 and R-5).
- No red-on-base logic exists anywhere, and `VerifyStep` has no `expect` field.

## Why it matters

tldr/04 design rule 3: every task gets at least one check written at plan time and proven to fail on the base
(`konstantinou2024do`). S07's rule SQ06 (15 of 100 points) and hard fail HF3 depend on it, and assessment W8's
canary C4 asks for it. Part of epic spec-e57870.

## Where

- **New:** `benchmarks/viabilitybench/speclint/dynamic.py` and `tests/test_dynamic.py`, with fixtures under
  `fixtures/dynamic/`. The path follows decision D4, as for gap-1cd8d3.
- Entry: `speclint.py plans/ --dynamic`, plus a fixture mode for tests.

## Current state

Checked at `41c7ffbd6`: nothing exists. gap-1cd8d3 (the static slice) comes first.

## Plan

1. For each plan, create a clean base worktree with `git worktree add --detach <scratch>/<id> <base>` in a scratch
   directory outside the checkout. The base is HEAD for a plan that has not run yet. Archived plans need their
   pre-change commit; when it is unknown, the result is `unknown`.
2. Run each implementer task's verify steps there twice, with 120 s per step. Record `red_on_base` as `fail`, `pass`
   or `unknown` (the two runs disagree, or a step times out).
3. Score: `fail` gives SQ06 = 1; `pass` is HF3; `unknown` scores 0 and is flagged. Until S07.8 adds
   `expect = "pass_on_base"` for refactors, treat every implementer task as expected to fail on the base.
4. Remove only the worktrees this run created, once the run ends.

## Done when

- [ ] Fixtures: a step that passes on the base is HF3; a step that fails on the base scores SQ06 = 1; a flaky step
      is `unknown`.
- [ ] A run leaves no worktree or branch behind and touches no other worktree.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Never delete existing worktrees or plan branches** (repo rule). Use detached worktrees only, in a scratch
  directory.
- Expect many `unknown` results on archived plans whose base no longer builds. Report them; do not fail on them.
- The Rust port is a later `plan validate --spec-quality --dynamic` flag (S07.9), after gap-46ab3f.
