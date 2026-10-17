+++
id = "bug-d58954"
kind = "bug"
title = "vb run checks --credential-fingerprint only after it has made the run directory, written the manifest and started the proxy, so a typo leaves a phantom run"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-bccf58 worker (claude-agent), 2026-10-17"
discovered_from = "gap-bccf58: moving _start_proxy ahead of the manifest left the fingerprint check after both"
anchors = ["benchmarks/viabilitybench/driver/vb.py::cmd_run"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-3cfe4f", "gap-bccf58"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_bad_credential_fingerprint_creates_nothing' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_a_bad_credential_fingerprint_creates_nothing -q"
+++

## Problem

`vb run` checks `--credential-fingerprint` (gap-3cfe4f) only after it has created the run and work directories,
written `manifest.json`, opened the ledger and started the metering proxy (`vb.cmd_run`). A malformed fingerprint
(shorter than 8 characters, or not hex) still exits 2, as a usage error should. But by then the run already exists:
`<results>/<experiment>/<run_id>/` holds a manifest for a run that never ran, the work directory stays, and an
in-process call (`vb.main([...])`, as the tests make) keeps the proxy's thread and log file open.

The other usage checks run first: `--experiment` and `--run-id` are matched against `ID_RE` before `run_dir.mkdir`, and
admission runs "before anything else: no directory, no workdir, no request" (module docstring).

## Why it matters

A typo leaves a phantom run behind. A rerun with the same `--run-id` fails with "already exists; pick another
--run-id". `vb report` reads every run directory of the experiment (`analysis/report.py::load_runs`), and its bundle
check wants each run's records to be exactly the runs its manifest planned, so the phantom fails `--check` for the
whole experiment until someone deletes it by hand.

## Where

- `benchmarks/viabilitybench/driver/vb.py::cmd_run`: the `fingerprints` loop that raises
  "--credential-fingerprint must be a sha256-hex prefix…" runs after `run_dir.mkdir`, `_write_json(manifest)` and
  `_start_proxy`.

## Current state

At the gap-bccf58 commit the loop sits between the proxy's start and `Run(...)`. Nothing earlier reads
`args.credential_fingerprint`. Reproduced on 2026-10-17, offline (`--provider-url http://127.0.0.1:9/v1`, toy stream,
cheap_direct): `vb run … --credential-fingerprint xyz` exits 2 and leaves `<results>/TEST-FP/run-1/manifest.json` and
an empty `<work>/run-1/`. The same command without the flag then exits 2 with "already exists; pick another --run-id".

## Plan

1. Parse and check the fingerprints with the `--experiment` and `--run-id` checks, before `run_dir.mkdir`, and pass
   the parsed tuple on to `Run`.
2. Add `test_a_bad_credential_fingerprint_creates_nothing` in `driver/test_driver.py`: `vb run` with
   `--credential-fingerprint xyz` exits 2 and leaves no run directory and no work directory.

## Done when

- [ ] A malformed `--credential-fingerprint` exits 2 before any directory, file or proxy exists.
- [ ] The `[[verify]]` command passes.

## Notes

- Found while moving `_start_proxy` ahead of the manifest for gap-bccf58, which left this check where it was.
