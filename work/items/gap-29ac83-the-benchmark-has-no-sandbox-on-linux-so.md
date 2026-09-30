+++
id = "gap-29ac83"
kind = "gap"
title = "The benchmark has no sandbox on Linux, so records there say sandbox: none"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench's report)"
anchors = ["benchmarks/viabilitybench/driver/census.py", "benchmarks/viabilitybench/driver/records.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-0bd49a", "gap-8c3752"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_sandbox_confines_on_linux' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_the_sandbox_confines_on_linux -q"
+++

## Problem

The benchmark's sandbox exists only on macOS: "On macOS it uses `sandbox-exec`. Elsewhere there is no confinement yet" (`driver/census.py:44`). Runs on Linux record `sandbox: none` (`records.py:122`). The obvious Linux tool, bubblewrap, needs unprivileged user namespaces, which Ubuntu runners restrict (AppArmor's `kernel.apparmor_restrict_unprivileged_userns`). So it doesn't work on the CI and cloud machines the pilot would use.

Two facts constrain the design:

- Apple has deprecated `sandbox-exec`. It still works on macOS 26.4, but it can go away.
- Code inside the sandbox can't exec setuid tools such as `ps`, so anything that needs them must run outside.

## Why it matters

Pilot benchmark (epic spec-567e52): results from Linux runs would come without the confinement S08 relies on (gap-8c3752 for the census, gap-0bd49a for agents).

## Where

The sandbox set-up in `census.py`, and the records' `sandbox` field.

## Plan

1. Pick a Linux mechanism that works on the runners: a container per task (Docker or Podman, S08 decision 4), bubblewrap on hosts that allow user namespaces with a clear refusal elsewhere, or a separate uid.
2. Keep macOS on `sandbox-exec` for now, and note its deprecation in S08.
3. Refuse (or clearly label) runs without a sandbox, so `none` is never silent.
4. Add `test_the_sandbox_confines_on_linux`, skipped where the mechanism is unavailable, with the record stating why.

## Done when

- [ ] Linux runs are confined, or are refused or labelled when they can't be.
- [ ] The `[[verify]]` command passes.
