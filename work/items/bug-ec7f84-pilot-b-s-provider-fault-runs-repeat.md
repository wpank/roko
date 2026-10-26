+++
id = "bug-ec7f84"
kind = "bug"
title = "Pilot B's provider-fault runs repeat roko_fixed's (arm, task, seed) key and share its cell, so vb report refuses PILOT-B once that optional block runs"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "gap-d9e9fe (vb report --pilot), 2026-10-26"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py::check_unique", "benchmarks/viabilitybench/analysis/metrics.py::cells", "benchmarks/viabilitybench/experiments/pilot_b.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242", "gap-d9e9fe", "gap-f3944c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_disturbed_runs_are_a_cell_of_their_own' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_disturbed_runs_are_a_cell_of_their_own -q"
+++

## Problem

A disturbed run and a clean run of the same arm share one report cell and one repeat key. Pilot B's optional
`provider-fault` block (`experiments/pilot_b.toml`) reruns `roko_fixed` on gpt-oss-120b at seed 1 on five pilot
instances, under `experiments/provider_fault.toml`. The `roko-fixed` block runs the same arm, model, instances and
seed. `vb run` writes `replicate = 0` on every record (`driver/records.py`). So once both blocks have run:

- `metrics.check_unique` raises on the repeated (arm, model, harness, task, seed, replicate) key, and
  `vb report --experiment PILOT-B`, `report.py --check` and `report.py --pilot` all refuse the experiment
  ("arm roko_fixed ran F4-l5-0002 (precise) with seed 1 and replicate 0 twice ...; a repeat is not an independent
  run", reproduced in memory on 2026-10-26 with two Pilot B-shaped records).
- `metrics.cells` puts both runs in one cell, `(roko_fixed, None, "")`, so the fault runs' outcomes would also
  enter `roko_fixed`'s VS rate, $/VS and pass^3.

The records already say which runs were disturbed: `stream.perturbations_active` (for example `["provider_fault"]`),
and the run manifest's `config.disturbances`.

## Why it matters

- Running the optional row, which G0's fault-rate check needs, would break Pilot B's report and the pilot page
  (gap-d9e9fe) until the five fault runs are moved out of the experiment.
- If the key were relaxed without splitting the cell, the fault runs would silently skew `roko_fixed`'s numbers.

## Where

- `benchmarks/viabilitybench/analysis/metrics.py`: `check_unique`, `with_models` and `cells`.
- Possibly `driver/records.py` (`replicate`) or the manifest, depending on the fix.
- `experiments/test_pilot_b.py` rehearses the block offline but never reports the experiment.

## Current state

Checked at 15335e2f9 (2026-10-26). The provider-fault block has not run: Cerebras answers 402 (gap-f3944c), so
nothing is broken yet.

## Plan

Choose one, then test it with Pilot B-shaped records:

1. **A cell per disturbance.** Key the cell and the repeat check on the disturbance as well, from
   `stream.perturbations_active`, as bug-40de03 did for the harness. A disturbed run then gets its own row, for
   example `roko_fixed [provider_fault]`.
2. **A separate experiment id** for the fault runs (for example `PILOT-B-FAULT`), set in the manifest.

Add the offline rehearsal's `vb report --experiment PILOT-B` step to `experiments/test_pilot_b.py`.

## Done when

- [ ] A Pilot B-shaped set of records with both blocks reports without a repeat error, and the fault runs are
      not in `roko_fixed`'s clean numbers.
- [ ] The `[[verify]]` command passes.

## Notes

- Found while building `vb report --pilot` (gap-d9e9fe), which merges Pilot B's bundle.
