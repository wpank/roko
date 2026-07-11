# Spawn busy loop and log storm

- Severity: high
- Area: scheduler backpressure

This run wrote 1,396 `spawning agent` INFO messages. T15 was logged roughly every 100 ms from 15:34:28 through 15:36:46 even though no T15 process was dispatched until 15:41:48.

The log occurs before task semaphore acquisition (`event_loop.rs:4471` versus permit logic around 4517). Permit failure returns Noop, allowing the action to be selected and logged again on the next tick.

Acquire/reserve capacity before announcing spawn, mark queued state, and wake on permit availability rather than polling at 10 Hz. Log only actual lifecycle transitions.

