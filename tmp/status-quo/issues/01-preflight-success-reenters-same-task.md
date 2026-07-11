# Preflight success re-enters the same task

- Severity: high
- Status: reproducible in the live run
- Area: runner-v2 state machine / preflight

## Observation

At `2026-07-11T13:06:51`, E01-T01's preflight passed and the runner logged that it would skip the agent. It then failed `Enriching -> Gating` with `ImplementationDone`, immediately spawned E01-T01 again, and ran the same four verification steps a second time. Both passes were recorded before the runner advanced to T07.

Relevant implementation is in `crates/roko-cli/src/runner/event_loop.rs:4628-4696`. The preflight path unconditionally applies `ImplementationDone`, even though the runtime phase in this case is `Enriching`.

## Impact

- Expensive compile/test gates execute more than once.
- Terminal events and metrics are duplicated.
- The TUI can show the task as done while the runner is visibly still working.
- On a non-idempotent gate this could produce different results on the second pass.

## Expected

A successful preflight should use a valid phase-specific transition and enqueue exactly one gate-success/terminal sequence.

## Crash-run evidence

The current run emitted two E01-T01 starts and two completions, both labeled attempt 1, despite T01 already being seeded complete. Its first gate took 88,994 ms and the duplicate took another 5,378 ms. This corrupted attempt accounting before later concurrency failures.
