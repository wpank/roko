+++
id = "bug-935cfa"
kind = "bug"
title = "An agent can list its run's experiment directory and read every other run in the results root: the run ids, other runs' final diffs (archives/), their records with hidden-check verdicts, and their private/ with the precise spec"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d308f7 worker (claude-agent), 2026-10-17 (two vb runs of the toy stream in one experiment: under the agents' sandbox on macOS 26.4 the second's agent lists <results>/EXP and reads run-1's archives/F1-l1-0001.s1.diff, records.jsonl and private/F1-l1-0001.s1/.vb/task.json)"
discovered_from = "bug-d308f7"
anchors = ["benchmarks/viabilitybench/driver/vb.py::_agent_deny", "benchmarks/viabilitybench/driver/run_cli.py::settings", "benchmarks/viabilitybench/driver/run_codex.py::denied"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d308f7", "bug-3bab88", "bug-4c3a83", "bug-3c921e", "gap-394f28", "gap-0bd49a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_cannot_read_another_run' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_cannot_read_another_run -q"
+++

## Problem

Every agent process is denied its own run directory (`vb.py::_agent_deny`, since bug-d308f7) and nothing else of the
results root (`$VB_RESULTS`, default `~/.roko-bench/viability`), so an agent can list the experiment directory and read
every other run in the results root, `private/` included:

- `ls <results>/<experiment>/` lists the run ids, its own among them; a campaign's run id names its block (`e-oss-1`:
  LOG1's block E, the honeypots), which bug-3c921e kept out of every path an agent sees;
- another run's `archives/<key>.diff`, `.tar.gz` and `.bundle` hold that run's agent's final tree for the instance:
  in a campaign, perhaps another arm's passing solution of the agent's own task;
- its `records.jsonl` names each instance and its VS label, and the hidden checks it failed (`vs.failed`);
- its `private/<key>/.vb/` holds the task manifest (with the canary), `spec.precise.md` (so a vague-variant run can
  read the precise spec) and the pristine bundle; `transcripts/`, `proxy.jsonl` and `ledger.jsonl` as well.

A campaign runs its units one after another in one experiment directory (`campaign.py::_run_unit`), so every unit
after the first has earlier runs beside it. To see it (checked 2026-10-17 on macOS 26.4, with bug-d308f7's deny list):
two `vb run`s of the toy stream in one experiment, the second's agent running `ls <results>/EXP`,
`cat <results>/EXP/run-1/archives/F1-l1-0001.s1.diff`, `head -c 300 <results>/EXP/run-1/records.jsonl` and
`head -c 120 <results>/EXP/run-1/private/F1-l1-0001.s1/.vb/task.json`. Each succeeds: the listing shows `run-1` and
`run-2`, the diff is run 1's agent's solution, the record and the manifest name the instance. Only the canary in
`task.json` would show in a census, and only if the agent printed it.

## Why it matters

Goal `proof`. Runs contaminate each other: a later run's agent can copy an earlier agent's final diff of its own
instance, learn which hidden checks failed, read the precise spec while it runs the vague one, and read every run id.
Under Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier reaches an agent
(bug-3c921e's and bug-d308f7's Decision). Changing what agents may read changes every arm's sandbox, so it belongs
before the pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/vb.py::_agent_deny`: denies `run.run_dir`, where the results root is needed.
- `vb.py::cmd_run`'s `results_root` (`--results`, `$VB_RESULTS`, `DEFAULT_RESULTS`), which `_outside_repo` checks
  against the repository and `~/.roko` only, and against the work root.
- `benchmarks/viabilitybench/driver/run_cli.py::settings` and `run_codex.py::denied`, which deny the run directory
  too (Claude Code's file tools; Codex's sandbox).

## Current state

bug-d308f7 denied each agent its own run directory, whose proxy logs name the running task's key. The rest of the
results root has never been denied; the old deny list named only the current run's `private/`.

## Plan

1. Deny the results root, not the run directory, in `_agent_deny` (`Run` needs the root), in `run_cli.settings` and
   in `run_codex.denied`.
2. Refuse, before anything runs, a results root that holds what agent processes need: the repository (the Roko
   binary, `.venv-msa/`), the work root (already refused), the Rust toolchain (`toolchain.read_only()`), the base
   interpreter (`agent_env._base_python`), or the operator's home itself.
3. Add `test_an_agent_cannot_read_another_run` to `driver/test_driver.py` (macOS): two runs in one experiment, the
   second's agent lists the experiment directory and reads the first run's archive, records, task manifest and spec;
   each is refused, and nothing it printed names an instance.

## Done when

- [x] No agent process can list or read anything under the results root.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-d308f7 worker, 2026-10-17, checking what its deny list leaves.
- A name under a denied directory still answers a lookup with ENOENT or EPERM by whether it exists: the existence
  oracle, bug-3bab88. Claude Code's argv names the run directory: bug-4c3a83.
- Fixed 2026-10-17. `vb._agent_deny` denies `Run.results_root` (new), which holds the run directory; `ctx.deny`
  carries it to every arm's sandbox, so the direct loop's shell and the Roko, mini-swe-agent, Claude Code and Codex
  sessions are all denied it. `run_cli.settings` denies Claude Code's file tools each directory of `ctx.deny` in
  place of the run directory, and `run_codex.denied` takes the root from `ctx.deny` instead of the ledger's
  directory. `vb run` refuses, before it makes a directory, a results root that holds the repository, the Rust
  toolchain the sandbox keeps readable, the Python agents run or the operator's home (`vb._results_hold`); the work
  root was refused already.
- The census still runs the agent's code with only the secret file and the task's own `.vb/` denied: filed as
  bug-42b214.

## Decision (2026-10-17)

Whether an agent process may read the rest of the results root, the other runs beside its own, had no recorded
default. Under Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the coordinator's
recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it later), the
coordinator decided that no benchmark-internal identifier reaches an agent: every agent process is denied the whole
results root, not only its own run directory. The change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. A campaign runs its units one after
another in one experiment directory, so a later run's agent could read every run id (a campaign's names its block,
`e-oss-1` the honeypots), copy an earlier agent's final diff of its own instance, learn which hidden checks failed,
and read the precise spec while it runs the vague one. The run ids and the task keys in the archives' names are also
search keys into the public repository. Denying the root is the conservative option: no agent process reads or
writes there (the driver writes the proxies' logs, the ledger, the records, the archives and the transcripts), and
`vb run` refuses a root that would take from agents what they need, so the change takes away nothing an agent needs
or is graded on, only what it can read.
