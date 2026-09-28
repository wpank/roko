+++
id = "find-033e09"
kind = "finding"
title = "[provider F111] ProviderSemaphores creates new Semaphore for unknown provider IDs"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F111"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F111"
anchors = ["crates/roko-agent/src/provider/mod.rs", "ProviderSemaphores", "Semaphore"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
When a provider ID is first seen, `ProviderSemaphores` creates a new `Semaphore` without checking configuration. Unknown providers receive a default concurrency limit.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F111`
- `tmp/archive/provider-audit/01-provider-adapters.md`

How to verify: Confirm in crates/roko-agent/src/provider/mod.rs whether still true: `ProviderSemaphores` creates new `Semaphore` for unknown provider IDs
