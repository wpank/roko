# Plan state start timestamps are zero

- Severity: medium
- Run: `run-1783779617962`

The snapshot created after dispatch persists all six executor plan states with `started_at_ms=0`, including active SH01 and five blocked plans. This prevents meaningful plan elapsed time, timeout attribution, ordering, and recovery diagnostics.

Set start time only on the actual queued-to-active transition and preserve it across snapshots/resume. Blocked plans need separate queued/blocked timestamps.

