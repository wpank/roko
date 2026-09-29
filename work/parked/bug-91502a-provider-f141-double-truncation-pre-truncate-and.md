+++
id = "bug-91502a"
kind = "bug"
title = "[provider F141] Double truncation — pre-truncate and hard_cap with identical limits"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/templates"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F141"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F141"
anchors = ["crates/roko-compose/src/templates/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Templates pre-truncate content to `budget.X` characters before creating `PromptSection`, then set `hard_cap(budget.X)` on the same section. The `enforce_hard_cap()` method never fires because content was already truncated to the same limit.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F141`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/templates/ whether still true: Double truncation — pre-truncate and hard_cap with identical limits
