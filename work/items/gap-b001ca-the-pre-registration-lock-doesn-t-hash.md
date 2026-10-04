+++
id = "gap-b001ca"
kind = "gap"
title = "The pre-registration lock doesn't hash experiments/ or arms/, and planemit's runs don't freeze learning (gap-394f28 preconditions)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "coordinator check of gap-394f28's preconditions at 09387c6fc"
discovered_from = "gap-394f28"
anchors = ["benchmarks/viabilitybench/analysis/lock.py", "benchmarks/viabilitybench/driver/planemit.py"]
lane = "bench"
links = { depends_on = [], blocks = ["gap-394f28"], related = ["gap-394f28", "gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_lock.py -q -k lock_hashes_experiments_and_arms"

[[verify]]
command = "benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -q -k emitted_config_freezes_learning"
+++

## Problem

Two of gap-394f28's preconditions for the ViabilityBench pre-registration lock (task 3345) are still open at
09387c6fc. `analysis/lock.py`'s `HASHED` is `("analysis", "audit", "streams", "requirements-analysis.lock",
"reports/simulation")`, so LOG1's cells, caps and models (`experiments/log1.toml`, `experiments/budget.toml`) and the
arm files (`arms/`) can change after the lock without being caught. And `driver/planemit.py`'s emitted roko.toml
has `[learning]` with only `auto_playbook_refresh` and `dream_on_completion` off: it doesn't set `frozen = true`
(decision 2218, gap-644040), so a real Roko run's S01 manifest lacks `learning_frozen` in `ablation_flags` and G2's
frozen-loop census (`analysis/gates.py::_frozen_loops`) fails on every `roko_fixed` and `fr_claude` run.

## Why it matters

The lock is what LOG1's confirmatory results are read against. Locking without these either lets the experiment
drift unseen or fails G2 on the first real runs.

## Where

`benchmarks/viabilitybench/analysis/lock.py` (`HASHED`, `lock.build`, its `status` check),
`benchmarks/viabilitybench/driver/planemit.py` (both config templates: openai_compat and claude_cli),
`analysis/gates.py::_frozen_loops`, and their tests (`analysis/test_lock.py`, `driver/test_planemit.py`).

## Current state

As above. The lock hasn't been taken (gap-394f28 holds it for Will's go-ahead), so the hash list can still change.

## Plan

1. Add `experiments` and `arms` to `HASHED`, excluding the lock file itself (`experiments/prereg.lock.json`) from
   its own hash list. Test: `lock_hashes_experiments_and_arms`.
2. Emit `frozen = true` under `[learning]` in every planemit config template, for the arms G2's census covers
   (`roko_fixed`, `fr_claude`); if an arm must learn during a run, make the freeze an explicit opt-out and say which
   arm. Test: `emitted_config_freezes_learning`. Check that the S01 manifest Roko writes for such a run carries
   `learning_frozen` (the field gap-644040 added), from the Rust source, not by running Roko.
3. Update gap-394f28's Plan to mark preconditions 2 and 3 done.

## Done when

- [ ] Both `[[verify]]` commands pass.
