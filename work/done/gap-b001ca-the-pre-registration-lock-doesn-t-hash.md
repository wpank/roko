+++
id = "gap-b001ca"
kind = "gap"
title = "The pre-registration lock doesn't hash experiments/ or arms/, and planemit's runs don't freeze learning (gap-394f28 preconditions)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "a4962b9b1"
source = "coordinator check of gap-394f28's preconditions at 09387c6fc"
discovered_from = "gap-394f28"
anchors = ["benchmarks/viabilitybench/analysis/lock.py", "benchmarks/viabilitybench/driver/planemit.py"]
lane = "bench"
links = { depends_on = [], blocks = ["gap-394f28"], related = ["gap-394f28", "gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_lock.py -q -k lock_hashes_experiments_and_arms"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T02:47:34Z"
commit = "a4962b9b1"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T00:56:12Z"
forced = false
evidence = "Gate 13b (merged a4962b9b1): lock.py hashes experiments/ and arms/ without its own file (d16063767); verify lock_hashes_experiments_and_arms passes. Part 2 (planemit's frozen learning) was reverted at the gate (202fb29b2): frozen runs write no episodes, so the driver saw no attempts; it moved with its verify to gap-127263. Bench suite and shakedown 8/8 pass after the revert."
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

## Progress

- Part 1 (HASHED): implemented at d16063767. `HASHED` gained `experiments` and `arms`; `file_hashes` takes the
  lock's own path and excludes it, so a rebuild after the lock exists never hashes itself as drift. New test
  `test_lock_hashes_experiments_and_arms` (`analysis/test_lock.py`): both dirs are hashed, the lock file is not,
  `check()` stays clean right after the lock is written even though `experiments/` now holds it, and a real edit
  under either new dir still shows as drift. Verify passes.
- Part 2 (frozen learning): implemented at 7629f434f. `CONFIG_TAIL` gained a `frozen = {frozen}` field in
  `[learning]`; `is_frozen(spec)` resolves it to true for a pinned-mode arm (no rungs: roko_fixed, fr_claude) and
  false for a routed, ladder-mode one, unless `PlanSpec.learning_frozen` overrides it. Confirmed from the Rust
  source, not by running Roko, that `config.learning.frozen` is what a real run's S01 manifest reads into
  `ablation_flags = ["learning_frozen"]` (`crates/roko-core/src/config/learning.rs`,
  `crates/roko-cli/src/graph_execution/run_manifest.rs`). New test
  `test_emitted_config_freezes_learning_for_pinned_arms_not_ladder_mode` (`driver/test_planemit.py`). Since
  `CONFIG_TAIL` is shared by every mode, this one change is not purely additive: it moved the pinned-mode golden
  fixture (`testdata/planemit/pinned.roko.toml`) and the pinned `TEMPLATE_SHA256`, updated in both
  `test_planemit.py` and `test_run_roko.py`. Verify passes.
- Part 3: gap-394f28's Plan marks preconditions 2 and 3 done, at 69d62a9f2.
- No arm needs `learning_frozen`'s explicit opt-out today (checked `arms/*.toml`'s harness/models_allow): the
  5 roko-harness arms split cleanly into pinned (roko_fixed, fr_claude) and routed (roko_ladder, roko_plan,
  roko_full), matching G2's census exactly.

- Gate 13b (2026-10-04, coordinator): part 2 (planemit's frozen learning, 7629f434f) was reverted in 202fb29b2.
  A frozen run writes no episodes, so the driver reported every pinned-mode Roko run as model_unverified and six
  real-roko bench tests failed. Its verify moved to gap-127263, which reconciles G2's census with the driver.
