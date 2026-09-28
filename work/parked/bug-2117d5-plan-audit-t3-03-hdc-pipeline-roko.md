+++
id = "bug-2117d5"
kind = "bug"
title = "[plan-audit T3-03] HDC pipeline roko-cli -> roko-compose -> roko-neuro"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/hdc"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["hdc feature in crates/*/Cargo.toml", "backlog #67"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Plan audit lists fixing the HDC pipeline as open; CLI audit says HDC feature propagation to compose/fs/serve was done (#67).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`

A source claims this was fixed; confirm against current code before closing.

How to verify: cargo tree -e features for hdc across roko-cli/compose/neuro.
