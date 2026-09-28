+++
id = "q-a2e94a"
kind = "question"
title = "DA-08: FAST default-lane, auto-merge, zero-test lane, model-policy and disk-refusal decisions pending"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["dev-workflow/FAST"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/dev-audit/08-decisions-needed.md#1. Default lane"
discovered_from = "audit:tmp/dev-audit/08-decisions-needed.md#1. Default lane"
anchors = ["dev.sh fast", "docs/v2/29-FAST-DEVELOPMENT.md", "DiskBudgetTracker"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Open policy decisions: make FAST the default (all plan runs vs a flag), allow T0/T1 auto-merge, add a zero-test prototype lane, benchmark optimization target (wall time/cost/regressions), mandatory async RELEASE merge policy, refusing cold runs under disk/swap pressure. Promotion criteria unmet.

Imported without verification from:
- `tmp/dev-audit/08-decisions-needed.md#1. Default lane`
- `tmp/dev-audit/08-decisions-needed.md#5. Machine/cache policy`
- `tmp/dev-audit/07-rollout.md#Promotion criteria`
- `tmp/dev-audit/11-implementation-status.md#Explicit residuals`

How to verify: Owner decision; check first whether FAST still exists on the Graph engine.
