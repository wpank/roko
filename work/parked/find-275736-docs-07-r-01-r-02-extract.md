+++
id = "find-275736"
kind = "finding"
title = "DOCS-07 R-01/R-02: Extract TUI data pipeline and unify EventBus with Lens/StateHub"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#R-01: Extract TUI Data Pipeline"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#R-01: Extract TUI Data Pipeline"
anchors = ["EventBus", "StateHub"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
TUI mixes data fetching with rendering (testability/caching), and EventBus and Lens/StateHub coexist as separate event systems; both proposed refactors (L/XL).

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#R-01: Extract TUI Data Pipeline`
- `tmp/docs-audit/07-TECH-DEBT.md#R-02: Unify Event Systems`
- `tmp/dogfood/2026-09-19-session.md#2026-09-20 Continuation`

How to verify: Architectural; check for any in-flight refactor.
