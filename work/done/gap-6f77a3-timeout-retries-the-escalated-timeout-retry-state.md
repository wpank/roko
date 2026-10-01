+++
id = "gap-6f77a3"
kind = "gap"
title = "timeout_retries (the escalated-timeout retry state) is memory-only and resets on resume"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-34b2ed"
anchors = ["crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-34b2ed"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib timeout_retries_survive_resume"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:33Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:37:48Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-34b2ed persisted per-task spend and turn caps in `retry-feedback.json`. The escalated-timeout retry count (`timeout_retries`, which grows a retry's timeout 1.5x) is still in memory only, so a resumed run restarts the escalation from the base timeout.

## Plan

Persist it beside spend and turn_caps, serde-defaulted. Add a test named `timeout_retries_survive_resume`.

## Done when

- `cargo test -p roko-cli --lib timeout_retries_survive_resume` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-34b2ed, during the evening close-out round.
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `retry-feedback.json` gains `timeouts` (task id to the timeout in ms its last attempt ran out of, serde-defaulted)
  beside `spend` and `turn_caps`. `keep_timeout_retry` and `take_timeout_retry` keep it in step with the in-memory
  `timeout_retries`, `attach_retry_feedback` restores it on resume, and a pass clears it. Test:
  `timeout_retries_survive_resume`.
