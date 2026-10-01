+++
id = "bug-4c553b"
kind = "bug"
title = "A stall-watchdog timeout settles as provider_error with infra blame, because its error text lacks the \"timed out after\" marker the classifier keys on"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/provider"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "8a3c530af"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-9eebcb, branch work/gap-9eebcb at 55ffa7074)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs", "crates/roko-agent/src/provider/error_classify.rs", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-9eebcb", "bug-aa2044"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_stalled_attempt_settles_as_a_timeout' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_stalled_attempt_settles_as_a_timeout"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T15:16:11Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20f gate on 2ff1b7891 (MAIN has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/core/graph/serve; lib tests roko-cli 3309, roko-agent 2296, roko-core 1963, roko-serve 992, roko-graph 480 pass; extras: all eight canaries + golden_path_suite + secret_canary 11/11 + C2 2/2 + worktree_task_diff + default_engine pass, bin 429, graph_task_dispatch suite at --test-threads=32 passed 10 of 10, including a_stalled_attempt_settles_as_a_timeout (one typed path, watchdog::failed_call_settlement). Merged (work/gap-9eebcb-l7 9424434fd)."
+++

## Problem

When the stall watchdog cancels a silent attempt, the attempt settles as `provider_error` with infrastructure blame, not as a timeout. The watchdog's `RokoError::Timeout` text lacks the `timed out after` marker that `provider_failure_reason` and `error_classify` match on (wk-tiers, while writing C7).

## Why it matters

Record truth (epic spec-b7303f): a stall is the agent's or the model's failure to make progress, not a provider outage. Mislabelled, it skews failover, the router's provider health and the infra-error rate the bench reports.

## Plan

1. Classify a watchdog stall as a timeout: give its error the marker, or better, a typed timeout kind that the classifier reads without string matching.
2. Add `a_stalled_attempt_settles_as_a_timeout`: a silent fake is cancelled by the watchdog, and its attempt record says timeout, not provider_error.

## Done when

- [ ] A watchdog-cancelled attempt settles as a timeout.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-tiers): implemented on `work/gap-9eebcb-l7` at `fe18ce50e`; cargo verification deferred to the batch check. `AttemptInterrupted::settlement` settles a stall as `timeout` on both dispatch paths, using the typed interruption rather than the error text. The timeout is the agent's once the attempt reported anything, the provider's before that. A conductor restart still settles as a provider failure, as before; it arguably belongs to the harness (`cancelled`), which is left for a follow-up. Test: `a_stalled_attempt_settles_as_a_timeout`.
- 2026-10-01 (wk-tiers): at `c6bb37a33`, merging batch 20e reconciled this with model-truth's `Settlement::provider_call_error` (bug-2b1ddc) into one typed path, `watchdog::failed_call_settlement`, used at both dispatch sites and in streaming:
  - a stall settles as a timeout, as above;
  - a cancellation settles as `cancelled`, a stopping plan run's included;
  - anything else is a provider failure, a conductor restart included.

  `AttemptInterrupted::settlement` is gone. Cargo verification is deferred to the batch check.
