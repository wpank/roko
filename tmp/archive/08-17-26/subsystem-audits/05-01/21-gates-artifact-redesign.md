# 21 - Gates and Artifact Validation Redesign

Scope: `crates/roko-core/src/foundation.rs`, `crates/roko-gate/src/gate_service.rs`, `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-cli/src/runner/gate_dispatch.rs`, `crates/roko-cli/src/gate_runner.rs`, `crates/roko-cli/src/plan_validate.rs`, `crates/roko-cli/src/prd.rs`

The gate layer improved by adding skipped verdicts, but the system still treats gate names, rung mappings, runner verdicts, plan validation, and artifact validation as loosely related strings. This is where a lot of "done" claims can compile while not proving the intended artifact.

## Findings

### HIGH: skipped/not-wired gates are status strings, not first-class outcomes

`GateVerdict` has `passed`, `skipped`, and `skip_reason` at `foundation.rs:282-296`, and `GateReport::all_passed` rejects skipped gates at `foundation.rs:304-309`. That is a good start.

But `GateService` still represents judge, unknown, missing custom config, and adaptive skip as skipped verdicts (`gate_service.rs:249-255`, `258-331`, `336-345`). `EffectDriver::record_gate_verdict` ignores `skipped` and emits `GateFailed` for every non-passed verdict at `effect_driver.rs:471-486`.

Expected design: replace boolean pairs with `GateStatus::{Passed, Failed, Skipped, NotWired, InvalidConfig}`. Runtime events and reports should preserve the status instead of flattening skipped/not-wired into failed.

### HIGH: required gates can be silently non-executable

`gate_service.rs:75-80` says shell/custom gates must have explicit commands, and judge is represented by `StubJudgeGate`. `run_gates` then records skipped verdicts when judge is not implemented or shell/custom commands are missing. That avoids false pass, but it still lets a required gate be configured without being executable until runtime.

Expected design: gate configuration should be validated before execution. Missing implementation for a required gate should be a config error, not a runtime skipped verdict. Optional adaptive skips should be separate from "not wired."

### HIGH: gate/rung ownership is duplicated across crates

`gate_service.rs:50-61` maps gate names to rung numbers. `effect_driver.rs:638-655` duplicates a similar map and includes a TODO to expose it from `roko-gate`. `gate_runner.rs:51-69` has another mapping from primary gate phases to runner rungs.

Duplicated mappings are why fixes drift: adding or renaming a gate can update one path while runner classification, workflow reporting, or validation still uses the old concept.

Expected design: create a single gate registry that owns gate id, aliases, rung, required inputs, executor, result schema, and whether it is optional/adaptive. All runner/runtime/validation code should query that registry.

### MEDIUM: runner failure classification uses JSON serialization as a contract

`runner/gate_dispatch.rs:287-323` classifies a gate failure, serializes the classification to JSON, then checks whether the JSON string contains `"external_environment"`. This couples behavior to the debug shape of a serialized object rather than a typed field.

Expected design: the classifier should return typed fields such as `failure_domain`, `recommended_action`, and `retryability`. Runner code should match on those fields.

### MEDIUM: global gate concurrency is hardcoded

`runner/gate_dispatch.rs:20-25` creates a process-wide semaphore with one permit for all gates. That is safe but it is not a designed resource policy. It serializes unrelated gates and hides the fact that some gates are CPU-bound, some are IO-bound, and some should never run concurrently against the same worktree.

Expected design: concurrency should be configured by gate kind/resource group/worktree, with defaults in the gate registry.

### MEDIUM: artifact validation is not integrated as a blocking contract

`plan_validate.rs` has detailed checks for architecture queues, dependencies, contexts, files, verify steps, acceptance contracts, and parity ledgers. `prd.rs` records `GenerationOutcome { process_success: true, artifact_valid, validation_report }` after dry-run validation. That means "the generator process succeeded" and "the artifact is valid" are adjacent fields, not a single enforced outcome.

Expected design: generated artifacts should return `ArtifactOutcome::{Valid, Invalid(report), NotProduced}`. Workflows that require valid artifacts should not be able to report success while `artifact_valid` is false.

## Redesign Direction

1. Replace gate booleans with a typed `GateStatus` enum.
2. Add a single gate registry shared by `roko-gate`, runtime, runner, and validation.
3. Validate required gate wiring before execution.
4. Make runner failure classification typed end to end.
5. Treat artifact validity as a workflow outcome, not a side field beside process success.
6. Add fitness tests for "unknown gate", "required judge not wired", "custom gate missing command", and "artifact invalid cannot be success."
