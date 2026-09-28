+++
id = "bug-9f340c"
kind = "bug"
title = "DF-0926 R-7: Plan API residuals after directory-plan discovery"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies"
anchors = ["crates/roko-serve/src/routes/plans.rs find_plan", "create_plan", "crates/roko-cli/src/serve_runtime.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
find_plan still backs nine handlers that 404 for directory plans; plans_dir change redirected them away from legacy .roko/plans; create_plan writes UUID JSON into source-controlled plans/; completed predicate differs from CLI; core logic untested (roko-serve cannot depend on roko-cli).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-7. The plan-01 code is good where it counts, weak where it verifies`

How to verify: Call each plans route for a directory plan id.
