+++
id = "bug-d58954"
kind = "bug"
title = "vb run checks --credential-fingerprint only after it has made the run directory, written the manifest and started the proxy, so a typo leaves a phantom run"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "2c8713247"
source = "gap-bccf58 worker (claude-agent), 2026-10-17"
discovered_from = "gap-bccf58: moving _start_proxy ahead of the manifest left the fingerprint check after both"
anchors = ["benchmarks/viabilitybench/driver/vb.py::cmd_run"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-3cfe4f", "gap-bccf58"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_bad_credential_fingerprint_creates_nothing' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_a_bad_credential_fingerprint_creates_nothing -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T04:48:53Z"
commit = "2c8713247"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T04:27:06Z"
forced = false
evidence = "2c8713247: vb.cmd_run parses and checks --credential-fingerprint beside the --experiment and --run-id checks, before run_dir.mkdir, and passes the same tuple on to Run; nothing before that point creates a file or directory (lock check, make_plan, admission, _secret_file, _outside_repo, secret.preflight and the runner's preflight only read). The verify passes: test_a_bad_credential_fingerprint_creates_nothing (driver/test_driver.py), 1 passed: four malformed values (xyz, nothex!!, seven hex characters, one bad among two), each with and without --proxy under a run id of its own, exit 2 and leave no results or work directory, and _start_proxy is never called; the same run id then runs with a valid upper-case fingerprint (exit 0, one completed record). On the previous vb.py the test fails (the proxy started for the four --proxy calls and the directories stayed). The item's offline repro (toy stream, cheap_direct, --provider-url http://127.0.0.1:9/v1, --credential-fingerprint xyz): before, exit 2 left results/TEST-FP/run-1/manifest.json and an empty work/run-1/, and the rerun without the flag failed with 'already exists; pick another --run-id'; now exit 2 leaves nothing and the rerun writes 1/1 records. test_run_cli.py's gap-3cfe4f fingerprint tests still pass. Bench pytest: 794 passed, 16 skipped (no roko binary, the analysis stack, opt-in sweeps)."
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
