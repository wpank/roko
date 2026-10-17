+++
id = "bug-d308f7"
kind = "bug"
title = "An agent's shell can read its run's results directory outside private/: proxy.jsonl and egress.jsonl name the running task's key, order-<seed>.json and records.jsonl give its position"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-3c921e worker (claude-agent), 2026-10-17 (vb.py::_agent_deny denies <run_dir>/private alone; on macOS 26.4 common.sandbox.command under that deny list cats <run_dir>/proxy.jsonl and order-1.json)"
discovered_from = "bug-3c921e"
anchors = ["benchmarks/viabilitybench/driver/vb.py::_agent_deny"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-3c921e", "gap-394f28", "gap-0bd49a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_cannot_read_its_run_directory' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_cannot_read_its_run_directory -q"
+++

## Problem

Every agent process is denied the secret file, the key file and the run's `private/` directory
(`vb.py::_agent_deny`), and nothing else of the run directory, `<results>/<experiment>/<run_id>/`. The direct loop's
shell, the Roko and mini-swe-agent arms' process trees and Claude Code's Bash tool all run under that list
(`ctx.deny`). Only the Codex arm denies the whole run directory (`run_codex.denied`: `ctx.ledger.path.parent`), and
Claude Code's `--settings` keep it from Claude's file tools only (`run_cli.settings`), not from its shell. The run
directory tells an agent which task it is on:

- `proxy.jsonl` (the metering proxy) and `egress.jsonl` (the CLI arms' egress proxy) get one row per model call with
  `"task": "<instance_id>.s<seed>"`, written while the task runs, so the newest row names the running task's key;
- `order-<seed>.json` holds the stream's order and `records.jsonl` one row per finished task, so together they give
  the running task's position, and so its instance id;
- `manifest.json` names the stream, its instances and the experiment, and its `work_dir` matches the agent's `..`.

An agent can find the directory: the default results root is `~/.roko-bench/viability` in the operator's real home,
which the default work root `~/vb-work` puts in the agent's own `pwd`, and USER is passed through. To see it, on
macOS: `common.sandbox.command(["cat", "<run_dir>/proxy.jsonl"], deny=[<run_dir>/private], network="none")` prints the
file, and the same for `order-1.json`; `<run_dir>/private/<key>/...` is refused (checked 2026-10-17, macOS 26.4).

## Why it matters

Goal `proof`. bug-3c921e took the task key and the run id out of every path an agent can see, and bug-d7ff47,
bug-9f6252 and bug-b5eab4 out of the task repos, so that an agent cannot learn its family (F8 is the honeypots) or
its ladder level. An agent that looks for its run directory still reads its task key there, while it runs. Under
Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier reaches an agent
(bug-3c921e's Decision). Changing what agents may read changes every arm's sandbox, so it belongs before the
pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/vb.py::_agent_deny`: `run.run_dir / "private"`, where the whole `run.run_dir`
  is needed.
- `benchmarks/viabilitybench/driver/run_codex.py::denied` already denies the run directory, and
  `benchmarks/viabilitybench/driver/run_cli.py::settings` denies it to Claude Code's file tools.

## Current state

The deny list has held the secret file, the key file and `private/` since gap-0bd49a and gap-308373. Nothing that an
agent process runs needs the run directory: the proxies, Roko's evidence copy (`run_roko._save_evidence`), the
ledger, the records and the transcripts are written by the driver's own process, and the visible-verify wrapper logs
under the agent's HOME (`.vb-bin/vb-verify.jsonl`).

## Plan

1. Deny the whole run directory in `_agent_deny` (`run.run_dir`). Check that no agent process writes or reads there
   (each runner's own files, Roko's `.roko/`, mini-swe-agent's trajectory, which lies in a driver temp directory).
2. Off macOS nothing applies the deny list (`sandbox.KIND` "none", gap-29ac83); say so where the README describes
   isolation.
3. Add `test_an_agent_cannot_read_its_run_directory` to `driver/test_driver.py` (macOS): an offline run through
   `--proxy` whose agent cats `<run_dir>/proxy.jsonl`, `order-1.json`, `records.jsonl` and `manifest.json`; each read
   is refused, and no observation holds a task key.

## Done when

- [x] No agent process of any arm can read a file of its run directory.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-3c921e worker, 2026-10-17, while choosing where to keep the mapping from aliases to task keys.
- The census and the audit battery run the agent's code after its session with the key in their own paths
  (`private/<key>/census/`, `vb-battery-*/<key>/`); bug-3c921e left those, as its item said. The census cannot deny
  the run directory to that code anyway: its scratch, the code's working directory, is `private/<key>/census/`.
- What the deny list leaves (checked 2026-10-17 on macOS 26.4, filed separately): an agent can still list
  `<results>/<experiment>/`, so the run ids, and read every other run there, their agents' final diffs, records and
  `private/` among them (bug-935cfa); under a denied directory a missing name fails with ENOENT where an existing one
  fails with EPERM, so probing `<run_dir>/private/<key>` for the stream's public keys finds the running task's
  (bug-3bab88); and Claude Code's inline `--settings` put the run directory's path on its argv, which the agent's
  shell can read (bug-4c3a83).
- One test fixture relied on writing the run directory from inside the agents' sandbox: test_run_roko.py's fake roko
  played the metering proxy by appending `proxy.jsonl` itself. The proxy writes that log from the driver's process,
  so the test now writes it there (`test_plan_slice_records_carry_queue_waits_and_class_costs`).

## Decision (2026-10-17)

Whether an agent process may read its run's results directory outside `private/` had no recorded default. Under
Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the coordinator's
recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it later), the
coordinator decided that no benchmark-internal identifier reaches an agent: every agent process is denied its whole
run directory, as the Codex arm's already was. The change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. While a task runs, the proxies' logs name
its key, `<instance_id>.s<seed>`, and the order and the records give its position, so an agent that reads its run
directory learns its family (an F8 instance is a honeypot) and its ladder level, which bug-3c921e took out of every
path it can see. The key is also a search key into the public repository, where the family's generator and truth
suite are. Denying the directory is the conservative option: no agent process reads or writes there (the proxies,
the ledger, the records, the transcripts and Roko's evidence copy are the driver's own), so the change takes away
nothing an agent needs or is graded on, only what it can read.
