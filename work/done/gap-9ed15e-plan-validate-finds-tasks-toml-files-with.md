+++
id = "gap-9ed15e"
kind = "gap"
title = "plan validate finds tasks.toml files with its own walker instead of discover_plans"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan_validate", "roko-cli/plan-discovery"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-cli/src/plan_validate.rs::collect_tasks_files", "crates/roko-cli/src/plan_validate.rs::collect_tasks_files_recursive", "crates/roko-cli/src/orchestrator/plan_discovery.rs::find_plan_dirs", "crates/roko-cli/src/orchestrator/plan_discovery.rs::discover_plans"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "fn collect_tasks_files_recursive" crates/roko-cli/src/plan_validate.rs'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:44Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:21Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`plan_validate` walks directories with a private recursive walker (plan_validate.rs:280-303). Plan runs, serve and the TUI resolve plans through `orchestrator::plan_discovery::discover_plans`. Any difference in their rules means `roko plan validate plans/` can check a different set of plans than `roko plan run plans/` executes.

Fix: validate exactly the set that `discover_plans` returns.

Rechecked 2026-09-29 at d9e79e9d8: still open. Concrete differences between the plan_validate walker (plan_validate.rs:285) and find_plan_dirs (orchestrator/plan_discovery.rs:218): the walker does not skip _meta/ or dot-directories, has no MAX_PLAN_DEPTH limit, recurses into plan directories, matches archive names case-sensitively and does not report duplicate plan ids.

## Notes

- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check.
  `collect_tasks_files` is now built on `find_plan_dirs`, and its own walker `collect_tasks_files_recursive` is
  gone. So `plan validate` and `plan validate --spec-quality` (gap-4b3bd5) skip `_meta/`, dot-directories and
  plans nested in a plan, stop at `MAX_PLAN_DEPTH`, and fail on duplicate plan ids, as `plan run` does. The test
  is `validate_reads_the_plans_that_discovery_finds`.
