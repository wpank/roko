+++
id = "bug-35379d"
kind = "bug"
title = "Provider failover silently runs a different model and records it as if it had been chosen"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "6f8286d48"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover", "crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_model", "roko.toml:282"]
links = { depends_on = [], blocks = [], related = ["find-229e9c", "bug-f68404"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'planned_model|substituted_from' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-learn/src/efficiency.rs && cargo test -p roko-cli --lib failover_records_planned_and_substitute_model"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 6f8286d48. Failover records the planned and the substitute model, and a substitute earns no router credit. Batch 12b gate on the merged tree (MAIN 6f8286d48 has the same crates and Cargo.lock as gated b0ede92d7): check, nightly fmt and clippy -p roko-cli -p roko-agent -p roko-learn -p roko-serve --no-deps -D warnings clean; lib tests pass: roko-cli 3167, roko-agent 2262, roko-learn 1200, roko-serve 977 (two load flakes, a_timed_out_attempt_reports_the_usage_it_streamed and a_timed_out_attempt_is_resumed_with_an_escalated_timeout, pass alone); cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: failover_records_planned_and_substitute_model passes (roko-cli lib)."
+++
When the planned provider is blocked or reports usage exhaustion, `run_bridge_with_failover` runs `failover_model()` instead. The substitution is only logged (`tracing::warn!("model substitution: …")`, `graph_task_dispatch.rs:4600`).
- `dispatch.target` names the model that actually ran, and nothing on the attempt, episode or efficiency record marks it as a substitute.
- The router then credits the substitute as its own pick.
- This repo's `roko.toml:282` falls back to kimi-k2-5, glm51 and gpt-4o, so a task planned for one model can be finished by another without a trace.
- Pinning with `--model` turns failover into an error.

Fix: record the planned model, the substitute and the reason on every attempt record; keep substituted attempts out of router credit, or credit them explicitly; allow failover to be switched off per run.

2026-09-29: re-verified at d9e79e9d8. Unchanged: substitution is still only logged (failover_model WARN at graph_task_dispatch.rs:4971, formerly ~4600); no record field for the planned model or substitution reason and no per-run opt-out.

## Notes

- Implemented on `work/bug-31438d` at `480f463bf` (feedback.rs rows at `06bdb71be`); cargo verification deferred to the batch check. `failover_records_planned_and_substitute_model` (targeted `cargo test` passed at the branch head). Changes:
  - `run_bridge_with_failover` returns a `FailoverChain`: the refused model keys, the planned one first, and why the planned one did not run.
  - The verdict's `executed.failover_chain` and `failover_reason` carry it, and `model_dispatched` names the substitute.
  - Episodes get `extra.substituted_from`, `failover_chain` and `failover_reason`; cost and efficiency rows get `substituted_from` and `substitution_reason`.
  - Router credit (at `c50d36be2`): `RoutingObservationSink` skips attempts whose verdict has a failover chain, so a substitute is never credited as the router's pick (test `a_failover_substitute_earns_no_router_credit`). wk-settle's gap-8f6206 rewrote the start of `on_event`; when merging, keep their label check and put this guard after it.
  - Not done: a per-run failover switch. `RoutingConfig` is wk-tiers' (gap-9cbf35) and the plan runner is wk-telemetry2's. A `--model` pin already disables failover.
