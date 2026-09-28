+++
id = "q-778b4f"
kind = "question"
title = "DOCS-07 TD-20 / 03: Budget enforcement effectively disabled by zero defaults"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-20: Budget Enforcement Disabled by Default"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-20: Budget Enforcement Disabled by Default"
anchors = ["budget defaults"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Default config has zero budget limits, disabling enforcement; decision in 03 is FIX with sensible non-zero defaults (or document the zero default). 09-03 says 0.0 intentionally means unlimited.

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-20: Budget Enforcement Disabled by Default`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Config Issues`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`

How to verify: Check default budget values in roko-core config defaults.
