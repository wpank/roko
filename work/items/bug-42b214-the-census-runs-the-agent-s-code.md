+++
id = "bug-42b214"
kind = "bug"
title = "The census runs the agent's code with the results root readable: only the secret file and the task's own .vb/ are denied, so code the truth suite imports can read this run's and every other run's archives, records and private/, another arm's archived solution of the same instance among them"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-935cfa worker (claude-agent), 2026-10-17 (on macOS 26.4, common.sandbox.command(['cat', ...], deny=sandbox.denied(secret, <run-2's task.json>), network='none') run from run 2's census scratch prints run 1's archives/F1-l1-0001.s1.diff and private/F1-l1-0001.s1/.vb/spec.precise.md, rc 0; only run 2's own .vb/task.json is refused)"
discovered_from = "bug-935cfa"
anchors = ["benchmarks/viabilitybench/driver/census.py::run_census", "benchmarks/viabilitybench/families/common/sandbox.py::denied"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-935cfa", "bug-d308f7", "bug-3bab88", "gap-394f28", "bug-45a215"], supersedes = [], duplicate_of = "" }

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

- [x] No code the census runs can read anything under the results root but its own scratch.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-935cfa worker, 2026-10-17, while denying agent processes the results root.
- A lookup under a denied directory still tells an existing name from a missing one (bug-3bab88, held by Will).
- Fixed 2026-10-17 (the Plan's first option). The census's code runs are denied what every agent process is
  denied: `vb run` passes `census.run_census` its agents' deny list (`deny=vb._agent_deny(run)`: the secret file, the
  files the tripwire holds and the whole results root), the census adds it to the suite's own (`sandbox.denied`: the
  secret file and the task's `.vb/`) for the visible re-run, and hands the truth suite the rest as `--deny` (new in
  every family's `hidden.py` and the toy one, `common.sandbox.add_deny_argument`; `common` 1.5.0, so the families'
  `verifier_version` shows it). The census's scratch, where that code runs, moved out of the results root to
  `<work_dir>/<alias>.census` (beside its HOME `<work_dir>/_home/<alias>.census`), and the census refuses a scratch,
  HOME or TMPDIR inside a denied path (`infra_error`) rather than mislabel a task whose checks could not run. What the
  census reads under the root (the manifest, the pristine bundle, c_i's repository) it reads on the driver's side:
  git makes the exports outside the sandbox, and `hidden.py` reads its manifest before it starts the agent's code.
  The audit battery over a run directory denies the whole results root too (`battery.results_root`: the root, when
  the run's manifest places the directory at `<root>/<experiment_id>/<run_id>`, else the run directory itself), in
  place of the run's `private/` and `archives/` alone.
- The census's code runs can still write outside their scratch, where a later run's agent can read what they wrote,
  the hidden cases among it: filed as bug-45a215.
- The plan slice's own census (`families/plan_slice/slicekit.py census`) is an operator's command outside `vb run`,
  with no results root to deny; it denies the agent's code no file, only the network, and is unchanged.

## Decision (2026-10-17)

Whether the code the census runs (the truth suites' runs of the agent's code, the visible re-run, the audit battery's
re-runs) may read the results root had no recorded default. Under Will's decision 9 of 2026-10-15 (technical questions
with no recorded default take the coordinator's recommendation, the conservative option, recorded on the item with the
reasoning; Will can veto it later), the coordinator decided that nothing benchmark-internal reaches an agent: that
code is denied the whole results root, as every agent process is since bug-935cfa, and what the census itself must
read there it reads on the driver's side, never by reopening a path inside the sandbox. The change lands before the
pre-registration lock (gap-394f28).

Why: benchmark validity. The census's verdicts are the labels the comparison between arms rests on, and a campaign
runs its units one after another in one experiment directory, so code an agent left in its tree could have loaded,
at scoring time, an earlier arm's archived solution of its own instance, the precise spec of a vague-variant task, or
another run's hidden-check verdicts, and passed the hidden checks with them; an archive holds no canary that would
show it. Denying the root is the conservative option: no code the census runs needs anything under it (the census,
git and `hidden.py` read what is needed outside the sandbox), so honest code is labelled as before (the verdicts of
every family's reference, stub and planted gaming solutions are unchanged with the new deny list), and only what the
agent's code can read changes.
