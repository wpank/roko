+++
id = "gap-3bea93"
kind = "gap"
title = "revise agent receives no execution-failure context"
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
command = "grep -r 'last_error\\|failure_context\\|run_context' crates/roko-serve/src/routes/plans.rs"
+++

When `POST /api/plans/{id}/revise` is called after a plan run has failed, the planning
agent receives only the current `tasks.toml` and the user's feedback text. It has no
information about which task failed, what error it produced, or what gate output caused
the failure.

A smarter revision experience would optionally include the last run's failure summary —
the failed task id, the gate output or agent error — so the agent can produce a more
targeted fix without the user having to copy-paste the failure message into the feedback
field manually.

This is a `p3` polish gap: the feature works correctly without it, and the user can
include failure context in the feedback field manually.

**A fix must:** pass execution-failure context (e.g. the last checkpoint's error or
`last_error` from `PlanSummary`) into the revision prompt alongside the feedback, when
the plan has a recorded failure and the caller does not supply explicit failure detail.
