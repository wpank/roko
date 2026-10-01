+++
id = "gap-b3e513"
kind = "gap"
title = "revision retry count is hardcoded to one"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "plan:portal-programme/04b-backend-plan-revision#T06"
discovered_from = "plan:portal-programme/04b-backend-plan-revision#T06"
anchors = ["crates/roko-cli/src/plan_authoring.rs::revise_plan_source", "crates/roko-serve/src/routes/plans.rs::revise_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'revision_max_retries' crates/roko-core/src/config/serve.rs && grep -q 'revision_max_retries' crates/roko-cli/src/plan_authoring.rs crates/roko-cli/src/serve_runtime.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:47Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:36Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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

Re-checked 2026-09-29: unchanged. The hardcoded single retry is in crates/roko-cli/src/plan_authoring.rs::revise_plan_source (:398-413), not in the serve route. The existing verify always passes (grep | head exits 0) and targets the wrong file.

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `[serve] revision_max_retries` (`ServeConfig`, u32, default 1) sets how many times `revise_plan_source` asks the planning agent again after a rejected revision. Each retry carries the last rejection's diagnostics; `0` makes one attempt only. The setting is read from the workspace's resolved config, which the function already loads, so `revise_plan_source` keeps its signature and serve's caller is unchanged. Test: `revision_retries_default_to_one_and_are_configurable` (roko-core `config/serve.rs`). Documented in `docs/v3/depth/21-config/01-schema-sections.md`.
