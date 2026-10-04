+++
id = "gap-127263"
kind = "gap"
title = "A frozen run writes no episodes, so the bench driver sees no Roko attempts: planemit can't freeze learning yet (gap-394f28 precondition 3)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate 13b: gap-b001ca part 2 (7629f434f) reverted in 202fb29b2"
discovered_from = "gap-b001ca"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/planemit.py"]
lane = "bench"
links = { depends_on = [], blocks = ["gap-394f28"], related = ["gap-b001ca", "gap-394f28", "gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -q -k emitted_config_freezes_learning"

[[verify]]
command = "VB_REQUIRE_REAL_ROKO=1 benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_shakedown.py benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider"
+++

## Problem

G2's frozen-loop census (`analysis/gates.py::_frozen_loops`) needs every `roko_fixed` and `fr_claude` run to carry
`learning_frozen` in its S01 manifest, which only `[learning] frozen = true` (decision 2218) produces. But a frozen run
writes no `.roko/episodes.jsonl` (`crates/roko-core/src/config/learning.rs`, `frozen`'s doc: episodes are learned
state), and the bench driver reads each attempt from its episode (`driver/run_roko.py`: "an attempt has no episode"
-> `model_unverified`, "Roko recorded no attempt"). So the two can't both hold: gap-b001ca's planemit change (commit
7629f434f) made six real-roko bench tests fail at gate 13b, the shakedown's D1, D2 and D6 among them, and the gate
reverted it (202fb29b2).

## Why it matters

Without the freeze, G2 fails on every real LOG1 Roko run; with it, today's driver can't see Roko's attempts. Either
way LOG1 can't run, and the pre-registration lock (gap-394f28, precondition 3) waits on this.

## Where

`crates/roko-core/src/config/learning.rs` (`frozen`), the episode writer (`crates/roko-cli/src/runtime_feedback/
episodes.rs`), `crates/roko-cli/src/graph_execution/run_manifest.rs`, `benchmarks/viabilitybench/driver/run_roko.py`
(the evidence reader around `rows("episodes.jsonl")`), `driver/planemit.py`, and 7629f434f for the reverted change.

## Current state

At 202fb29b2: planemit emits no `frozen`; the six tests pass; G2's census would fail on real LOG1 runs.

## Plan

1. Pick one (the coordinator's suggestion is (a), since decision 2218 keeps a run's `.roko/runs/<run_id>/` files
   as telemetry):
   (a) A frozen run writes its episodes to `.roko/runs/<run_id>/episodes.jsonl` (telemetry, never read back by
       later runs) instead of `.roko/episodes.jsonl`, and the driver reads that file when the root one is absent;
   (b) the driver builds attempt evidence from S01's `runs/*/attempts.jsonl` and the efficiency rows when episodes
       are missing, and stops requiring an episode per attempt for frozen runs.
2. Re-apply 7629f434f (planemit's `is_frozen` and its golden fixture and template hash).
3. Run the real-roko bench tests and the shakedown with the freeze on.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Progress

- 2026-10-04 (w4-length): option (a), implemented on `work/gap-127263`; cargo verification deferred to the batch
  gate. Nothing read per-run episodes before, and the episode sink sees each attempt's run id (the settled verdict's
  `identity.run_id`), so (a) held.
  - 20eb73a96: a frozen run's facade holds one sink, `EpisodeSink::per_run`. It writes each episode to
    `.roko/runs/<run_id>/episodes.jsonl`, beside the run's attempt log; `.roko/episodes.jsonl` and every other
    learned state stay untouched. The sink is named `run_episodes`, so the wiring census still reports no learning
    sink. `roko learn telemetry check` (`LegacyRows`) joins a frozen run's own episodes too. Tests:
    `per_run_sink_writes_each_episode_to_its_runs_own_log`, `legacy_rows_read_a_frozen_runs_own_episodes`, and two
    updated plan_runner tests (`frozen_learning_run_writes_no_learned_state` now also reads the run's episodes,
    T1 plus T2 twice; `frozen_run_registers_no_learning_sinks` expects the one `run_episodes` sink).
  - 447a00e1c: `run_roko.read_evidence` reads `runs/*/episodes.jsonl` when the workspace's log holds none of the
    plan's episodes. `_save_evidence` already copies `runs/`, and `analysis/replay.py` now reads
    `runs/*/episodes.jsonl`. Python tests added; the bench driver and analysis suites pass (264 passed, 14 skipped).
  - a557864f0: 7629f434f re-applied (cherry-pick), with its tests. Verify 1 passes.
- Verify 2 needs a binary built from this branch. With a copy of the batch binary (d5f4f38d2, before the Rust
  change) it fails exactly as at gate 13b: D1, D2, D6, `test_real_roko_run_against_a_fake_provider` and
  `test_real_roko_gate_meets_the_flaky_verify_wrapper` each end `model_unverified: Roko recorded no attempt`, and
  the other 24 pass. The same binary's frozen D2 run shows `ablation_flags = ["learning_frozen"]` in its manifest
  and its attempt log in `runs/<run_id>/`, where the new sink writes.
- The driver side was checked on real Roko output. An unfrozen D2 run's records (from main's driver), with the
  episodes moved into `runs/<run_id>/episodes.jsonl`, settle to the same three attempts and no problems as from the
  workspace's log.
- Follow-ups: `roko run`'s one-task report reads `.roko/episodes.jsonl` since its start offset (`run.rs`,
  `task_episodes_since`), so under a frozen config it reports no turns, tokens or cost. bug-dd20bd (the workflow
  episode a frozen `roko run` still appends to the root) could use the same per-run log.

