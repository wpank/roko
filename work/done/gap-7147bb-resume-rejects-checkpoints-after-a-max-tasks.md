+++
id = "gap-7147bb"
kind = "gap"
title = "Resume rejects checkpoints after a --max-tasks change, and `roko resume` cannot pass --max-tasks"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_checkpoint", "roko-cli/resume"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3b-fingerprint"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::convert_plan", "crates/roko-graph/src/fingerprint.rs::EXECUTION_META_FIELDS", "crates/roko-cli/src/main.rs:1028"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'fn resume_accepts_a_different_max_tasks' crates/roko-cli/src && cargo test -p roko-cli --lib resume_accepts_a_different_max_tasks"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:41Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:54Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

The fingerprint no longer changes with the binary (w3b), but it still covers run policy. `--max-tasks` becomes the graph's `max_parallel` (graph_checkpoint.rs:1477), and both the graph policy and the `max_parallel` meta field are fingerprinted (roko-graph fingerprint.rs:20-41). Resuming with a different `--max-tasks` than the original run is therefore refused as a different graph. `roko resume` takes only a run id and `--workdir` (main.rs `Command::Resume`), so a run started with `--max-tasks` cannot be resumed through it at all. w3b also reports that checkpoints written before the fingerprint fix used a different `TaskDef` serialization and cannot be migrated.

Fix: leave scheduling policy out of the fingerprint (or store it in the checkpoint and reuse it on resume), and let `roko resume` reuse the original run's options.

## Notes

- 2026-10-01 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  `run_one_plan` and the resume preview (`convert_plan`) convert a plan at its own `max_parallel` (1 when omitted),
  and the run applies `--max-tasks` to `graph.policy.max_concurrent_nodes` after the checkpoint identity is taken, as
  it already did for the failure strategy and the DAG width. `roko resume` takes `--max-tasks` and passes it on.
  A checkpoint written before this change by a run that passed `--max-tasks` recorded that concurrency, so it resumes
  only if the plan's own `max_parallel` matches; otherwise `--force-resume` starts a new run.
