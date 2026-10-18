+++
id = "bug-4f9e49"
kind = "bug"
title = "Every agent's process still has the driver as an ancestor, and the driver's command line names the experiment, the run id and the benchmark's path, with VB_DRIVER_ENV in its environment; from inside the sandbox an agent walks up to it with proc_pidinfo and reads it with sysctl(KERN_PROCARGS2)"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "bug-0fd744 worker (claude-agent), 2026-10-18 (driver/test_secret.py's DRIVER_PROBE, run as the direct loop's agent command inside the sandbox on macOS 26.4, walks up from the trampoline with proc_pidinfo PROC_PIDTBSDINFO and reads the driver's argv and environment with KERN_PROCARGS2: --experiment, --run-id, <repo>/benchmarks/viabilitybench/driver/vb.py and VB_DRIVER_ENV=scrubbed)"
discovered_from = "bug-0fd744"
anchors = ["benchmarks/viabilitybench/driver/agent_env.py::exec_scrubbed", "benchmarks/viabilitybench/driver/campaign.py::_run_unit", "benchmarks/viabilitybench/driver/vb.py::cmd_run"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-0fd744", "gap-b94a2c", "bug-4c3a83", "bug-3c921e", "bug-d68575", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_agent_ancestor_names_the_run' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_no_agent_ancestor_names_the_run -q"
+++

## Problem

Every agent process still has the driver as an ancestor, and the driver's command line and start-up environment name
the run. bug-0fd744 gave the direct loop's shell a neutral trampoline for its parent, and bug-4c3a83 made Claude Code's
own command line neutral, but the driver, `vb.py run`, is one step above the trampoline. It is also the parent of every
other arm's harness process (claude, codex, roko, mini), whose tool shells are its grandchildren, and of the census's
`hidden.py`, whose child the agent's code becomes. From inside the agents' sandbox, libproc's `proc_pidinfo`
(`PROC_PIDTBSDINFO`, `pbi_ppid`) gives any process's parent, and `sysctl(KERN_PROCARGS2)` its argv and start-up
environment: `driver/test_secret.py`'s `DRIVER_PROBE` walks up from the direct loop's shell to the driver and reads
both (checked on macOS 26.4, 2026-10-18).

The driver's argv is `<repo>/benchmarks/viabilitybench/.venv/bin/python <repo>/benchmarks/viabilitybench/driver/vb.py
run --experiment <id> --run-id <id> --stream ... --arm ... --results ... --work ...` (and `--secret-file`,
`--key-file` when given). `agent_env.exec_scrubbed` keeps it across its restart (`sys.orig_argv`), and a campaign
starts each unit that way with the unit's run id, which names its block (`campaign._run_unit`: `e-oss-1` is LOG1's
block E, the honeypots). Its environment holds `VB_DRIVER_ENV=scrubbed` and the passed-through `VB_*` settings, and on
a python.org framework build `__PYVENV_LAUNCHER__=<repo>/benchmarks/viabilitybench/.venv/bin/python`, which the
interpreter's own re-exec adds.

## Why it matters

Goal `proof`. Under Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier
reaches an agent (bug-3c921e's Decision), the benchmark's name included (bug-d68575), and the paths are search keys
into the public repository. The parents are neutral now (bug-4c3a83, bug-0fd744); the ancestor above them is not.
gap-b94a2c owns reading any same-user process's environment in general, but one of its options (accept it, and refuse
a run while a process exports a credential) would leave the driver's command line readable, and naming the run. Before
the pre-registration lock (gap-394f28), since it changes what the agents can learn.

## Where

- `benchmarks/viabilitybench/driver/agent_env.py::exec_scrubbed`: restarts the driver with `sys.orig_argv` and
  `driver_env()`, which marks it `VB_DRIVER_ENV=scrubbed`.
- `benchmarks/viabilitybench/driver/campaign.py::_run_unit`: the unit's `vb.py run` command line.
- `benchmarks/viabilitybench/driver/vb.py::cmd_run`: where the restart happens, once the checks have passed.

## Current state

The direct loop's shell's parent is the trampoline (bug-0fd744, `mini_loop.TRAMPOLINE`): the base interpreter as
`-I -S -`, with the agent's own environment. Claude Code's and Codex's command lines name no run (bug-4c3a83); Roko's
names its binary under the repository, which names no run or benchmark; mini-swe-agent's names its venv
(bug-e16450). None of that hides the driver above them.

## Plan

1. bug-0fd744's plan option 1: restart the driver (`exec_scrubbed`) under a neutral command line: the base interpreter
   outside the repository (`agent_env._base_python`), a bootstrap that reads the real argv, the venv's paths and the
   driver's path from a pipe or a private file with a random name that it deletes once read, and an environment
   without `VB_*` names (the restart's mark under a neutral name). Check every place that relies on `sys.executable`
   or `sys.orig_argv` (the campaign's units, the census's interpreter, `materialize`'s generators).
2. Or let gap-b94a2c's decision settle it: a separate uid or a container per task hides the driver as well, and this
   item closes as superseded.
3. Add `test_no_agent_ancestor_names_the_run` to `driver/test_secret.py`: a `vb run` started as a subprocess, with a
   campaign's run id, whose direct-loop agent walks up its ancestors to the test's own process (its pid in the
   command) and prints for each one only whether its argv or environment names the experiment, the run id, the
   results root or the benchmark (`BENCHMARK_MARK`), never their text; none does, and the walk reached the driver
   (the control).

## Done when

- [ ] No ancestor of an agent's process, up to the process that started `vb run`, names the experiment, the run id,
  the results root or the benchmark on its command line or in its environment.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-0fd744 worker, 2026-10-18, while checking what the trampoline leaves readable one step up.
- A probe must never print another process's argv or environment: above the driver are the test's own process and the
  operator's shell, which may carry credentials. `DRIVER_PROBE` prints only the ancestor that runs `vb.py`.
