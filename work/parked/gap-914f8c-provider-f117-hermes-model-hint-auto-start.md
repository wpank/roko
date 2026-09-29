+++
id = "gap-914f8c"
kind = "gap"
title = "[provider F117] Hermes model_hint, auto_start_gateway, HermesFlavor::Z are dead code"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F117"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F117"
anchors = ["crates/roko-agent/src/provider/hermes.rs", "model_hint", "auto_start_gateway", "HermesFlavor::Z"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The `model_hint` field on `HermesAgent` is set but never read. `auto_start_gateway` is parsed from config but never acted upon. `HermesFlavor::Z` dispatch arm is never selected by any configuration path.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F117`
- `tmp/archive/provider-audit/01-provider-adapters.md`

How to verify: Roadmap P4-7 (Hermes auto_start_gateway / CrashRecoveryConfig) deferred. Confirm in crates/roko-agent/src/provider/hermes.rs whether still true: Hermes `model_hint`, `auto_start_gateway`, `HermesFlavor::Z` are dead code
