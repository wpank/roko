# TUI log omits runtime and exit state

- Severity: medium

`.roko/tui.log` records only `TUI file logging enabled` at startup. It contains no refresh errors, event lag, active-agent changes, blocked scheduler state, timeout, terminal outcome, or shutdown reason.

The TUI should log state-source connectivity, snapshot/event sequence numbers, refresh latency, dropped events, terminal outcome, and exit reason.

