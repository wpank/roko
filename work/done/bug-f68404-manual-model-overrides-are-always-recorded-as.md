+++
id = "bug-f68404"
kind = "bug"
title = "Manual --model overrides are always recorded as router successes"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8b26e4839"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-learn/src/cascade_router.rs::CascadeRouter::record_override_outcome", "crates/roko-learn/src/cascade_router.rs::CascadeRouter::observe_multi_objective"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn failed_override_lowers_success_rate' crates/roko-learn/ && cargo test -p roko-learn failed_override_lowers_success_rate"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 8b26e4839. Batch 11 gate on the merged tree (MAIN 8b26e4839 has the same tree as gated 194658fed): cargo check --workspace --tests, nightly fmt --check and clippy -p (10 crates) --no-deps -D warnings clean; lib tests pass: roko-cli 3126, roko-agent 2257, roko-core 1936, roko-learn 1188, roko-serve 958, roko-gate 688, roko-graph 471, roko-dreams 252, roko-std 223, roko-execution 100. Verify: cargo test -p roko-learn failed_override_lowers_success_rate: 1 passed."
+++

`record_override_outcome` (`cascade_router.rs:1540`) forwards to `observe_multi_objective` (`:1703`), which increments both `trials` and `successes` unconditionally (`:1718-1719`); cost and latency are passed as 0.0.
A failed override run therefore counts as a free, instant success, and `override_learning_dampening` shrinks the reward instead of the observation weight.
Fix: honour the success flag, record real cost/latency, apply dampening as an importance weight, and test that a failed override lowers the arm's success rate.

## Notes

- Implemented on `work/bug-f68404` at `cfcb7fd75`. Checked in the worker's own target dir: `cargo check --tests` on roko-learn, roko-agent, roko-cli and roko-serve; `cargo test -p roko-learn --lib` (1184 passed, at `7e930d2cb`); fmt, and clippy `-D warnings` on those four crates. The batch check re-verifies after merge.
- Overrides now count fully in the confidence counters, honouring success, and enter LinUCB as an importance-weighted update (weight = the dampening, `OVERRIDE_LEARNING_RATE` 0.5 by default) with the real-cost reward, or 0 on failure. `ForceBackendOverrideRecorder::record_override_outcome` gained `cost_usd` and `latency_ms`, so the model-call service records real values too.
- Premise re-checked at BASE `942d2a6c3`, which includes gap-8f6206 (`04c1da262`). Both override callers, the Graph `RoutingSink` and `RoutingObservationSink`, now pass the settled verdict as `success`. The router still dropped it: `CascadeRouter::override_observation` set `success: true` for every override (`cascade_router.rs:1588-1589`, "until bug-f68404 honours `success` here"), and scored a failed override `compute_routing_reward_with_weights(0.0, 0.0, 0.0, ..)` = 0.5. So gap-8f6206 fixed the input, and `cfcb7fd75` fixes the router.
