# Preflight has no durable heartbeat

- Severity: high
- Run: `run-1783779617962`

SH01-T01 entered preflight at 16:20:19. While live children ran `cargo clippy` and then `cargo test -p roko-cli`, `.roko/roko.log`, `events.jsonl`, the state snapshot, and the run ledger remained unchanged for more than three minutes.

Headless mode therefore still cannot distinguish active gate work from a deadlock without inspecting the OS process tree. Preflight needs step-start, command-output/heartbeat, elapsed, last-output, and step-complete events persisted at a bounded cadence.

