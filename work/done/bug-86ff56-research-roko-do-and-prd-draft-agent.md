+++
id = "bug-86ff56"
kind = "bug"
title = "Research, roko do and PRD-draft agent calls record no spend"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-ac5432"
anchors = ["crates/roko-cli/src/research.rs", "crates/roko-cli/src/commands/do_cmd.rs", "crates/roko-cli/src/prd.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-ac5432"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib research_calls_record_spend"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:17Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:43Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After bug-ac5432, capture episodes no longer write $0 cost rows. That exposes that research, `roko do` and PRD-draft agent calls record no spend at all.

## Plan

Record each call's real usage and cost in the cost log, the way plan generation does after bug-ac5432. Add a test named `research_calls_record_spend_*`.

## Done when

- `cargo test -p roko-cli --lib research_calls_record_spend` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-ac5432, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - `AuthoringSpend` gains `operation(workdir, task_id, role)`: the same cost record, efficiency row and attempt id as
    plan authoring, with no plan id, under the operation's own task id and role.
  - New lib `agent_exec::run_agent_capture_silent_recorded` runs the agent and records the call through it. Every
    research Claude path (enhance-prd, enhance-plan, enhance-tasks, analyze, topic fallback), `roko do`'s PRD draft,
    and `roko prd draft new`, `draft edit` and `consolidate` call it. The Perplexity deep and standard and the Gemini
    topic paths record their `AgentResult` usage through `record_research_spend`.
  - Test: `research_calls_record_spend` (fake Claude CLI with a known cost).
  - Not covered: `roko research search` calls the Perplexity Search API, which reports no token usage or cost, so it
    still records nothing.
