+++
id = "gap-58b0dc"
kind = "gap"
title = "DF-0829: Generated plans lack read_files context, acceptance contracts and per-task estimates; no quality score"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan_generate"]
created = 2026-08-29
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-29/DOGFOOD-DEBRIEF.md#What could be better"
discovered_from = "audit:tmp/archive/dogfood-2026-08-29/DOGFOOD-DEBRIEF.md#What could be better"
anchors = ["plan_generate.rs", "[task.context]"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Audit of generated plans: no [task.context] read_files, no acceptance_contract, only plan-level estimates, backlog metadata dropped; proposes automatic plan quality scoring (context, verify, valid paths, slug consistency, validation).

Imported without verification from:
- `tmp/archive/dogfood-2026-08-29/DOGFOOD-DEBRIEF.md#What could be better`
- `tmp/archive/dogfood-2026-08-29/DOGFOOD-DEBRIEF.md#4. Plan quality scoring`

How to verify: Generate a plan and inspect for read_files/acceptance_contract.
