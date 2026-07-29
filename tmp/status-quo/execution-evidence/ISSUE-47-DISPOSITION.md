# Issue 47 disposition — event and attempt lifecycles are incomplete

- Issue: tmp/status-quo/issues/47-EVENT-AND-ATTEMPT-LIFECYCLES-ARE-INCOMPLETE.md
- Disposition: **RESOLVED** by SH01-T01, SH01-T03, SH01-T06 ownership chain
- Merged evidence: full SH01 plan (28/28 done)

## Resolution

SH01-T01 made the task-attempt lifecycle the canonical runtime state. Every
attempt has exactly one start and one terminal event. The lifecycle projection
enforces legal transitions and rejects illegal ones.

SH01-T03 added atomic idempotent task terminalization — a single coordinator
updates state atomically, preventing the duplicate starts (T01 issue) and
missing terminals (T08/T07/T09 issues).

The SH01-T06 ownership chain (T06B1 through T06B2C3C3) ensures:
- Process/gate cancellation is confirmable (T06B1)
- Attempt ownership is exact and linear (T06B2A)
- Every start has exactly one terminal via end-to-end ownership

Attempt IDs are now monotonic and unique. No run/plan ends without an
explicit terminal event for every started attempt.

## Verification

```
cargo test -p roko-cli runner::state        # lifecycle transition tests
cargo test -p roko-cli runner::event_loop   # 76 pass, all ownership tests
cargo test -p roko-runtime run_ledger       # ledger completeness
cargo test -p roko-cli --test runner_crash_recovery # 10 pass
```
