+++
id = "bug-3c921e"
kind = "bug"
title = "An agent's workdir and HOME are named by its instance id (<work>/<run_id>/F8-l3-0038.s1), so pwd tells it its family and ladder level"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d7ff47 worker (claude-agent), 2026-10-17 (vb.py::_run_one's workdir and HOME paths; test_offline_run_writes_valid_records asserts the agent's own env shows HOME=<work>/run-1/_home/F1-l1-0001.s1)"
discovered_from = "bug-d7ff47"
anchors = ["benchmarks/viabilitybench/driver/vb.py::_run_one"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d7ff47", "bug-9f6252", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_never_sees_its_instance_id' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_never_sees_its_instance_id -q"
+++

## Problem

`vb run` names each task's workdir and the agent's HOME by the task key, `<instance_id>.s<seed>`
(`vb.py::_run_one`: `workdir = work_dir / key`, `homes = [work_dir / "_home" / key, ...]`), and every arm starts its
agent there: `mini_loop` runs each command with `cwd=ctx.workdir` and that HOME, `run_cli` and `run_codex` start the
CLI session with `cwd=ctx.workdir`, `run_msa` sets mini-swe-agent's `environment.cwd` to it, and `run_roko` runs
`roko plan run` in `ctx.workdir`. So an agent that runs `pwd`,
`echo $HOME` or `env` reads its instance id, for example `<work>/<run_id>/F8-l3-0038.s1`: the benchmark family (F8,
the honeypot family), the ladder level (l3) and the instance number. `driver/test_driver.py::
test_offline_run_writes_valid_records` asserts that the agent's own `env | sort` output holds
`HOME=<work>/run-1/_home/F1-l1-0001.s1`. Claude Code, for one, also writes its working directory into its own system
prompt, so a CLI arm's agent is told the path without asking.

`planemit.plan_slug` already hashes the key into an opaque plan id because "the instance id names the ladder level,
which the direct arm's agent never sees"; the workdir path shows it anyway, on every arm.

## Why it matters

Goal `proof`. The same validity concern as bug-d7ff47 (family ids in F2, F3 and F5 module docstrings) and bug-9f6252
(F8's README said honeypot): an agent should not learn which family it faces, that an F8 instance is the honeypot
family, or the difficulty level it was given. Here it holds for every family and every arm. The fix changes the
driver's layout, not the task repos, but it changes what every agent sees, so it belongs before the pre-registration
lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/vb.py::_run_one`: `key`, `workdir`, `homes` (the agent's and the census's HOME).
- The runners only use `ctx.workdir` and `ctx.agent_env`; nothing in them needs the path to carry the key.

## Current state

Since the driver's first version. bug-d7ff47 removed the family ids from the F2, F3 and F5 task repos; the paths
around them still carry the instance id.

## Plan

1. In `vb._run_one`, name the workdir and both HOMEs by an opaque id of the key (for example
   `planemit.plan_slug(key)`, or an HMAC of it), and record the mapping where the agent never looks (the run
   directory). Keep `private/<key>`, `archives/<key>.*`, the proxy's task key and the records as they are: the agent
   never sees the run directory.
2. Update the tests that assert the old paths (`test_offline_run_writes_valid_records`' HOME line).
3. Add `test_an_agent_never_sees_its_instance_id` in `driver/test_driver.py`: an offline run whose agent prints
   `pwd` and `env`; no observation holds the instance id or the task key.

## Done when

- [ ] No agent-visible path (the workdir, HOME, TMPDIR) holds the instance id or the task key.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-d7ff47 worker while checking what else in an agent's view names a family.
- `--keep-workdirs` users and anything that reads `<work>/<run_id>/<key>` by hand need the mapping.
