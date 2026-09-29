+++
id = "gap-03a4b5"
kind = "gap"
title = "[plan-audit T3-09] `roko inject` should deliver live directives (mori ingest)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/inject"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["crates/roko-cli/src/commands/util.rs", "backlog #325 #361 #202"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Plan audit says roko inject is a stub printing status: queued; CLI audit says #325 fails closed and #361/#202 add acknowledged transport and routing.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Run roko inject against a running plan; confirm ack and effect.
