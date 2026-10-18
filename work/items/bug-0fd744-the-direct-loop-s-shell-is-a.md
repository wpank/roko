+++
id = "bug-0fd744"
kind = "bug"
title = "The direct loop's shell is a child of the driver, whose command line names the experiment, the run id and the benchmark's own path, and whose environment keeps VB_DRIVER_ENV; a campaign puts every unit's run id there, and the agent reads it from inside the sandbox"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-18
source = "bug-4c3a83 worker (claude-agent), 2026-10-17 (vb run started as a subprocess, as test_secret.py's test_the_driver_runs_with_a_scrubbed_environment starts it, on macOS 26.4: the direct loop's agent runs PS_PROBE, and its parent's argv and environment hold '--experiment TEST-SECRET', '--run-id run-1', 'benchmarks/viabilitybench', '--results' and 'VB_DRIVER_ENV=scrubbed')"
discovered_from = "bug-4c3a83"
anchors = ["benchmarks/viabilitybench/driver/agent_env.py::exec_scrubbed", "benchmarks/viabilitybench/driver/campaign.py::_run_unit", "benchmarks/viabilitybench/driver/mini_loop.py::run_command"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-4c3a83", "gap-b94a2c", "bug-d68575", "bug-3c921e", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_agent_parent_names_the_run' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_no_agent_parent_names_the_run -q"
+++

## Problem

In the direct loop the driver runs the agent's commands itself (`mini_loop.run_command`: `sandbox-exec` then
`bash -c`), so the shell's parent is the driver process. Its command line is the one `vb run` was started with:
`<repo>/benchmarks/viabilitybench/.venv/bin/python <repo>/benchmarks/viabilitybench/driver/vb.py run --experiment
<id> --run-id <id> --stream ... --arm ... --results ... --work ... --secret-file ...`, and `agent_env.exec_scrubbed`
restarts it with the same `sys.orig_argv`. A campaign starts every unit that way (`campaign.py::_run_unit`), with
the unit's run id, which names its block (`e-oss-1`: LOG1's block E, the honeypots). The scrubbed environment keeps
`VB_DRIVER_ENV=scrubbed` and the passed-through `VB_*` settings. From inside the agents' sandbox,
`sysctl(KERN_PROCARGS2, getppid())` returns that argv and environment (test_secret.py's `PS_PROBE` reads them in
`test_the_driver_runs_with_a_scrubbed_environment`). So the direct loop's agent can read its experiment id, its run
id, the benchmark's name and where its results and secret file are, and the other arms' agents reach the same
process one step further up.

## Why it matters

Goal `proof`. Under Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier
reaches an agent (bug-3c921e's Decision): neither its run id nor the benchmark's name (bug-d68575). bug-4c3a83 took
them off Claude Code's command line. The driver's own command line and environment are the same leak for the
direct loop's parent, and the paths are search keys into the public repository. Before the pre-registration lock
(gap-394f28), since it changes what the agents can learn.

## Where

- `benchmarks/viabilitybench/driver/agent_env.py::exec_scrubbed`: restarts the driver with `sys.orig_argv`.
- `benchmarks/viabilitybench/driver/campaign.py::_run_unit`: the unit's `vb.py run` command line.
- `benchmarks/viabilitybench/driver/mini_loop.py::run_command`: the direct loop's shell, a child of the driver.

## Current state

gap-b94a2c owns the mechanism in general: any same-user process's argv and environment are readable from inside the
sandbox, and only a separate uid or a container hides them. This item is narrower: what the driver itself shows as
its agents' parent and ancestor.

## Plan

1. Restart the driver (`exec_scrubbed`) under a neutral command line: the base interpreter outside the repository
   (`agent_env._base_python`), a bootstrap that reads the real argv, the venv's paths and the driver's path from a
   private file with a random name that it deletes once read, and an environment without `VB_*` names (the restart's
   mark under a neutral name). Check every place that relies on `sys.executable` or `sys.orig_argv` (the campaign's
   units, the census's interpreter, the Roko arm's `.venv-msa`).
2. Or run the direct loop's commands from a neutral helper process, and accept that its ancestor is still readable
   (gap-b94a2c).
3. Add `test_no_agent_parent_names_the_run` to `driver/test_secret.py`: the direct loop's agent reads its parent's
   argv and environment (`PS_PROBE`) in a `vb run` started as a subprocess; neither names the experiment id, the run
   id or the benchmark (`vb`, `viabilitybench`).

## Done when

- [ ] The direct loop's shell's parent carries neither the experiment id, the run id nor the benchmark's name on its
  command line or in its environment.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-4c3a83 worker, 2026-10-17, while checking every arm's parent command line (that item's plan,
  step 2).
- The interpreter's own path, `<repo>/benchmarks/viabilitybench/.venv/bin/python`, names the benchmark too: the
  restart must not exec it.

## Decision (2026-10-18, Will)

Approved: give the direct loop's agent shell a neutral launcher, so it is no longer a child whose argv and environment name the experiment and run. Land it before the pre-registration lock.
