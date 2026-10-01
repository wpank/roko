+++
id = "bug-8b1bf8"
kind = "bug"
title = "POST /api/prds/{slug}/plan still sends a hand-built prompt through run_once"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-serve/routes/prds"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-serve/src/routes/prds.rs::queue_plan_generation_op"]
links = { depends_on = [], blocks = [], related = ["bug-d7c5dd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn test_prd_plan_uses_generate_pipeline' crates/roko-serve/src/routes/prds.rs && ! sed -n '/async fn queue_plan_generation_op/,/^}/p' crates/roko-serve/src/routes/prds.rs | grep -q 'run_once' && cargo test -p roko-serve -- prds::tests::test_prd_plan_uses_generate_pipeline"
+++

`queue_plan_generation_op` in `routes/prds.rs` constructs a raw prompt string and dispatches it through `runtime.run_once()` instead of calling `runtime.generate_plan_from_prd()`. This is the same defect that `POST /api/plans/generate` had before plan 04 T08–T10 fixed it: `run_once` does not run the PRD pipeline (fenced-TOML extraction, repair, model escalation, `validate_plan_context`), so the produced plan may fail to load or regenerate every old-format plan in `plans/` as a side effect.

Fix: replace the `run_once` call in `queue_plan_generation_op` with `runtime.generate_plan_from_prd(slug, workdir)`, matching the pattern established in `routes/plans.rs` by plan 04.

## Notes

- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  `queue_plan_generation_op` (used by `POST /api/prds/{slug}/plan`, auto-plan on promote, and the PRD-publish
  subscriber) now calls `runtime.generate_plan_from_prd(workdir, slug, prd_path)`; the hand-built
  `build_plan_generation_prompt` is gone. `plan_from_prd` now passes the PRD's real directory (an idea used to map
  to `drafts/`). The test runtime records `generate_plan_from_prd` calls, and `test_prd_plan_uses_generate_pipeline`
  checks the route makes exactly that one call.
