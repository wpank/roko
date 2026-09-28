+++
id = "q-7afd9c"
kind = "question"
title = "VCG auction vs greedy knapsack in context composition: wire or delete?"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
anchors = ["vcg_allocate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
MASTER-TASKS: `vcg_allocate` built but greedy path dominates. The 2026-04 MASTER-IMPLEMENTATION-PLAN 6.6 instead proposed deleting VCG (keep greedy). Later E44 claims conflict VCG wired. Needs a decision/verification.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring`
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#Phase 6: Retirement (6.6 delete VCG auction)`

How to verify: grep `vcg_allocate` callers; check which composition strategy is the production default.
