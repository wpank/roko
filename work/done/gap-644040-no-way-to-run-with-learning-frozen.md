+++
id = "gap-644040"
kind = "gap"
title = "No way to run with learning frozen: prompts and routing change from run to run"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "2347ad858"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::plan_skips_enrichment", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/dispatch/prompt_builder.rs:153"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn frozen_learning_run_writes_no_learned_state' crates/roko-cli/src && cargo test -p roko-cli --lib frozen_learning_run_writes_no_learned_state"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T01:24:37Z"
commit = "2347ad858"
forced = false
evidence = "Frozen learning exists end to end: --frozen-learning / [learning] frozen (PK10, gap-f61823, 2724386ea) and the frozen gate settlement plus the digest proof over every learned-state file (PK11, gap-2b5d37, 2347ad858). frozen_learning_run_writes_no_learned_state passed in gate 5b's nextest run."
+++
`[meta] skip_enrichment` suppresses only eval artifacts and routing advice (`graph_task_dispatch.rs:3061-3074`). Every run still reads and writes learned state:
- c-factor context, section effectiveness, knowledge, episode knowledge and playbooks are injected into prompts (`dispatch/prompt_builder.rs`);
- post-gate reflections are persisted for later attempts (`graph_task_dispatch.rs:2458-2505`);
- T0 reflexes can replace dispatch for tasks with no verify step (`:2990-3059`);
- the router and knowledge stores update after each run;
- dreams and `roko serve` timers keep changing knowledge in the background.

Benchmarks and A/B comparisons therefore cannot hold learning fixed, and repeated seeds see each other's traces.

Fix: a frozen-learning mode (learned state read-only, no writes, dreams and timers off), recorded in each run's manifest.

2026-09-29: re-verified at d9e79e9d8. Still open; anchors moved (skip_enrichment use at graph_task_dispatch.rs:3288, reflections 2575-2618, T0 reflexes 3213, prompt_builder.rs:153). `roko bench swe --no-learning` exists but only stops bench telemetry writes; it is not a frozen-learning run mode.

## Notes

- 2026-10-01 (wk-honestbench): blocked; this needs a scope decision and several days, so no code changed. The
  premise holds at BASE `ebdc0f5d5`. Nothing freezes learning, and the run manifest's
  `experiment.ablation_flags` (`roko-learn/src/telemetry/records.rs:1072`) exists but nothing sets it.
  - The Graph writers come in through `GraphFeedbackContext` (`graph_task_dispatch/feedback.rs`): the feedback
    facade (episodes, router, knowledge), efficiency, playbooks, experiments, gate failures, post-gate
    reflections, gate thresholds and ratchet, retrieval outcomes, and the holdout and gaming detectors.
  - Several of these paths are both read (prompt injection, retry budgets) and written, so "read-only" cannot be
    done by dropping the paths.
  - Decisions needed: what counts as learned state (are `costs.jsonl`, episodes and attempt logs telemetry that
    stays on?), and the switch's form (`[learning] frozen`, a `plan run` flag, or both).
  - Next step: one `learning_frozen` flag in `GraphFeedbackContext`, set from that switch and recorded as an
    ablation flag. Each writer above checks it while its reads stay on, plus dreams and serve timers off. Then
    `frozen_learning_run_writes_no_learned_state` runs a fake-provider plan and compares `.roko/learn` before and
    after.
