+++
id = "bug-5e7de4"
kind = "bug"
title = "plans/INDEX.md is inaccurate (omits portal-programme, zero task counts, counts junk as executable)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan-index"]
created = 2026-09-25
updated = 2026-09-28
source = "plans/INDEX.md:12"
discovered_from = "audit:plans/INDEX.md:12"
anchors = ["roko plan index", "plans/INDEX.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
INDEX lists 15 executable plans/17 tasks incl. junk; shows 0 tasks for plans using [[tasks]] (scratch-test, test-fibonacci, audit-test, modal-test, test-check, test-greeting-module); omits nested portal-programme/01-08 and plan.md-only dirs. CLAUDE.md claims 124/124 across 30 plans.

Imported without verification from:
- `plans/INDEX.md:12`
- `plans/INDEX.md:28`

How to verify: Run `roko plan index --check` (or equivalent); inspect index generator's tasks.toml parsing and recursion.
