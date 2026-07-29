# Issue 42 disposition — lost gate completion leaves DAG task running forever

- Issue: tmp/status-quo/issues/42-LOST-GATE-COMPLETION-LEAVES-DAG-RUNNING.md
- Disposition: **RESOLVED** by SH01-T02 + SH01-T06 ownership chain
- Merged evidence: SH01-T02 (preflight/gate transitions exactly-once), SH01-T06B2C1–T06B2C3C3 (ownership chain)

## Resolution

SH01-T02 ensures every gate completion terminalizes its exact attempt.
Transition errors are terminal failures, not warning-only. The ownership
chain (SH01-T06B2C1 through T06B2C3C3) enforces that gate/merge/plan
completions are claimed before effects execute, preventing detached work.

The `terminalize_attempt` function atomically transitions the attempt to a
terminal state and records the outcome in both the lifecycle projection and
the run ledger. Late/duplicate completions are rejected.

## Verification

```
cargo test -p roko-cli runner::event_loop   # 76 pass, gate terminalization covered
cargo test -p roko-cli runner::gate_dispatch # gate failure propagation
cargo test -p roko-cli --test runner_crash_recovery # 10 pass, concurrent completions
```
