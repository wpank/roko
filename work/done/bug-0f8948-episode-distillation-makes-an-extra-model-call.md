+++
id = "bug-0f8948"
kind = "bug"
title = "Episode distillation makes an extra model call per captured episode whose spend is recorded nowhere"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/learning", "roko-neuro"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "session:roko-b6 2026-09-29 portal close-out (a background episode-distiller call next to each plan generation)"
anchors = ["crates/roko-cli/src/learning_helpers.rs::distillation_model_caller", "crates/roko-cli/src/agent_exec.rs::persist_capture_episode", "crates/roko-cli/src/commands/util.rs::persist_capture_episode", "crates/roko-neuro/src/episode_completion.rs::spawn_episode_distillation", "crates/roko-agent/src/model_call_service.rs::ModelCallService"]
links = { depends_on = [], blocks = [], related = ["bug-ac5432", "gap-a6e2c3", "gap-288e38"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn distillation_spend_is_recorded' crates/roko-cli/src && cargo test -p roko-cli --lib distillation_spend_is_recorded"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:09Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:53Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

Every episode recorded through `persist_capture_episode` starts a detached model call that distils the episode into
durable knowledge. Plan generation records such an episode, so each generation (the portal's and `roko prd plan`)
also makes this call. So do `roko plan generate`, `roko plan regenerate`, the research commands, `roko do` and the
PRD commands. Nothing records what the call costs: no `costs.jsonl` row, no efficiency row, no StateHub event, no
gateway event. The only trace is an in-memory budget counter that is gone when the process exits.

## Why it matters

Goal `core`. Cost totals, the portal's cost and any "cheaper at equal quality" figure leave out a paid call that
happens on every captured episode. gap-a6e2c3 (`9a7e8a1cb`) made generation and revision spend visible, but not
this call.

## Where

- `crates/roko-cli/src/agent_exec.rs::persist_capture_episode` (:303) installs an episode-completion hook
  (:348-355) that calls `roko_neuro::spawn_episode_distillation` with `distillation_model_caller(workdir)`. Its
  twin, `commands/util.rs::persist_capture_episode`, does the same (:2591-2597).
- roko-learn `runtime_feedback/mod.rs:1308-1310` fires the hook for every logged episode.
- `crates/roko-neuro/src/episode_completion.rs::spawn_episode_distillation`: `tokio::spawn`s the work.
  `GatewayDistillationBackend` asks for model `""` (:47), meaning the workspace's `agent.default_model`, with role
  `episode-distiller` (:105). `Distiller::distill` makes one backend call for any non-empty batch
  (`distiller.rs:85-98`).
- `crates/roko-cli/src/learning_helpers.rs::distillation_model_caller` (:84-93) builds
  `ModelCallService::new(model).with_config(..).with_working_dir(..).with_immune_root(..)`. `ModelCallService::new`
  (`roko-agent/src/model_call_service.rs:164-192`) leaves `event_consumers`, `inference_observer`, `feedback_sink`,
  `gateway_event_writer` and `metrics` empty, and none is attached, so `inference_completed`, `record_feedback` and
  `write_gateway_event` go nowhere.
- Production callers of `persist_capture_episode`: `prd.rs` (4 sites, task kind `prd-plan-generate`),
  `agent_exec.rs:270` (every `run_agent_logged` / `run_agent_capture_logged` with an episode),
  `commands/research.rs` (12), `commands/prd.rs` (3) and `commands/do_cmd.rs` (1).

## Current state

Checked at `d5c1dc6be` by reading the code; the spend was not measured. In `roko serve` (portal generation) the
process lives on, so the detached call runs to the end. In a short CLI command the runtime may drop it at exit, so
whether and how often it completes there is unknown. roko-serve's template dispatch distils through the shared
`state.model_call_service` (`roko-serve/src/dispatch.rs:2618-2621`); whether that service records spend was not
checked. Graph task dispatch grows durable knowledge only from gate-verified attempts
(`graph_task_dispatch.rs:1759`, `VerifiedAttempt`) and does not use this hook.

## Plan

Options:

- (a) Record the call. Wrap the distillation `ModelCaller`, or attach a sink, so that each call writes a
  `CostRecord` and an efficiency row the way `plan_authoring::AuthoringSpend` does, with role `episode-distiller`
  and the episode's plan and task ids. Where a StateHub is at hand (serve), publish it too.
- (b) Stop distilling capture episodes. Plan-generation and research outputs are not gate-verified, and the Graph
  path distils only verified attempts. This removes the call and its spend.
- (c) Put distillation of capture episodes behind a config switch, off by default, and record it (a) when on.

(a) is the minimal truth fix. (b) or (c) also changes what enters the knowledge store, so check with Will before
choosing it. Then add `distillation_spend_is_recorded` (roko-cli lib): with a fake model caller that reports a known
cost, persist a capture episode and assert that the distillation's cost appears once in `costs.jsonl`. For (b), assert
instead that no distillation call is made.

## Done when

- Every distillation call that runs is recorded with its tokens and cost, or capture episodes no longer trigger one.
- The `[[verify]]` command passes.

## Notes

- bug-ac5432 covers the other cost-record defect of the same capture episodes: they also write a $0 row of their own.
- Durable-knowledge decay and dreams are on hold. This item is only about the spend of the distillation call.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e (option a); cargo verification deferred to the batch check.
  `learning_helpers::install_capture_distillation` replaces the two copies of the hook (`agent_exec.rs` and
  `commands/util.rs`). It wraps the caller in `DistillationSpend`, a `ModelCaller` that appends one `CostRecord` to
  `costs.jsonl` and one efficiency row per returned call, under role `episode-distiller` with the episode's plan and
  task ids. No StateHub is at hand on this path, so nothing is published live. Test: `distillation_spend_is_recorded`.
- Still unrecorded, outside this item's capture paths: roko-acp `bridge_events/cost.rs:237` distils every ACP episode
  through a bare `ModelCallService` (the same defect; `DistillationSpend` lives in roko-cli, which roko-acp cannot use),
  and roko-serve `dispatch.rs:2633` distils through `state.model_call_service`, whose spend recording was not checked.
