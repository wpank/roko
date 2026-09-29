+++
id = "bug-32eb77"
kind = "bug"
title = "Operator credentials that aren't arm keys, such as ANTHROPIC_API_KEY or GITHUB_TOKEN, stay readable by agents through the driver's environment"
status = "open"
triage = "unverified"
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

- [ ] No credential from the operator's shell is readable from any process an agent can inspect.
- [ ] The `[[verify]]` command passes.
