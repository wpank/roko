# Gate Pipeline Issues

Investigation of `crates/roko-gate/` and `crates/roko-cli/src/runner/gate_dispatch.rs`.

## Critical

### Rungs 3-6 always stub-pass
- `gate_dispatch.rs:104`: `let inputs = RungExecutionInputs::default()` — all fields are `None`.
- Rung 3 (Symbol): `rung_dispatch.rs:303` — stubs if `symbol_signal.is_none()`.
- Rung 4 (GeneratedTest): `rung_dispatch.rs:330` — stubs.
- Rung 5 (PropertyTest): `rung_dispatch.rs:365` — stubs.
- Rung 6 (Integration): `rung_dispatch.rs:396,413` — stubs.
- `enrich_rung_config` is never called from the runner path. All adaptive apparatus is dark.

### Two parallel threshold systems — one never persisted
- Runner uses `GateThresholds` (simplified EMA-only) saved in `StateSnapshot`.
- `roko-gate` has full `AdaptiveThresholds` with CUSUM/SPC/Hotelling — only in dead `orchestrate.rs`.
- Runner's thresholds are never written to standalone `gate-thresholds.json`.
- TUI and `roko learn gates` read `gate-thresholds.json` — they only see legacy data.

## High

### GateComposition::Parallel runs sequentially
- `gate_pipeline.rs:380-393`: `run_parallel` awaits each gate in a `for` loop. No `tokio::spawn` or `join_all`. Same for `Voting` mode (line 470).

### Gate tool-not-found silently passes
- `compile.rs:118-128`, `test_gate.rs:149-160`: If `cargo` not on PATH, returns `Verdict::pass("skipped:")`. No gate actually validates.

### Semaphore acquisition failure silently abandons gate
- `gate_dispatch.rs:43-45`: If semaphore is closed, task returns without sending `GateCompletion`. Task stays permanently in `GateActive` state until plan timeout.

## Medium

### `has_custom_rungs` branch drops execution config
- `gate_dispatch.rs:110-119`: When `has_custom_rungs()` is true, `from_config` is called without `inputs` or `config`. Timeout override is lost.

### Schema mismatch on threshold load silently loses history
- `event_loop.rs:480`: `load_gate_thresholds(&paths).unwrap_or_default()`. If deserialization fails (different schema), falls back to empty. All accumulated data lost.

### Binary 0/1 gate score on learning bus
- `event_loop.rs:1407-1414`: `score: if passed { 1.0 } else { 0.0 }`. No partial credit. `observe_residual()` never called from runner.

### Retry+Permanent misclassified as retryable
- `gate_dispatch.rs:432-448`: When oracle says `Retry` but text-classification returns `Permanent`, result is `Structural` (retryable). Permanent errors retried indefinitely.
