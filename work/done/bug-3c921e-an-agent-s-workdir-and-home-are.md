+++
id = "bug-3c921e"
kind = "bug"
title = "An agent's workdir and HOME are named by its instance id (<work>/<run_id>/F8-l3-0038.s1), so pwd tells it its family and ladder level"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "ae5be8f1c"
source = "bug-d7ff47 worker (claude-agent), 2026-10-17 (vb.py::_run_one's workdir and HOME paths; test_offline_run_writes_valid_records asserts the agent's own env shows HOME=<work>/run-1/_home/F1-l1-0001.s1)"
discovered_from = "bug-d7ff47"
anchors = ["benchmarks/viabilitybench/driver/vb.py::_run_one"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d7ff47", "bug-9f6252", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_never_sees_its_instance_id' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_never_sees_its_instance_id -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T06:29:44Z"
commit = "ae5be8f1c"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T05:23:23Z"
forced = false
evidence = "ae5be8f1c: no path an agent can see names its task or run. The run's work directory is $VB_WORK/<16 random hex> (the manifest's work_dir), not <run_id>, which a campaign names by its block (e-oss-1: LOG1's block E, the honeypots). Each task's workdir, HOME (and so TMPDIR) and census HOME are <work_dir>/<alias>, <work_dir>/_home/<alias> and <alias>.census, where the alias is 16 random hex digits (vb._alias), never a hash of the run id or the key, which an agent could invert by hashing the few, public keys. The record keeps it (provenance.alias), as do the task's errors.jsonl lines, and runners get it as TaskContext.alias (required). Roko's plan slug hashes it (PlanSpec.key renamed alias; run_roko, run_roko_plan), where it hashed the key, and the visible-verify wrapper's draws are keyed by it, so its config in HOME no longer holds the task key. The key stays on the driver's side: private/<key>/, archives/<key>.*, transcripts/<key>.json, the ledger's attempt keys and the proxies' task. Decided under Will's decision 9 (## Decision below): no benchmark-internal identifier reaches an agent, for benchmark validity and against contamination through the public repository. The verify passes: test_an_agent_never_sees_its_instance_id (driver/test_driver.py), 3 passed (direct, Roko and mini-swe-agent arms): an offline flaky_verify run with run id e-oss-1 and --keep-workdirs, whose agent prints pwd, HOME, TMPDIR and env, lists the work directory, _home, HOME, .vb-bin and plans/, and reads the wrapper's config, roko.toml and git log. No observation, and no name, link target or file under the kept work directory, holds an instance id, a task key, a key's plan slug, the run id or the experiment id. The aliases name the kept directories, the wrapper's key and Roko's plan and s01 records, and a second run with the same run id gives the same task another alias and work directory. On the previous driver it fails on all three arms; under five mutations (the old paths, alias = key, alias = sha256(key), the wrapper keyed by the key, Roko's slug from the key) it fails each time. test_offline_run_writes_valid_records' HOME assertion now checks the opposite (HOME is <work_dir>/_home/<alias>; the listing names neither the instance, the run nor the experiment). Bench pytest: 798 passed, 16 skipped (13 with no roko binary in the worktree, the analysis stack, two opt-in sweeps). Filed bug-d308f7 (p2: every arm's agent shell but Codex's can read the run directory outside private/, where proxy.jsonl and egress.jsonl name the running task's key and order-<seed>.json with records.jsonl give its position; checked under the agents' sandbox), bug-c1b04b (p2: planemit's tasks.toml and roko.toml headers tell only the Roko arms' agents that they face a ViabilityBench task with hidden checks) and bug-d68575 (p3: .vb-bin, vb-agent <agent@vb.invalid>, 'vb: pristine base', VB_SUBMIT and the flaky_verify wrapper's own source name the benchmark to every agent)."
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

- [x] No agent-visible path (the workdir, HOME, TMPDIR) holds the instance id or the task key.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-d7ff47 worker while checking what else in an agent's view names a family.
- `--keep-workdirs` users and anything that reads `<work>/<run_id>/<key>` by hand need the mapping.

## Decision (2026-10-17)

Whether a path an agent can see may carry its instance id, its task key or its run id had no recorded default. Under
Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the coordinator's recommendation,
the conservative option, recorded on the item with the reasoning; Will can veto it later), the coordinator decided to
hide every benchmark-internal identifier from agents: no path or file an agent can see names its instance, its task
key or its run. The change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. The instance id names the family and the
ladder level (an F8 instance is a honeypot), and a campaign's run id names its block (LOG1's `e-*` blocks hold the
honeypots, `c-vague-*` run the vague variant). So an agent that reads its path learns which family it faces and how
hard its instance is. That holds on every arm, and on the Claude Code arm without asking, since the CLI writes its
working directory into its own system prompt. The names are also search keys into the public repository and its
truth suites. Random names are the conservative option: they change no task repo, spec or visible test, nothing an
agent is graded on, only where the driver puts things. A hash of the key or of the run id would not do, since the keys
are few and their scheme is public. The cost: a flaky_verify run's draws are keyed by the alias, so another run of the
same spec draws flakes of its own. S08 and S06 ask only for seeded draws, and a run's draws still follow from its
spec's seed and its records.
