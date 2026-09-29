+++
id = "gap-6e451b"
kind = "gap"
title = "[provider F145] No distinction between 401 (expired key) and 403 (forbidden)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F145"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F145"
anchors = ["crates/roko-agent/src/provider/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
All providers map both 401 and 403 to `ProviderError::AuthFailure`. An expired key (refreshable) and an invalid key (requires user action) produce the same error and the same behavior.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F145`
- `tmp/archive/provider-audit/16-error-retry.md`

How to verify: P3-1 pattern maps 401|403 => AuthFailure without distinguishing expired key. Confirm in crates/roko-agent/src/provider/ whether still true: No distinction between 401 (expired key) and 403 (forbidden)
