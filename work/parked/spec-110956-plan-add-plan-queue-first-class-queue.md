+++
id = "spec-110956"
kind = "spec"
title = "Plan add-plan-queue: first-class [queue] config section + queue docs (3 ready tasks)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config+roko-cli/plan-queue"]
created = 2026-09-24
updated = 2026-09-28
source = "plans/add-plan-queue/plan.md"
discovered_from = "audit:plans/add-plan-queue/plan.md"
anchors = ["roko plan queue show", "roko_core::config QueueConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Ready plan: add QueueConfig to roko-core config schema (queue file path, max agents, execution mode), wire defaults into `roko plan queue show`, and write plan-queue feature documentation.

Imported without verification from:
- `plans/add-plan-queue/plan.md`
- `plans/add-plan-queue/tasks.toml`

How to verify: grep for QueueConfig in roko-core; check plan task statuses.
