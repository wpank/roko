+++
id = "gap-c09fc7"
kind = "gap"
title = "Agents are not told which plans run beside them, so their ad-hoc builds can see other plans' half-finished edits"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/dispatch/prompt_builder", "roko-cli/graph_execution/plan_set"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_execution/plan_set.rs"]
links = { depends_on = [], blocks = [], related = ["gap-0001a1", "gap-c89b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'concurrent_plans' crates/roko-cli/src/dispatch/prompt_builder.rs && cargo test -p roko-cli --lib prompt_names_concurrent_plans"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:49Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:31Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

With `max_parallel_plans` above 1, independent plans run at the same time in the operator's single working tree. Verify gates are serialized by the compile lock, but an agent's own ad-hoc `cargo build` or `cargo test` can compile another plan's half-finished edits and fail for reasons unrelated to its task. The prompt has no section telling the agent that other plans are running.

Fix: tell agents which plans and areas are in flight and to scope their builds to their own crate, or isolate plans in worktrees (gap-0001a1).

## Notes

- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check.
  - The plan-set loop (`graph_execution/plan_runner.rs`) tells the shared `GraphTaskDispatcher` when each plan
    starts, with the areas its `PlanFootprint` writes, and when it finishes.
  - Every dispatch carries the other running plans in `DispatchContext::concurrent_plans`. The runner context
    renders them under `# Plans Running Beside This One`, asking the agent to build and test only its own crates
    and to leave those areas alone.
  - Tests: `prompt_names_concurrent_plans` and `concurrent_plans_name_the_other_running_plans`.
