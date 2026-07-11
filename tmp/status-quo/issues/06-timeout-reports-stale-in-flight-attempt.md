# Timeout reports a stale in-flight attempt

- Severity: medium
- Status: observed in the preceding run
- Area: timeout / attempt lifecycle

## Observation

E01-T06 completed successfully at `10:46:55`, was committed, and advanced to E01-T07. When the plan timed out at `10:56:38`, diagnostics still listed both:

```text
E01-execution-engine:E01-T06:1:Retrying
E01-execution-engine:E01-T07:2:AgentRunning
```

Timeout collection occurs in `crates/roko-cli/src/runner/event_loop.rs:6017-6059`.

## Impact

Snapshots and diagnostics disagree with terminal task state. Resume logic may requeue work that already passed, and operators cannot trust the in-flight list when diagnosing timeouts.

## Expected

Completing a retry should terminalize or remove all superseded attempts for that task before advancing. Timeout snapshots should only contain genuinely active attempts.

## Crash evidence

The final timeout repeated this defect: it reported E01-T07 attempt 1 and E01-T09 attempt 1 as `Retrying`. T09 attempt 3 had passed 25 minutes earlier and T07 attempt 3 had exhausted 24 minutes earlier. The stale records appear to have prevented quiescence detection and contributed directly to the 22-minute dead wait.
