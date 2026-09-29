+++
id = "gap-bbbfbc"
kind = "gap"
title = "Gate payloads outside Graph verify drop [gates] env_passthrough"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/gate_dispatch", "roko-acp"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/hermetic-child-env dc99a9e81"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::gate_signal", "crates/roko-acp/src/runner.rs::build_gate_signal", "crates/roko-gate/src/gate_service.rs:238"]
links = { depends_on = [], blocks = [], related = ["bug-7d7200"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "{ ! grep -q 'fn gate_signal(' crates/roko-cli/src/runner/gate_dispatch.rs || grep -q '\\.with_env_passthrough(' crates/roko-cli/src/runner/gate_dispatch.rs; } && { ! grep -q 'fn build_gate_signal(' crates/roko-acp/src/runner.rs || grep -q '\\.with_env_passthrough(' crates/roko-acp/src/runner.rs; }"
+++

## Problem

dc99a9e81 made every gate command start from an empty environment plus an allowlist. `[gates] env_passthrough`
is the only way to hand a gate an extra variable (`DATABASE_URL`, `AWS_*`), and it reaches the child only through
`GatePayload::env_passthrough`. Only the Graph verify path fills that field. Every other payload builder leaves
it empty, so there the setting is silently ignored, although the config docs (`config/gates.rs:165-180`,
`docs/v2/19-CONFIG.md`) say it covers "task verify steps, build and test gates".

## Why it matters

A project whose tests need a variable passes on `roko plan run` and fails on these paths, with nothing saying
why. The paths are legacy or opt-in today (see Current state), so this is config fidelity and dead-code hygiene,
not a secret leak: dropping variables is the safe direction. Follow-up of bug-7d7200.

## Where

- `crates/roko-cli/src/runner/gate_dispatch.rs::gate_signal` (:1970-2019): builds the payload for
  `run_gate_once` (:891, call at :1034), `spawn_plan_verify` (:1683) and `run_focused_baseline_verify` (:2165).
  No `.with_env_passthrough(...)`. `run_gate_once` holds `gates_config` and already passes
  `gates_config.env_passthrough` to auto-fix (`AutoFixBounds`, :1342), just not to the payload.
- `crates/roko-acp/src/runner.rs::build_gate_signal` (:1697): `GatePayload::in_dir(workdir)` for the ACP
  pipeline's compile/test/clippy gates (`run_gates`, :1711). `PipelineConfig` (:39) carries no gates config.
- `crates/roko-gate/src/gate_service.rs:238` (`GateService::run_gates`) and
  `crates/roko-gate/src/production_service.rs` (:140, :409, :505): same empty payload.
- Reference (works): `graph_task_dispatch.rs::settle_task_verification` sets
  `.with_env_passthrough(self.config.gates.env_passthrough...)`.

## Current state

Checked at `33e107da1`:
- `run_gate_once`, `spawn_gate` and `spawn_plan_verify` have no production caller since Runner-v2 was deleted;
  only tests in `gate_dispatch.rs` and `tests/gate_parity.rs` reach them.
- The ACP gate pipeline (`run_workflow_pipeline`) runs only with `ROKO_ACP_LEGACY` set
  (`bridge_events/mod.rs:696`, `bridge_events/slash_commands.rs:430`, :481). The default ACP path runs no gates.
- `GateService` is built by roko-serve (`service_factory.rs:359`, :539), but nothing reads its `gate_runner`
  field. `ProductionGateService` is reached only through `default_gate_adapter`, which only a test calls.
- `generated.rs:345` (tautology probe) and `benchmark_gate.rs:432` call `inherit_gate_env(&mut cmd, &[])`
  because they receive a path, not a payload.
- Also on a dead path: `runner/merge.rs::CargoCheckRegressionGate` (:476) runs `cargo check --workspace` with no gate
  environment at all; `PlanMerger` has no production caller.

## Plan

For each builder, choose between threading the setting and deleting the path:
1. `gate_signal`: take `&[String]` passthrough (from `gates_config.env_passthrough` in `run_gate_once`; from the
   config in `spawn_plan_verify`) and call `.with_env_passthrough(...)`. Or delete `run_gate_once`, `spawn_gate`
   and `spawn_plan_verify` with their tests, if nothing is meant to revive them.
2. ACP: add the passthrough to `PipelineConfig` (both construction sites have `roko_config`) and pass it to
   `build_gate_signal`. Or delete the `ROKO_ACP_LEGACY` pipeline.
3. `GateService` / `ProductionGateService`: take the passthrough from `GateConfig` / the request, or drop them.
4. Put `inherit_gate_env` on `CargoCheckRegressionGate`, or delete it with `PlanMerger`.

## Done when

- Every live gate payload builder applies `[gates] env_passthrough`, and each dead one is deleted.
- The `[[verify]]` passes: each of the two named builders is gone or calls `.with_env_passthrough(`.

## Notes

- Do not touch the Graph verify path; it is correct.
- Deleting the legacy runner gate path also closes this item. Record which way it went in the closing evidence.
