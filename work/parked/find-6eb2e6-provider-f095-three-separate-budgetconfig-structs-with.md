+++
id = "find-6eb2e6"
kind = "finding"
title = "[provider F095] Three separate BudgetConfig structs with different default values"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F095"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F095"
anchors = ["crates/roko-core/src/config/", "crates/roko-cli/src/", "crates/roko-agent/src/", "BudgetConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`roko-core`, `roko-cli`, and `roko-agent` each define a `BudgetConfig` struct with different field sets and defaults. Budget configuration is fragmented and callers must know which struct to use.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F095`

How to verify: Roadmap AR-6 (unify BudgetConfig) deferred. Count BudgetConfig definitions. Confirm in crates/roko-core/src/config/, crates/roko-cli/src/, crates/roko-agent/src/ whether still true: Three separate `BudgetConfig` structs with different default values
