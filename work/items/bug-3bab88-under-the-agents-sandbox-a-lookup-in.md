+++
id = "bug-3bab88"
kind = "bug"
title = "Under the agents' sandbox a lookup in a denied directory fails with EPERM when the name exists and ENOENT when it does not, so an agent that probes <run_dir>/private/<key> for its stream's public keys finds its own task's"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d308f7 worker (claude-agent), 2026-10-17 (a toy run's agent under the agents' sandbox on macOS 26.4: cat <run_dir>/private/F1-l1-0001.s1, its own key, says Operation not permitted; cat <run_dir>/private/F1-l1-0002.s1, a task not yet run, says No such file or directory)"
discovered_from = "bug-d308f7"
anchors = ["benchmarks/viabilitybench/driver/vb.py::_run_one", "benchmarks/viabilitybench/families/common/sandbox.py::profile"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d308f7", "bug-935cfa", "bug-4c3a83", "bug-3c921e", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_cannot_probe_its_task_key' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_cannot_probe_its_task_key -q"
+++

## Problem

`common.sandbox.profile` denies every file operation under each denied path (`(deny file* (subpath ...))`). macOS
then answers a lookup under a denied directory by whether the name exists: an existing name fails with EPERM
("Operation not permitted"), a missing one with ENOENT ("No such file or directory"), since the lookup of a missing
name never reaches a vnode the rule covers. The run directory, which every agent process is denied since bug-d308f7,
holds entries named by the task key: `private/<key>/` for the running task and every earlier one, `archives/<key>.*`,
`transcripts/<key>.json` and `s01/<key>/` for the finished ones (`vb.py::_run_one`, `run_roko`). The keys are few and
public: a stream's instances are in `streams/*.toml`, and its order for a seed is a public HMAC shuffle
(`vb.Stream.order`, `hmac_seed.surface_stream`). So an agent that knows its run directory can probe each candidate
key, `cat <run_dir>/private/<key>`, and find the running task: the key whose private directory exists and whose
archive does not.

To see it (checked 2026-10-17 on macOS 26.4, with bug-d308f7's deny list): in a toy run, the running task's agent's
`cat <run_dir>/private/F1-l1-0001.s1` (its own key) prints "Operation not permitted", and
`cat <run_dir>/private/F1-l1-0002.s1` (a task not yet run) "No such file or directory".

## Why it matters

Goal `proof`, as bug-d308f7: the task key names the family (an F8 instance is a honeypot) and the ladder level. Under
Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier reaches an agent
(bug-3c921e's Decision). It changes the run directory's layout or the agents' sandbox, so before the pre-registration
lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/vb.py::_run_one`: `private / key`, `archive.archive_task(..., key)`,
  `transcripts/{key}.json`; `run_roko.run_task`'s `s01/<key>`.
- `benchmarks/viabilitybench/families/common/sandbox.py::profile`: the deny rules.

## Current state

The agents' deny list named `private/` (gap-0bd49a), probe-able the same way, while the rest of the run directory was
readable outright. bug-d308f7 denied the whole run directory; the names in it still answer a probe.

## Plan

Options:
1. Make the run directory unfindable: a random directory name inside the run's results, as the work directory has
   (bug-3c921e), with the results root denied (bug-935cfa) and no agent-visible path naming it (Claude Code's
   `--settings` names it on its argv: bug-4c3a83). An agent then has no path to probe.
2. Name the run directory's per-task entries by the task's alias (`private/<alias>/`, `archives/<alias>.*`, ...),
   with `provenance.alias` mapping them back; every reader (`vb report`, the replay and showcase bundles, the audit
   battery) must follow.
3. A sandbox rule that denies the lookup itself: none found (macOS 26.4 answered missing names with ENOENT).

Option 1 is the smaller change once the results root is denied. Add `test_an_agent_cannot_probe_its_task_key` to
`driver/test_driver.py` (macOS): an agent that probes every key of its stream under the paths it can name learns
nothing that differs between keys.

## Done when

- [ ] No probe of names an agent can make tells it which task keys a run holds.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-d308f7 worker, 2026-10-17, checking what its deny list leaves.
