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
