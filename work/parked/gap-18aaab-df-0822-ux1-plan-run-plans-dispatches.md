+++
id = "gap-18aaab"
kind = "gap"
title = "DF-0822 UX1: `plan run plans/` dispatches every plan (no --plan filter)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan"]
created = 2026-08-22
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#UX1: No `--plan` filter flag"
discovered_from = "audit:tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#UX1: No `--plan` filter flag"
anchors = ["commands/plan.rs", "max_concurrent_plans"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Pointing plan run at plans/ launched all 41 plans (~40 agents) and burned quota; there was no --plan <name> filter and scoping by path was not obvious from help.

Imported without verification from:
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#UX1: No `--plan` filter flag`

How to verify: Check `roko plan run --help` for a plan filter.
