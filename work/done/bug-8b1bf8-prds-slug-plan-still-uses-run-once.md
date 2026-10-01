+++
id = "bug-8b1bf8"
kind = "bug"
title = "POST /api/prds/{slug}/plan still sends a hand-built prompt through run_once"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-serve/routes/prds"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-serve/src/routes/prds.rs::queue_plan_generation_op"]
links = { depends_on = [], blocks = [], related = ["bug-d7c5dd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn test_prd_plan_uses_generate_pipeline' crates/roko-serve/src/routes/prds.rs && ! sed -n '/async fn queue_plan_generation_op/,/^}/p' crates/roko-serve/src/routes/prds.rs | grep -q 'run_once' && cargo test -p roko-serve -- prds::tests::test_prd_plan_uses_generate_pipeline"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:22Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:27Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
