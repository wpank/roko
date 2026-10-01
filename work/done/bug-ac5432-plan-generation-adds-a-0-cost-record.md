+++
id = "bug-ac5432"
kind = "bug"
title = "Plan generation adds a $0 cost record beside the real one, and plan generate and regenerate record only $0"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/prd", "roko-cli/plan-generate", "roko-learn/costs"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "review of gap-a6e2c3's fix (9a7e8a1cb), whose message notes the generation episode's $0 record"
anchors = ["crates/roko-cli/src/agent_exec.rs::persist_capture_episode", "crates/roko-cli/src/agent_episode.rs::build_capture_episode", "crates/roko-learn/src/runtime_feedback/episode_helpers.rs::derive_cost_record", "crates/roko-cli/src/agent_exec.rs::run_agent_capture_logged", "crates/roko-cli/src/prd.rs::generate_plan_from_prd", "crates/roko-cli/src/prd.rs::regenerate_old_format_plan", "crates/roko-cli/src/plan_authoring.rs::AuthoringSpend"]
links = { depends_on = [], blocks = [], related = ["gap-a6e2c3", "bug-0f8948", "gap-288e38", "bug-690dc6", "find-6a5b62"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_generation_writes_one_cost_record' crates/roko-cli/src && cargo test -p roko-cli --lib plan_generation_writes_one_cost_record"

[[verify]]
command = "grep -rqw 'fn plan_generate_records_the_agent_spend' crates/roko-cli/src && cargo test -p roko-cli --lib plan_generate_records_the_agent_spend"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:25Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:12:59Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`.roko/learn/costs.jsonl` gets plan-authoring spend wrong in two ways:

1. **Double rows.** Each generation through `generate_plan_from_prd` (the portal's generate, `roko prd plan`) now
   records its real spend with `AuthoringSpend` (gap-a6e2c3), and it also writes a second `CostRecord` for the same
   call with 0 tokens and $0, from the learning episode it captures.
2. **Spend dropped.** `roko plan generate` (all three branches: plain or `--from-file`, `--from-notes`,
   `--from-backlog`), `roko plan regenerate` and the old-format regeneration inside `generate_plan_from_prd` record
   only that $0 row. The usage the provider reported is thrown away.

## Why it matters

Goal `core`. Cost totals and per-role or per-model averages read from `costs.jsonl` undercount CLI plan authoring,
and $0 rows skew counts and averages. The same capture episodes also trigger an unrecorded distillation call
(bug-0f8948).

## Where

- `crates/roko-cli/src/prd.rs::generate_plan_from_prd`: `AuthoringSpend::generation` (:1355) records each agent
  call. `persist_capture_episode(…, "prd-plan-generate", …)` runs on every outcome (:1498, :1591, :1968, :2098).
- `crates/roko-cli/src/agent_exec.rs::persist_capture_episode` (:303) builds the episode with
  `agent_episode.rs::build_capture_episode` (:74), which sets only `usage.wall_ms`: no tokens, no cost.
  `record_completed_run` receives no `cost_record`, so it derives one from the episode (roko-learn
  `runtime_feedback/mod.rs:1337-1349` → `episode_helpers.rs::derive_cost_record`) and appends it.
- `crates/roko-cli/src/agent_exec.rs`: `run_agent_logged` (:85) and `run_agent_capture_logged` (:102) map the
  `AgentCapture` to `(exit_code, output)` and drop its `usage`.
- Callers that record only $0: `commands/plan.rs` `PlanCmd::Generate` (:719 `--from-backlog`, :862 `--from-notes`,
  :959 plain), `PlanCmd::Regenerate` (:1121), and `prd.rs::regenerate_old_format_plan` (:274, run when
  `regenerate_old_plans` is set, :2001).
- Revision (`plan_authoring.rs:376-385`) records through `AuthoringSpend::revision` and writes no capture episode.
  It is correct.

## Current state

Checked at `d5c1dc6be` by reading the code. `9a7e8a1cb` (gap-a6e2c3) fixed the event-stream side for serve generate
and revise and `roko prd plan`. Its commit message notes that `costs.jsonl` "held only the generation episode's $0
record"; that row is still written. `commands/plan.rs` was not changed.

## Plan

1. One record per call. Either give the capture episode the call's real usage and pass `AuthoringSpend`'s
   `CostRecord` as `CompletedRunInput.cost_record` (with `AuthoringSpend` then not appending its own), or skip
   `derive_cost_record` for capture episodes whose spend is recorded elsewhere. Keep the episode itself: learning
   uses it.
2. Keep the usage. Have `run_agent_logged` / `run_agent_capture_logged` return or record the `AgentCapture` usage,
   and route `plan generate`, `plan regenerate` and `regenerate_old_format_plan` through `AuthoringSpend` keyed by
   the target plan.
3. Tests: `plan_generation_writes_one_cost_record` (a generation with a fake Claude CLI that reports a known cost
   leaves exactly one row, with that cost) and `plan_generate_records_the_agent_spend` (the `plan generate` path
   records the reported cost).

## Done when

- A generation leaves one `costs.jsonl` row per agent call, carrying the reported tokens and cost.
- `plan generate` and `plan regenerate` record their real spend.
- Both `[[verify]]` commands pass.

## Notes

- `plan_authoring::tests::authoring_spend_*` and `serve_runtime::tests_authoring_spend` already have a fake Claude CLI
  that reports a known cost. Reuse it.
- find-6a5b62 (parked, unverified) saw "$0 plan-operation costs" in an older audit; this item is the verified,
  current form of that part.
- gap-288e38 (no usage-source field on `CostRecord`) is related but separate.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - Re-checked at BASE: bb1d7f952 (gap-2623b2, one plan generator) already routes plain and `--from-notes`
    `plan generate`, `plan regenerate` and `regenerate_old_format_plan` through `prd::generate_plan`, which records
    each call with `AuthoringSpend`. Two defects were left: the $0 row of every capture episode, and
    `plan generate --from-backlog`, which still ran `run_agent_logged` and kept no usage.
  - Double rows: both copies of `build_capture_episode` (`agent_episode.rs`, `commands/util.rs`) mark the episode
    cost-unknown, as `bench.rs` does for spend it never sees, so `derive_cost_record` writes no $0 row. The episode
    itself is still logged.
  - `--from-backlog`: the new lib `agent_exec::run_agent_logged_with_spend` echoes, logs the episode and records the
    call through `AuthoringSpend::generation(<slug>)`. Tests: `plan_generation_writes_one_cost_record` and
    `plan_generate_records_the_agent_spend`, both in `agent_exec.rs` with a fake Claude CLI.
  - Not covered: research, `roko do` and the PRD draft commands also log capture episodes without usage. They lose
    their $0 rows, and their real spend is still recorded nowhere.
