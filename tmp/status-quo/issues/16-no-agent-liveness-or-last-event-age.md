# No agent liveness or last-event age

- Severity: medium
- Status: feature gap exposed by live run
- Area: operability

## Observation

Roko displays `active` but provides no turn elapsed time, time to first output, last backend event, session-file freshness, heartbeat, or stream health. Operators cannot distinguish an agent that is working silently from one whose output bridge is disconnected or a truly stalled process.

Mori tracks time to first output and turn timestamps (`apps/mori/src/app/parallel.rs:11884-11890`), emits liveness heartbeats after 30 seconds, warns after 90 seconds without fresh activity, and requeues zero-output stalled turns after five minutes (`parallel.rs:15877-16018`).

## Expected

Each active row should expose elapsed turn time and last-event age with explicit states such as starting, active, silent, stream disconnected, and stalled. Health checks should use provider/session activity rather than PID existence alone.

## Crash-run evidence

After T15 passed, all normal activity stopped for 21m56s. The TUI emitted no blocked-state heartbeat or last-progress warning before disappearing at timeout. A scheduler liveness indicator would have exposed the deadlock immediately.
