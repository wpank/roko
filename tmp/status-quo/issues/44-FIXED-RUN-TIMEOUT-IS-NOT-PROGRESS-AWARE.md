# Fixed run timeout is not progress-aware

- Severity: high
- Area: timeout policy

The deadline is created once at `event_loop.rs:875-879`. It neither resets after useful progress nor detects inactivity early. This run performed useful work for 38 minutes, then deadlocked for 22 minutes, and both periods counted identically toward a fatal one-hour cap.

Timeout handling (`event_loop.rs:6017-6056`) shuts down and returns an error, causing the TUI to disappear instead of entering a resumable failed/paused view.

Use separate hard-run, task, gate, agent-silence, and scheduler-no-progress timeouts. Preserve the terminal dashboard until acknowledged.

