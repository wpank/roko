+++
id = "bug-35c901"
kind = "bug"
title = "ACP and Graph write gate-thresholds.json with different schemas, so each writer drops the other's fields"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-acp", "roko-cli/runner"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-7f0dc8"
anchors = ["crates/roko-acp/src/runner.rs", "crates/roko-cli/src/runner/persist.rs::GateThresholds", "crates/roko-gate/src/adaptive_threshold.rs::RungStats"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7f0dc8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib gate_thresholds_schema"
+++

## Problem

roko-acp's `AdaptiveThresholds` and the Graph path's `GateThresholds` are different schemas persisted to the same `gate-thresholds.json`. When either saves, it drops the fields only the other knows, even under the shared lock (bug-7f0dc8).

## Plan

Use one schema, or keep unknown fields round-trip (`#[serde(flatten)] extra`). Add a test named `gate_thresholds_schema_*` that loads and saves one writer's file with the other and checks no field is lost.

## Done when

- `cargo test -p roko-acp --lib gate_thresholds_schema` passes.

## Notes

- Reported on 2026-10-01 by wk-honestbench, working on bug-7f0dc8, during the evening close-out round.
- 2026-10-01 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  Unknown fields now survive a round trip through either type. `GateThresholds` and `GateThresholdStats`
  (roko-cli `persist.rs`) and roko-gate's `RungStats` each get a `#[serde(flatten)] extra` map. The Graph path
  keeps roko-acp's CUSUM settings, SPC detectors, pass streaks and poisoning-defense state, and roko-acp keeps
  `pass_count`. The rung count is stored as `total_count` by one writer and `total_observations` by the other;
  both read either name through their existing aliases. Tests: `gate_thresholds_schema_keeps_graph_fields`
  (roko-acp) and `gate_thresholds_schema_keeps_acp_fields` (roko-cli). Still open: each writer leaves the other's
  fields as they were, so roko-acp's pass streak does not see Graph observations. Only one schema would fix that.
