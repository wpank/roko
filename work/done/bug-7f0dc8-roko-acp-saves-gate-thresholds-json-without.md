+++
id = "bug-7f0dc8"
kind = "bug"
title = "roko-acp saves gate-thresholds.json without the update lock"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e0f472"
anchors = ["crates/roko-acp/src/runner.rs::record_observations", "crates/roko-acp/src/runner.rs::run_gates", "crates/roko-cli/src/runner/persist.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e0f472"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'save_gate_thresholds\\|thresholds.save' crates/roko-acp/src/runner.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:31Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

bug-e0f472 moved Graph runs' gate-threshold updates into a locked read-modify-write (`GateThresholds::update_locked`). `crates/roko-acp/src/runner.rs` (around line 1793) still loads, updates and saves the file without the lock, so an ACP session and a plan run can drop each other's observations. `persist.rs::maybe_flush_gate_thresholds` is also called only by its tests.

## Plan

Use the locked update in roko-acp, and delete `maybe_flush_gate_thresholds` or give it a caller.

## Done when

- The verify passes, and roko-acp's gate tests pass.

## Notes

- Reported on 2026-10-01 by the worker on bug-e0f472, during the evening close-out round.
- 2026-10-01 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  roko-acp's `run_gates` keeps the loaded `AdaptiveThresholds` only for its skip decisions. It records the
  gates' observations in one `roko_fs::with_locked_json_transaction` (`record_observations`), under the sibling
  lock that `GateThresholds::update_locked` takes, including after a failed compile, which was not saved before.
  roko-acp now depends on roko-fs (Cargo.toml and Cargo.lock). `maybe_flush_gate_thresholds` and its test are
  deleted from persist.rs: the Graph flush cadence is `GateThresholdWrites` (reg-c7ecf6). Test:
  `concurrent_gate_observations_all_reach_the_thresholds`. The two writers still store different schemas in
  the one file (`GateThresholdStats` vs `RungStats`), so each drops the other's extra fields when it saves.
