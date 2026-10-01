+++
id = "gap-d31457"
kind = "gap"
title = "Plan Cost Budget Enforcement and Early Termination"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination"
discovered_from = "audit:tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::resolve_budget_ceiling", "crates/roko-cli/src/main.rs:2075", "crates/roko-cli/src/graph_task_dispatch.rs::with_plan_budget", "crates/roko-cli/src/graph_task_dispatch.rs::admit_task_budget", "crates/roko-core/src/config/budget.rs::BudgetConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^pub fn resolve_budget_ceiling/,/^}/p' crates/roko-cli/src/graph_execution/plan_runner.rs | grep -q '(ceiling.max(0.0), false)'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:52Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:55Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
retry loops can burn unlimited tokens with no circuit breaker. When a task enters a retry loop due to gate failures, each retry dispatches a new agent that reads the codebase, investigates the failure, and attempts a fix — all costing tokens. There is no per-task or per-plan cost budget that…

Imported without verification from:
- `tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A task that exceeds its cost budget is stopped with a clear message.; A plan that exceeds its cost budget stops dispatching new tasks.; `--max-cost 2.0` on CLI limits total plan spend to $2. [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): -- | -- |]

Verified 2026-09-28: Mostly in place: a config per-plan ceiling ([budget].max_plan_usd) blocks further dispatch at HEAD (resolve_budget_ceiling, plan_budget_snapshot().dispatch_blocked). Per-task and cumulative-retry caps (task_budget_ceiling_usd/admit_task_budget, max_task_retry_usd -> BudgetExceeded) exist only in the working tree. Still true: a CLI ceiling (`--budget-override <AMOUNT>`) only warns and execution continues (bypass_block), so the CLI cannot cap plan spend. Severity lowered p1 -> p2 (the source summary rated it P3).

Checked 2026-09-29 at d9e79e9d8: the per-task and cumulative-retry caps that were working-tree-only on 2026-09-28 are committed in 725f21e05 (graph_task_dispatch.rs task_budget_ceiling_usd and admit_task_budget, returning BudgetExceeded). What remains: an explicit CLI ceiling (--budget-override <AMOUNT>, main.rs:2075) still sets continue_on_exhaustion, so the plan warns and keeps dispatching past it; no CLI flag hard-caps total plan spend (there is no --max-cost).

## Notes

- 2026-10-01 (wk-childenv): implemented on work/gap-1555ac; cargo verification deferred to the batch check.
  `resolve_budget_ceiling` (plan_runner.rs) returns `(ceiling, false)` for `--budget-override`, so the Graph
  dispatcher's plan ledger blocks further dispatch once the plan has spent it, as for `[budget] max_plan_usd`;
  only `--no-budget` continues past a spent budget (and a spent day). Help text and docs/v2 CLI reference
  updated; test `a_budget_override_is_a_hard_ceiling`. `--budget-override` is the CLI cap (there is no
  `--max-cost`). Left alone: the unused duplicate `resolve_budget_ceiling` in commands/plan.rs, whose tests
  still assert the old warn-only result.
