# Active agent can show all phases complete

- Severity: high
- Status: reproduced
- Area: TUI state consistency

## Observation

The screenshot simultaneously shows an active implementer and `all phases complete`. Phase steps are inferred solely from task status (`crates/roko-cli/src/tui/state.rs:2811-2857`); the phase widget declares completion when all inferred steps are done (`phase_compact.rs:149-158`). Active-agent state is not reconciled with that inference.

The identity mismatch in issue 08 makes this worse: a newly active agent may not attach to its new task, leaving the prior completed task to drive the phase panel.

## Expected

The snapshot reducer should enforce invariants across active agent, active task, and phase. An active implementation or gate must override stale terminal phase state and display the exact current phase.

