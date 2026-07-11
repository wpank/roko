# Preflight work is invisible and counted as dispatch

- Severity: high
- Status: reproduced
- Area: responsiveness / timing

## Observation

E01-T07 ran four preflight steps, including a 78-second test, before starting its agent. The runner reported `dispatch_ms=388973`, over six minutes. During this period the TUI did not show the current command, gate step, elapsed time, or output and instead displayed completed phases.

At 15:15, T11 was actively running `cargo test -p roko-cli budget`, but events, ledger, and snapshot timestamps remained at the preceding dispatch event until a step finished.

## Expected

Preflight should be its own visible phase with `step n/N`, command summary, elapsed time, last-output age, and heartbeat events. Dispatch timing should measure model selection/process spawn rather than all preflight work.

## Crash-run evidence

Preflight inflated dispatch time to 388,973 ms for T07, 106,902 ms for T08, 285,583 ms for T11, and 461,155 ms for T12. These delays consumed a large fraction of the fixed one-hour run deadline before model work began.
