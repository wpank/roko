+++
id = "bug-32eb77"
kind = "bug"
title = "Operator credentials that aren't arm keys, such as ANTHROPIC_API_KEY or GITHUB_TOKEN, stay readable by agents through the driver's environment"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix2's report on bug-979a06)"
anchors = ["benchmarks/viabilitybench/driver/secret.py::key_exposures", "benchmarks/viabilitybench/driver/vb.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["bug-979a06", "bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_driver_runs_with_a_scrubbed_environment' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_the_driver_runs_with_a_scrubbed_environment -q"
+++

## Problem

bug-979a06 keeps the arms' provider keys out of the driver's environment. `secret.key_exposures` (secret.py:471-484) refuses a run when an arm's `api_key_env` is set, or when a known key's value appears under any name.

Any other credential the operator's shell exports stays in the driver's environment: an `ANTHROPIC_API_KEY` that no arm uses, `GITHUB_TOKEN`, cloud credentials. Every agent runs as the same user, so it can read that environment with `ps -E -p $PPID` (macOS) or `/proc/<ppid>/environ` (Linux), as the driver's own docs say (secret.py:54).

## Why it matters

Pilot benchmark (epic spec-567e52): an agent could spend on an unmetered provider or act on GitHub with the operator's token, outside the benchmark's accounting and guard.

## Where

`key_exposures` and the driver's start-up in `vb.py`.

## Current state

At 7fa54b873 only arm keys and known key values are checked.

## Plan

1. At start, re-exec the driver with an allowlisted environment (PATH, HOME, LANG, TMPDIR, the `VB_*` variables it needs), so its exec-time environment holds no credential. A scrub after start doesn't help, because `ps -E` shows the start-up environment.
2. Or refuse to start while any variable that looks like a credential is set (`*_API_KEY`, `*_TOKEN`, `AWS_*`, …), listing them.
3. Add `test_the_driver_runs_with_a_scrubbed_environment`.

## Done when

- [x] No credential from the operator's shell is readable from any process an agent can inspect (the driver's process tree; other processes of the operator's user are out of a driver's reach, see Notes).
- [x] The `[[verify]]` command passes.

## Notes

- **Done 2026-09-29 (wk-bench-fix2), plan option 1.**
  - Once `vb run`'s checks pass, the process starts itself again with the same command line and process id
    (`agent_env.exec_scrubbed`, `os.execve`).
  - The new environment is cut to an allowlist (`agent_env.driver_env`): `PATH`, `HOME`, `USER`, `LOGNAME`, `TMPDIR`,
    `LANG`, `TZ`, `LC_*`, `SSL_CERT_FILE`/`SSL_CERT_DIR`, `CLAUDE_CONFIG_DIR` and the driver's four `VB_*` paths,
    plus `VB_DRIVER_ENV=scrubbed`, which makes the restart happen once.
  - The checks run on the operator's environment first, so a benchmark secret or provider key there is still refused
    rather than silently dropped.
  - `run_cli.py probe` does the same before it starts claude.
  - Only a script restarts itself (`vb.main(argv)` with an explicit argv, as the tests call it, never execs).
- **Evidence.** `test_the_driver_runs_with_a_scrubbed_environment` runs `vb.py` as a process whose environment holds
  `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`, a `DATABASE_URL` with a password and a marker:
  - The agent's `ps -E -p $PPID` shows `HOME`, `LANG` and the restart's mark, and none of them.
  - No record, transcript, archive or workdir holds one.
  - The same start with `CEREBRAS_API_KEY` set exits 2 before any request.
- **Out of a driver's reach (checked on macOS 26 while doing this):** `ps -E` shows the start-up environment of any
  process of the same user that is not an Apple system binary, not only the driver's: an orphaned `python3` started
  from another shell showed its marker. So a credential the operator's shell exports is still readable in that shell's
  other children (an editor, another session). Only a separate user or a container per task hides them (S08 decision
  4); until then, run the benchmark from a session that exports no credential. `agent_env`'s docstring says so.
