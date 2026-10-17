+++
id = "bug-42b214"
kind = "bug"
title = "The census runs the agent's code with the results root readable: only the secret file and the task's own .vb/ are denied, so code the truth suite imports can read this run's and every other run's archives, records and private/, another arm's archived solution of the same instance among them"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
source = "bug-935cfa worker (claude-agent), 2026-10-17 (on macOS 26.4, common.sandbox.command(['cat', ...], deny=sandbox.denied(secret, <run-2's task.json>), network='none') run from run 2's census scratch prints run 1's archives/F1-l1-0001.s1.diff and private/F1-l1-0001.s1/.vb/spec.precise.md, rc 0; only run 2's own .vb/task.json is refused)"
discovered_from = "bug-935cfa"
anchors = ["benchmarks/viabilitybench/driver/census.py::run_census", "benchmarks/viabilitybench/families/common/sandbox.py::denied"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-935cfa", "bug-d308f7", "bug-3bab88", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_census_code_cannot_read_another_run' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_census_code_cannot_read_another_run -q"
+++

## Problem

The census runs the agent's code after its session: the family's truth suite against the final tree, the visible
re-run and the audit battery (`census.py::run_census`). That code runs under `common.sandbox` with
`sandbox.denied(guard.path, manifest_path)`: the secret file and the task's own `private/<key>/.vb/`, and nothing else
of the results root. Its scratch, where the code runs, is `private/<key>/census/` inside the run directory. So code
the agent left in its tree, run by the truth suite, can read every other task and run in the results root: this
run's earlier tasks' `private/` (their manifests, canaries and precise specs) and `archives/`, and every other run's
`archives/<key>.diff`, `.tar.gz` and `.bundle`, records and `private/`. A campaign runs its units one after another
in one experiment directory (`campaign.py::_run_unit`), so a later arm's code could load an earlier arm's archived
solution of its own instance and pass the hidden checks with it.

To see it (2026-10-17, macOS 26.4): a results root with `EXP/run-1/archives/F1-l1-0001.s1.diff` and
`EXP/run-1/private/F1-l1-0001.s1/.vb/spec.precise.md`, and a census scratch `EXP/run-2/private/F1-l1-0002.s1/census/`;
`sandbox.command(["cat", ...], deny=sandbox.denied(secret, <run-2's task.json>), network="none")` run from that scratch
prints run 1's diff and its precise spec (rc 0); only run 2's own `.vb/task.json` is refused.

## Why it matters

Goal `proof`. bug-935cfa denied every agent process the whole results root during its session; the census is the
other time the agent's code runs, and its verdicts are the labels (VS). Code that reads another arm's passing
solution, or a run's hidden-check verdicts, contaminates the label that the comparison between arms rests on, and an
archive holds no canary that would show it. Under Will's decision 9 the coordinator decided that no
benchmark-internal identifier reaches an agent (bug-3c921e's, bug-d308f7's and bug-935cfa's Decisions). Changing what
the census's runs may read changes every arm's labelling, so it belongs before the pre-registration lock
(gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/census.py::run_census`: `deny = sandbox.denied(guard.path, manifest_path)`, and
  the scratch under the task's private directory.
- `benchmarks/viabilitybench/families/common/sandbox.py::denied` and `profile`.

## Current state

bug-d308f7 left the census as it was: its scratch, the code's working directory, is `private/<key>/census/`, so it
cannot deny the run directory as such. bug-935cfa denied the results root to the agent's own processes only.

## Plan

1. Move the census's scratch (its exports, the hidden and visible runs) out of the results root, into a driver temp
   directory with a random name, and deny the census's runs the whole results root as the agent's own processes are
   denied it; or keep the scratch and deny the results root except that scratch, if `sandbox-exec` lets a later allow
   rule reopen a subpath (check on macOS 26).
2. Keep what the census writes for the record (its outputs, the battery's logs) in the run directory, written by the
   driver's process after the runs end.
3. Add `test_census_code_cannot_read_another_run` to `driver/test_driver.py` (macOS): two runs in one experiment,
   the second's final tree holding code that the truth suite imports and that reads the first run's archive and
   private spec; both reads are refused and the census still labels the task.

## Done when

- [ ] No code the census runs can read anything under the results root but its own scratch.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-935cfa worker, 2026-10-17, while denying agent processes the results root.
- A lookup under a denied directory still tells an existing name from a missing one (bug-3bab88, held by Will).
