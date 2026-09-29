+++
id = "gap-b3e513"
kind = "gap"
title = "revision retry count is hardcoded to one"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/04b-backend-plan-revision#T06"
discovered_from = "plan:portal-programme/04b-backend-plan-revision#T06"
anchors = ["crates/roko-serve/src/routes/plans.rs::revise_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'retry\\|max_retries\\|revision_retries' crates/roko-serve/src/routes/plans.rs | head -20"
+++

`POST /api/plans/{id}/revise` retries a rejected revision exactly once, with its
validation diagnostics appended to the feedback. The retry count is hardcoded to one
in the server implementation.

For most workloads one retry is sufficient. However, complex plans with many tasks and
cross-task dependencies may benefit from a second retry, and operators running large
plans may want to configure more attempts without changing code.

This is a `p3` polish gap: the current behaviour (one retry) is intentional per the
§2.8 spec and covers the practical case. A configurable retry count is a future
improvement.

**A fix must:** read an optional `[serve] revision_max_retries` config value (defaulting
to 1) and pass it to the revision handler, so the retry cap is not baked into the binary.
