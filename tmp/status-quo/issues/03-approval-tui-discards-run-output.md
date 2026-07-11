# Approval TUI discards runner output

- Severity: medium
- Status: directly observed and code-confirmed
- Area: TUI telemetry / output sinks

## Observation

The live TUI showed `no parallel agents`, `no agent route metrics`, `no agent output yet`, and `no conductor diagnoses yet` while the process tree showed an active Roko child gate and the main log contained task/gate activity.

The live run logs `output_sink=NoopSink`. In `crates/roko-cli/src/commands/plan.rs:552-566`, approval mode selects `NoopSink`; `crates/roko-cli/src/runner/output_sink.rs:382-400` defines that sink as discarding all events.

## Impact

The primary monitoring surface falsely appears idle. Users cannot distinguish a hung runner from a long compile/test gate and cannot see agent output or gate details without separately tailing `.roko/roko.log`.

## Expected

Approval/TUI mode should feed structured runner events into the dashboard even if stderr rendering is disabled. A silent sink should be reserved for quiet, JSON, tests, or embedded callers.

