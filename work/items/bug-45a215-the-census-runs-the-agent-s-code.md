+++
id = "bug-45a215"
kind = "bug"
title = "The census runs the agent's code with writes allowed outside its scratch: code the truth suite runs on the hidden cases can record them where a later run's agent, on the same instance, reads them"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-42b214 worker (claude-agent), 2026-10-17 (on macOS 26.4, common.sandbox.command(['sh', '-c', 'echo ... > <dir>/notes'], deny=sandbox.denied(secret, task, [results root])) run from a census scratch writes the file, rc 0; sandbox.command(['cat', '<dir>/notes'], deny=[secret, results root], network='none'), an agent's deny list, prints it)"
discovered_from = "bug-42b214"
anchors = ["benchmarks/viabilitybench/families/common/sandbox.py::profile", "benchmarks/viabilitybench/driver/census.py::run_census"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-42b214", "bug-935cfa", "gap-394f28", "gap-b94a2c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_census_code_cannot_write_outside_its_scratch' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_census_code_cannot_write_outside_its_scratch -q"
+++

## Problem

The census runs the agent's code after its session (the truth suite's runs, the visible re-run, and the audit
battery's re-runs) under `common.sandbox`, whose profile allows everything but the denied paths: `(allow default)`,
then `(deny file* ...)` for the secret file, the key file and, since bug-42b214, the whole results root. So that code
can write anywhere else the driver's user can: `/tmp`, `/Users/Shared`, the work root, the operator's home. A truth
suite hands it the hidden cases (the toy suite feeds them to the agent's `clamp` on stdin, F1 to its probes, F4 as
stores), and those are keyed by the secret, the family and the instance id (`hmac_seed.hidden_stream`): the same in
every run of the instance under that secret, whatever the arm or the seed. Code an agent leaves in its tree can record
them at census time in such a place, and a later run's agent on the same instance (another arm of the campaign, or
the next seed) can read them there, since no agent process is denied those places either, and special-case them.
Nothing records such a write: the census scans only its exports and its HOME (`census._finish`).

To see it (2026-10-17, macOS 26.4): `sandbox.command(["sh", "-c", "echo 'hidden case: clamp(57, -3, 41)' >
<dir>/notes"], deny=sandbox.denied(secret, task, [results]))`, the census's deny list, run from a census scratch,
writes the file (rc 0); `sandbox.command(["cat", "<dir>/notes"], deny=[secret, results], network="none")`, an agent's
deny list, prints it back.

## Why it matters

Goal `proof`. The census's verdicts are the labels (VS), and the comparison between arms rests on them: a later arm
that passes the hidden checks with cases an earlier arm's code recorded for it is not the arm that was measured, and
the record shows nothing. Under Will's decision 9 the coordinator decided that nothing benchmark-internal reaches an
agent (bug-3c921e's, bug-935cfa's and bug-42b214's Decisions); the hidden cases are the most internal of all. Changing
what the census's runs may write changes every arm's labelling, so it belongs before the pre-registration lock
(gap-394f28).

## Where

- `benchmarks/viabilitybench/families/common/sandbox.py::profile`: `(version 1) (allow default)` with denies; the only
  write rule is the Rust toolchain's read-only paths.
- `benchmarks/viabilitybench/driver/census.py::run_census` (the visible re-run, `_run_hidden`), every family's
  `hidden.py` that calls `sandbox.command`, and `benchmarks/viabilitybench/audit/battery.py::run_a2`/`run_b1`.

## Current state

Since bug-42b214 the census's runs cannot read the results root (`--deny` to the truth suites, `vb._agent_deny` to the
census, `battery.results_root` to the battery). Writes outside the denied paths have never been confined. The agents'
own processes can write those places too, which matters less: they never see the hidden cases.

## Plan

1. Give `sandbox.profile` a write rule: deny `file-write*` everywhere, then allow it under the places the code must
   write (its working directory, which is the census's export or a suite's temporary copy, its HOME and TMPDIR,
   `/dev/null`, `/dev/tty`). Check on macOS 26 that a later `(allow file-write* (subpath ...))` reopens a subpath of
   the denied `/` (a later allow does reopen a denied read subpath, checked 2026-10-17), and what Python, node (F6)
   and cargo (F7, whose target directory is in the suite's scratch) need to write.
2. Have each caller name its writable places: the census and the battery know their scratch and HOME, and a suite
   knows its temporary directory.
3. Add `test_census_code_cannot_write_outside_its_scratch` to `driver/test_driver.py` (macOS): the agent's clamp,
   whenever the truth suite and the visible re-run import it, writes a file outside the census's scratch and HOME;
   both writes are refused, and the census still labels the task.

## Done when

- [ ] No code the census runs can write outside its scratch, HOME and TMPDIR.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-42b214 worker, 2026-10-17, while denying the census's runs the results root.
- A container per task (S08 decision 4) would confine writes by construction.
