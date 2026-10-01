+++
id = "gap-6bc156"
kind = "gap"
title = "runner.max_concurrent_plans and [executor] max_concurrent_plans do nothing and are not flagged as inert"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-core/config", "roko-cli/config"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-core/src/config/schema.rs:2464", "crates/roko-cli/src/config.rs:1960", "crates/roko-cli/src/config_cmd.rs:126", "crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings"]
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "(grep -q '\"runner.max_concurrent_plans\"' crates/roko-cli/src/graph_task_dispatch.rs || ! grep -q 'max_concurrent_plans' crates/roko-core/src/config/schema.rs) && (grep -q '\"executor.max_concurrent_plans\"' crates/roko-cli/src/graph_task_dispatch.rs || ! grep -q 'pub max_concurrent_plans' crates/roko-cli/src/config.rs)"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:41Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:54Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

Plan concurrency is set by `max_parallel_plans` (config, or the `--max-parallel-plans` flag). Two older knobs with the same meaning remain: `runner.max_concurrent_plans` (roko-core schema.rs:2464) and `[executor] max_concurrent_plans` (roko-cli config.rs:1960, settable with `roko config set executor.max_concurrent_plans`). The Graph runner reads neither, and `graph_engine_inert_settings` lists neither, so `roko config doctor` does not warn about them.

Fix: delete both (with a migration note), or add them to the inert-settings list.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. Also, `roko config init` writes executor.max_concurrent_plans = 4 into new configs (crates/roko-cli/src/config_cmd.rs:126), so the inert key spreads to new workspaces.

## Notes

- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise still true at BASE `ebdc0f5d5`: nothing outside the loader's schema sentinel and one test read
  `runner.max_concurrent_plans`, and nothing in production reads the CLI config's `executor` section. Took the delete
  option, because `conductor.max_parallel_plans` is the knob that works.
- `runner.max_concurrent_plans`: removed from `CoreRunnerConfig` and the schema sentinel. A new `REMOVED_CONFIG_KEYS`
  list in `loader.rs` holds the migration note: validation reports the key as removed and names
  `conductor.max_parallel_plans`; loading strips it with that warning; and `RokoConfig::from_toml`, which `config doctor`
  and repo configs use and which denies unknown fields, now drops a removed key with the warning instead of failing
  (`drop_removed_config_keys`). Test `removed_runner_max_concurrent_plans_still_loads`. `roko config validate` reports
  an old file's key as an error, as it does for every removed key.
- `[executor] max_concurrent_plans`: `roko config init` no longer writes it, `config set` no longer accepts it (the
  allowlist arm is gone, and the schema has no `[executor]`), and the unused `ExecutorLayer` lost the field. The
  orchestrator `ExecutorConfig` keeps its field: `ParallelExecutor` uses it, but only its own tests and
  `tests/merge_proof.rs` build one. `graph_engine_inert_settings` is unchanged; it only sees core `RokoConfig` fields.
