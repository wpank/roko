+++
id = "bug-d7c5dd"
kind = "bug"
title = "WF-11: Plan generation lifecycle not observable (no plan_generating/generated/updated events; silent timeout)"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/routes/plans"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues"
anchors = ["crates/roko-serve/src/routes/plans.rs", "apps/portal/src/app/work/editor/page.tsx"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "plan:portal-programme/04-backend-plan-authoring#T11"
run_id = "graph-04-backend-plan-authoring-b5c81fc5-124f-4278-88b1-52edf652788f"
evidence = "AUTHORING-CHECK: PASS (33/33). Checks confirmed: 'generate rejects a body with neither prompt nor slug' (PASS), 'generate rejects a body with both prompt and slug' (PASS), 'generate from a sentence returns 202 with the new plan's slug' (PASS), 'the generation operation finishes within 120s' (PASS), 'the operation reports the plan it produced' (PASS), 'generation completion is announced on the event stream' (PASS). POST /api/plans/generate now accepts {prompt} or {slug}, returns 202 {id, plan_id}, finalizes its operation handle, and emits plan_generate.started/completed/failed events. GET /api/operations/{id} returns JSON {id, kind, status, result?, error?} instead of a Rust Debug string."
+++

POST /api/plans/generate returns plan_id before tasks exist; only OperationCompleted is emitted, so the portal polls ~20s and times out silently when no provider is available (no user-facing error).

**Fixed by plan 04 T08–T11 (2026-09-29):**
- `POST /api/plans/generate` now accepts `{prompt}` or `{slug}` (exactly one), returns 202 `{id, plan_id}`.
- The generation handle is finalized: `GET /api/operations/{id}` returns JSON `{id, kind, status: "running"|"completed"|"failed", result?: {slug, task_count}, error?}`.
- The event stream emits `plan_generate.started`, `plan_generate.completed`, and `plan_generate.failed` with `plan_id`.
- `regenerate_old_format_plans` is skipped on the server path (T10); `validate_plan_context` is run after generation (T12).
- All 33 authoring-check assertions passed, including the generation assertions.
