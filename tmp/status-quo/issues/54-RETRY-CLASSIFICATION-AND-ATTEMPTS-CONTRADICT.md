# Retry classification and attempts contradict

- Severity: high
- Area: retry policy

T13's digest says `failure_kind=permanent` while `retryable=true`, `recommended_action=retry`, and later reports retries exhausted on attempt 1. Other tasks jump from attempt 1 directly to attempt 3. Stale attempt 1 records remain `Retrying` after attempt 3 is terminal.

Classification, retry decision, attempt allocation, and terminalization are separate inconsistent sources of truth. Define one retry decision object, allocate a new attempt exactly once, and supersede the prior attempt atomically.

