+++
id = "bug-d0906c"
kind = "bug"
title = "The spec refiner keeps a round that adds a hard fail: refine.py compares scores only, so a live round's vacuous verify step (HF2) or missing read file (HF4) is kept when the score rises"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/specops"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gap-942f88 worker (claude-agent), 2026-10-18: live vb refine run refine-3236-20261018b at 8431b7f65, hard fails 2 -> 11 tasks (HF4 added on 10, HF2 on 1) while every round was kept"
discovered_from = "gap-942f88"
anchors = ["benchmarks/viabilitybench/specops/refine.py::refine", "benchmarks/viabilitybench/driver/refine_live.py::SYSTEM_PROMPT"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-942f88", "gap-4a5109"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_round_that_adds_a_hard_fail_is_rejected' benchmarks/viabilitybench/specops/tests/test_refine.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_refine.py -k test_a_round_that_adds_a_hard_fail_is_rejected -q"
+++

## Problem

The spec refiner keeps a round that adds a hard fail. `refine.refine` rejects a round that would lower the spec's
`speclint.score_task` score, but it never compares hard fails, and S07 §4.2's hard fails block a spec whatever its
score. So a round can raise the score and turn an unblocked spec into a blocked one, and the refiner keeps it.

Seen live on 2026-10-18 (gap-942f88, `vb refine` run `refine-3236-20261018b` at 8431b7f65, gpt-oss-120b, 20 band C/D
tasks): every task's two rounds were kept, the mean score rose from 45.3 to 79.0, and the tasks with a hard fail went
from 2 to 11.
- HF4 (missing context file) was added on 10 tasks: rounds added `context.read_files` entries for files that are not
  in the repository, mostly a fixture task's own `files` (`src/config.rs`, `greet.sh`) and once `.next/types`.
- HF2 (vacuous verify) was added on 1 task: `speclint/fixtures/dynamic/red-on-base` T3 got the step
  `output=$(./greet.sh); printf "%s" "$output" | grep -q $'\n' && exit 1 || exit 0`, which ends in `|| exit 0`.

The live adapter's prompt contributes to the HF4s: it describes a read file as "a file the spec names, or one of its
files", and a task's own `files` may not exist before the task runs.

## Why it matters

Goal `proof`. S07 §4.3's gate refines weak specs before cheap execution, and H3's R arm (3238, 3239) measures what
refinement recovers. A refined spec that a hard fail blocks is worse than the one it replaced, and the score change
overstates what refinement bought: gap-942f88's cost result stands (a round costs the same whether it is kept or
not), but its score change is not evidence of better specs. Related: gap-942f88, gap-4a5109 (the H3 pilot).

## Where

- `benchmarks/viabilitybench/specops/refine.py::refine`: the monotonicity check (`after_score < before_score`), with
  no hard-fail comparison; `_rejection` holds the other guards.
- `benchmarks/viabilitybench/driver/refine_live.py::SYSTEM_PROMPT` (`refine-live-1`): the wording on read files, and
  nothing on what makes a verify step vacuous.

## Current state

The guards reject a round that deletes, reorders or edits a verify step, changes `files`, names a hidden-suite path,
adds without a source or to a field outside `ADDITIVE_FIELDS`, puts an item under a list, or lowers the score. The run's
records (`reports/refiner-cost/refine.jsonl` on gap-942f88's branch) hold every kept round's additions and each task's
hard fails before and after.

## Plan

1. In `refine`, reject a round whose candidate has a hard fail its input did not have ("adds a hard fail: HF4"),
   using the same `score_task` record the score comes from.
2. In the live prompt, name read files only among files that exist on the base, never a file the task creates, and
   say that a step ending in `|| true` or `|| exit 0` checks nothing; bump `PROMPT_VERSION`.
3. Test: a stub round that adds a vacuous step, and one that adds a missing read file, are rejected with the hard
   fail named, and the input spec comes back unchanged.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Speclint's fixtures name files of a synthetic workspace, so scoring them against the repository root reports HF4
  for files the fixture never had; that is how speclint scores them by default, and the guard should hold to it.
