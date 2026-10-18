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
links = { depends_on = [], blocks = [], related = ["bug-4c3a83", "gap-b94a2c", "bug-d68575", "bug-3c921e", "gap-394f28", "bug-4f9e49"], supersedes = [], duplicate_of = "" }

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

- [x] The direct loop's shell's parent carries neither the experiment id, the run id nor the benchmark's name on its
  command line or in its environment.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-4c3a83 worker, 2026-10-17, while checking every arm's parent command line (that item's plan,
  step 2).
- The interpreter's own path, `<repo>/benchmarks/viabilitybench/.venv/bin/python`, names the benchmark too: the
  restart must not exec it.
- Fixed 2026-10-18 (plan option 2, Will's decision below). `mini_loop.run_command` no longer starts the shell itself:
  it starts a trampoline, the base interpreter (`agent_env._base_python`, outside the repository, the one the agent's
  own `python3` links to) run as `-I -S -`, with the agent's environment, and sends it its program
  (`mini_loop.TRAMPOLINE`) on stdin. The program holds the command line the driver used to start (`sandbox-exec`, then
  bash or the visible-verify wrapper) and the agent environment, as bytes. The trampoline starts that with
  `posix_spawn`, `/dev/null` on stdin and the signals `Popen` restores, waits for it, and reports on a pipe of its own,
  which the shell never inherits, `+` once the shell started, `=<code>` when it ended, or `-<error>` when it could not
  start. So the parent's command line is that interpreter and `-I -S -`, and its environment is the agent's own; the
  sandbox profile, with the deny paths, never appears on any command line an agent can read once the shell runs.
- Checked from inside the sandbox on macOS 26.4: the python.org framework interpreter re-executes itself as
  `Python.app`, so the trampoline's start-up environment also holds `__PYVENV_LAUNCHER__` (the base interpreter's own
  path) and `__CF_USER_TEXT_ENCODING`. The old parent, the driver, held `__PYVENV_LAUNCHER__=<repo>/benchmarks/
  viabilitybench/.venv/bin/python` as well, a path naming the benchmark.
- What did not change for the command: its output (both streams in one, in order), its exit code or the signal that
  killed it, its environment exactly, its working directory, no other file descriptor, SIGPIPE at its default, the
  submit word, the time limit and the kill of its whole session, background jobs included, and the message of a shell
  that cannot start. `driver/test_driver.py::test_the_shell_meets_the_same_under_its_trampoline` checks all of it and
  passes on the old driver too. What did: the shell is no longer its session's leader (the trampoline is, and the
  driver kills that session as before), and an agent that kills `$PPID` now ends its trampoline, not the whole run.
  Each command takes about 30 ms more to start.
- The driver stays readable one step further up, as every process of the operator's user does (gap-b94a2c). Its
  command line, which names the experiment, the run and the benchmark's path, is every arm's agents' ancestor:
  filed as bug-4f9e49 (p3). `driver/test_secret.py`'s `DRIVER_PROBE` reads it there, so
  bug-979a06's, bug-32eb77's and gap-a8a160's tests still show that the driver's start-up environment holds no key,
  no credential and no secret.

## Decision (2026-10-18, Will)

Approved: give the direct loop's agent shell a neutral launcher, so it is no longer a child whose argv and environment name the experiment and run. Land it before the pre-registration lock.

How to make the launcher neutral had no recorded default, so under Will's decision 9 of 2026-10-15 (technical
questions with no recorded default take the coordinator's recommendation, the conservative option, recorded on the item
with the reasoning; Will can veto it later) the change follows the coordinator's rule of 2026-10-17 (bug-3c921e's
Decision): no benchmark-internal identifier reaches an agent.

Why: benchmark validity, and contamination through the public repository. From inside the sandbox the direct loop's
agent read a parent whose command line named the experiment, the run id (a campaign's names its block: `e-oss-1` is
LOG1's block E, the honeypots), the results root and the benchmark's own path, and whose environment held
`VB_DRIVER_ENV` and the interpreter's path under `benchmarks/viabilitybench/.venv`. So it could learn which block and
which kind of task it faced, and the paths are search keys into the public repository, with its driver, generators and
truth suites. The trampoline is the conservative option: nothing an agent needs or is graded on changes (the
assertions above hold on the old driver and the new one), only what it can read of its parent. Its parent is the
interpreter the agent's own `python3` already is, with the environment the agent already has, so it learns nothing new
there. Plan option 1, restarting the driver itself under a neutral command line, would also hide the driver one step
further up, but it changes the driver's own start-up, the campaign's units and the census's interpreter, and it is
not what Will approved; it is bug-4f9e49's to decide.
