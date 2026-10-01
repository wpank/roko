+++
id = "bug-aad63e"
kind = "bug"
title = "roko acp distils every episode with unrecorded spend, and roko serve's dispatch path is unchecked"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0f8948"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs", "crates/roko-serve/src/dispatch.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0f8948"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib acp_distillation_records_spend"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:20Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:43Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

bug-0f8948 stopped the CLI's per-episode distillation call from going unaccounted. `roko-acp/src/bridge_events/cost.rs` (around line 237) still distils every ACP episode with spend recorded nowhere, and `roko-serve/src/dispatch.rs` (around line 2633) has not been checked.

## Plan

Apply bug-0f8948's rule in both places: record the distillation call's spend, or skip it as the CLI now does. Add a test named `acp_distillation_records_spend_*`.

## Done when

- The test passes, and serve's path is checked and noted.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-0f8948, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - bug-0f8948's `DistillationSpend` moved from roko-cli into roko-neuro (`episode_completion.rs`), next to
    `spawn_episode_distillation`, with `spawn_recorded_episode_distillation`. The CLI's capture paths now use it from
    there, and ACP distils through it via `bridge_events/cost.rs::spawn_acp_distillation`, so each ACP distillation call
    writes one cost record and one efficiency row under role `episode-distiller`. The provider comes from
    `roko_core::agent::resolve_model`. Test: `acp_distillation_records_spend`.
  - Serve checked: `roko-serve/src/dispatch.rs` distils through `state.model_call_service`, which
    `service_factory.rs` builds with a feedback sink, a gateway event writer, a runtime-event logger and, when serve
    has one, an inference observer. The call is accounted like every other serve model call, so it stays on the plain
    `spawn_episode_distillation`; wrapping it would count it twice.
