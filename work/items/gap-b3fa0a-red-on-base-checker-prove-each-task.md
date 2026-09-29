+++
id = "gap-b3fa0a"
kind = "gap"
title = "Red-on-base checker: prove each task's verify step fails on a clean base (S07.2)"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "9a442b443"
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

[closed]
at = 2026-09-29
commit = "9a442b443"
by = "commit trailer"
evidence = "9a442b443: `speclint.py plans/ --dynamic` (speclint/dynamic.py) runs each implementer task's verify steps twice, as bash -o pipefail -c, in a detached worktree of the base in a scratch directory outside every worktree (120 s cap). The same step red in both runs is fail (SQ06 = 1), all steps green is pass (HF3), and disagreement, a timeout, a failing pass_on_base step or no known base is unknown (0, flagged). Fixture fixtures/dynamic/red-on-base covers every outcome: fail, pass/HF3, flaky, timeout, not run, base_broken, and archived no_base. tests/test_dynamic.py (25 tests) proves a run leaves worktrees, refs, admin entries, the checkout and a prunable foreign worktree exactly as it found them, also after an interrupt and a mid-step SIGTERM (the step's process group is killed). Mutations that skip the worktree removal, prune, skip the reset between runs, skip the plan-directory copy, skip the landed check, drop the SIGTERM handler or kill only the step's leader each fail a test. Both [[verify]] commands pass, and the speclint suite passes 79 of 79."
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
- 2026-09-29 (wk-redbase): built at `9a442b443`. Where it differs from the Plan:
  - **One worktree per base commit, not per plan.** Before every run of every task the worktree is reset
    (`git reset --hard`, `git clean -ffdx`) and the plan's directory is copied in from the checkout. That is
    stricter than one worktree per plan, and an uncommitted plan still brings its own harness.
  - **"Has not run" comes from git**, because plan metadata cannot be trusted. Every portal-programme plan says
    `status = "ready"`, `done = 0`, and 7 of them have no checkpoint in `.roko/state/graph`.
    - Without `--base`, archived plans are `unknown`.
    - So is a plan whose `files` were touched by the commit that added it, or by any later commit. That catches
      the bulk commit `9c6ec420c`, which landed 30 plans together with their code.
    - At `7c556bc0a` the default checks 11 active plans at HEAD (9 demos, `qa-workflow-validation`,
      `workspace-doctor-improvements`). It reports 25 landed plans (every portal plan among them) and 96 archived
      plans as `unknown`.
  - **Not run on the real corpus**, because its steps run cargo. In a fresh worktree there is no `target/`, so
    expect most cargo steps to hit the 120 s cap and come out `unknown`.
