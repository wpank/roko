+++
id = "bug-4c553b"
kind = "bug"
title = "A stall-watchdog timeout settles as provider_error with infra blame, because its error text lacks the \"timed out after\" marker the classifier keys on"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/provider"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-9eebcb, branch work/gap-9eebcb at 55ffa7074)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs", "crates/roko-agent/src/provider/error_classify.rs", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-9eebcb", "bug-aa2044"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_stalled_attempt_settles_as_a_timeout' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_stalled_attempt_settles_as_a_timeout"
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
