+++
id = "gap-e2d2e5"
kind = "gap"
title = "Verify steps whose phase is not compile, clippy or test (acceptance, unit_test, structural, check …) never move the gate thresholds, and nothing says so"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/runner", "roko-gate/registry"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#S1"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-cli/src/runner/persist.rs::GateThresholds::observe_verify_steps", "crates/roko-gate/src/registry.rs::rung_for_gate_name"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn verify_step_named_acceptance_moves_the_test_rung' crates/roko-cli/ && cargo test -p roko-cli verify_step_named_acceptance_moves_the_test_rung"
+++

## Problem

Gate-threshold learning (`.roko/learn/gate-thresholds.json`, loop L-gate-thr) learns only from verify steps whose
`phase` names a gate that the registry maps to a rung: `compile`, `clippy` or `test`, plus their `:cargo` aliases.
Every other phase is skipped, and nothing says so.

Round v6 dogfood run 3 (2026-10-16, roko 47b3cdd20, with `[learning] gate_threshold_flush_interval = 1`) shows it:
- The pocket-synth showcase plan
  (`tmp/hermes-roko-v6/plans-research/showcases/pocket-synth/plans/pocket-synth/tasks.toml`) tags every check
  `phase = "acceptance"`, though each one runs `python3 -m unittest …`.
- Its 12 attempts and its plan-level check moved no EMA. Their verdict rows say `rung: "custom:acceptance"`.
- The only observation in `gate-thresholds.json` came from a prompt run whose generated check was `verify[0:test]`:
  rung 2, 1 pass of 1.
- `roko learn loops` printed `L-gate-thr  flagged  dormant:no_opportunity`.

The same holds beyond the showcase:
- `roko plan import --mori` writes mori's `type` as the phase, `unit_test` for example.
- Existing plans use `structural`, `check`, `smoke`, `content` and others, none of which map.
- `roko plan validate` says nothing about any of it.

## Why it matters

Goal `learning`: "thresholds move" is one of the loops round v6 wants to show on camera, and it is dormant on exactly
the plans used to show it. A plan author has no way to know that the phase name decides this.

## Where

- `crates/roko-cli/src/runner/persist.rs::GateThresholds::observe_verify_steps` skips any step whose phase
  `rung_for_gate_name` does not map.
- `crates/roko-gate/src/registry.rs::rung_for_gate_name` and `GATE_SPECS` form the single name-to-rung table:
  `compile`, `clippy` and `test`, with their `:cargo` aliases.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs` (`verdict_step_rung`) labels an unmapped phase
  `custom:<phase>`.
- `crates/roko-cli/src/plan_validate.rs` is where a warning would go.

## Current state

Skipping is right for truly custom checks, such as a judge or a file-exists check. What goes wrong is that test-like
phases with other names are skipped too, and silently.

## Plan

1. Map test-like phase names to the Test rung. Either:
   - add registry aliases such as `acceptance`, `unit_test`, `unit`, `tests` and `integration` (and decide on
     `check`), or
   - classify a step by its command when its phase is unknown: a recognised test runner such as `cargo test`,
     `python -m unittest`, `pytest`, `go test` or `npm test` means the Test rung.
2. Have `roko plan validate` warn once per plan when no verify step feeds gate learning. For example: "no verify step
   names a phase gate learning knows: compile, clippy, test …".
3. Add the test `verify_step_named_acceptance_moves_the_test_rung`: an `acceptance` step that runs a test command
   updates rung 2's EMA in `observe_verify_steps`.

## Done when

- The `[[verify]]` passes.
- Re-running the pocket-synth showcase moves rung 2.

## Notes

- A registry change also changes the verdict rows' rung label (`verify:<i>/acceptance`) and the metric labels. Check
  that the opportunity count `roko learn loops` gives for L-gate-thr follows.
- The round-v6 showcase plans can use `phase = "test"` today. That is plans-research's call, independent of this item.
- Evidence: `tmp/hermes-roko-v6/dogfood-will/evidence/run3/learn/gate-thresholds.json` and
  `tmp/hermes-roko-v6/dogfood-will/logs/learn-loops-after.txt`.
