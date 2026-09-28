+++
id = "spec-dba39a"
kind = "spec"
title = "`nunchi` CLI wrapper and demo commands (agents list, audit, resume, replay)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (deadline: May 6)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (deadline: May 6)"
anchors = ["roko resume", "roko replay"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
May-6 demo checklist (P0-1..P0-5): `nunchi` wrapper binary, `nunchi agents list` with identity display, scripted `nunchi audit` (identity/predict/gates/knowledge), `nunchi resume` crash recovery, `nunchi replay` JSON audit trail. Demo date has passed; only the product-facing commands remain…

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (deadline: May 6)`

How to verify: Check for a `nunchi` wrapper/alias binary and whether `roko agent list`/`roko resume`/`roko replay` cover identity display + audit trail; likely obsolete demo scaffolding — confirm with owner.
