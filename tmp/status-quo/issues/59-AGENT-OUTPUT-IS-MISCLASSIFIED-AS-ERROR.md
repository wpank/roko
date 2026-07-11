# Agent output is misclassified as error

- Severity: medium

`events.jsonl` contains normal source-code lines from T07 as dozens of `agent.error` events. This inflates error counts, obscures genuine provider/process failures, and makes diagnosis feeds unreliable.

Preserve backend channel/type information. Stderr text, command output, assistant deltas, diagnostics, and fatal agent errors need distinct typed events and severity rules.

