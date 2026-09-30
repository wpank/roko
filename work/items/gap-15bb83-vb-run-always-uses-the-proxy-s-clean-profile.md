+++
id = "gap-15bb83"
kind = "gap"
title = "vb run always uses the proxy's clean profile, and the other disturbance hooks aren't built, so H6 has no disturbance mechanism"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "437951810"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-appAB's report)"
anchors = ["benchmarks/viabilitybench/driver/vb.py", "benchmarks/viabilitybench/driver/faultproxy.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e003ec", "dec-39c781"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_vb_run_applies_the_chosen_disturbance_profile' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_vb_run_applies_the_chosen_disturbance_profile -q"

[closed]
at = 2026-09-30
commit = "437951810"
by = "wk-bench-fix1"
evidence = "437951810: vb run --disturbance SPEC.toml (vb.disturbance/1, S06 §4.9's DisturbanceSpec list; driver/disturb.py) applies provider_fault (per-task proxy profile), budget_cut (task caps scaled, runner and proxy), harder_mix (level-first reorder from start_at) and convention_flip (gen.py --latent, checked against every family before the run; F1/F4 build v1 only, so it is refused today); model_swap and flaky_verify are refused as not built (they need the served-model checks to expect a swap, gap-dad97b, and an arm-side visible-verify wrapper). Records carry perturbations_active and each attempt's fault_injected; the hashed config carries the spec. None of the hooks waits on gap-0bd49a. Verify passes (test_vb_run_applies_the_chosen_disturbance_profile); full viabilitybench suite 334 passed, 2 skipped with the prebuilt roko."
+++

## Problem

The metering proxy has fault profiles besides `clean` (`faultproxy.py`, `PROFILE_PARAMS`: `http_5xx`, `rate_limit`, `hang`, …, :116), but `vb run` always starts it with the default `Profile()`, which is `clean` (vb.py:38, :350), and has no option to choose another. wk-rp-appAB reports that the other five disturbance hooks that S09's H6 needs aren't built either. So H6, how the arms cope with disturbances, has no way to inject any.

## Why it matters

Pilot benchmark (epic spec-567e52): H6 can't run until the driver can apply a disturbance per run or per task, and record which one it applied.

## Where

`cmd_run` and the proxy set-up in `vb.py`. The profiles are in `faultproxy.py`. The hooks are defined in S09's H6 design (see dec-39c781 for the open decisions).

## Plan

1. Add a `--disturbance` (or per-experiment manifest) option that selects a proxy profile and its parameters, and record it in each run record.
2. Build the other disturbance hooks H6 names, as separate small items if they are large.
3. Add `test_vb_run_applies_the_chosen_disturbance_profile`.

## Done when

- [ ] `vb run` can apply each of H6's disturbances, and the records say which one applied.
- [ ] The `[[verify]]` command passes.
