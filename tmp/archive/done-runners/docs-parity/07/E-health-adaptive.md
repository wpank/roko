# E — Health, Timeouts, And Process Support

Refresh of parity for `docs/07-conductor/06`, `10`, `12`, and `13`.

Generated: 2026-04-18

---

## Bottom Line

This area needed narrowing more than deletion.

- health monitoring exists as an implemented support module,
- state-machine and timeout helpers exist,
- process supervision exists and is already part of the runtime story,
- but the docs should stop using the theory-heavy pressure material as if it
  were part of the current implementation contract.

---

## What Exists Today

### Health monitor

`HealthMonitor` is implemented in `crates/roko-conductor/src/health.rs`.

Key anchors:

- `SystemSnapshot`: `crates/roko-conductor/src/health.rs:87-114`
- `HealthMonitor::new()`: `crates/roko-conductor/src/health.rs:148-172`
- `check_all()` / `overall_status()`: `crates/roko-conductor/src/health.rs:180-194`

The health surface should be described as **implemented**.

### State-machine helpers

`phase_timeout()` and `PhaseTransition` are implemented in
`crates/roko-conductor/src/state_machine.rs`.

That means the timeout/state-machine docs should be framed as nearby shipped
support code, not as pure design.

### Process supervision

`ProcessSupervisor` exists in `crates/roko-runtime/src/process.rs`, and
`PlanRunner` holds one at `crates/roko-cli/src/orchestrate.rs:2604`.

This is enough to say the process-supervision chapter is grounded in real
runtime code rather than fantasy.

---

## Caveats To Keep Explicit

### `golem_status` is stale naming

The second built-in health check is still registered as `golem_status` at
`crates/roko-conductor/src/health.rs:158-160`, even though the snapshot field
it inspects is `chain_connected`.

Docs should call this out as naming drift, not as a missing subsystem.

### Ownership is split

There is still a real ownership seam between:

- `ProcessSupervisor` in `roko-runtime`
- agent-process tracking in `roko-agent/src/process/registry.rs`

That is the useful takeaway from Doc 13 parity now. The chapter should not be
reduced to "unwired," but it also should not imply a single perfectly unified
owner when the code still has two.

### Keep timeout adoption wording careful

`PhaseTransition` and timeout helpers exist, but parity docs should avoid
claiming that every downstream adaptive-timeout path is fully standardized.
Describe the helpers as implemented support surfaces and leave broader
adoption as follow-up work.

---

## What Moves Out Of This File

Do not keep Yerkes-Dodson pressure tuning here as an implementation checklist.

Move that material into the informational/frontier posture used by
[F-theory-learning.md](F-theory-learning.md):

- pressure dial
- flow detection
- model pressure profiles
- cooperation-pressure calibration

Those are not the current health/process contract.
