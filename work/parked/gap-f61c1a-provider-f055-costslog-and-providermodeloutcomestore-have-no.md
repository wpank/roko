+++
id = "gap-f61c1a"
kind = "gap"
title = "[provider F055] CostsLog and ProviderModelOutcomeStore have no self-rotation"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/costs_db"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F055"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F055"
anchors = ["crates/roko-learn/src/costs_db.rs", "crates/roko-learn/src/provider_outcome.rs", "CostsLog", "ProviderModelOutcomeStore"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Both JSONL files grow unboundedly. Neither file implements size-based or time-based rotation. In long-running deployments, these files can grow without limit.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F055`
- `tmp/archive/provider-audit/04-health-efficiency.md`

Some cited files are gone: `crates/roko-learn/src/provider_outcome.rs`.

How to verify: Confirm in crates/roko-learn/src/costs_db.rs, crates/roko-learn/src/provider_outcome.rs whether still true: `CostsLog` and `ProviderModelOutcomeStore` have no self-rotation
