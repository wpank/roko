+++
id = "bug-acab47"
kind = "bug"
title = "A Claude CLI run that exits 0 without a result line counts as a successful provider run"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "faa378453"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-9eebcb, branch work/gap-9eebcb at 55ffa7074)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["gap-9eebcb", "gap-cd3529"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_run_without_a_result_line_is_a_provider_failure' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_run_without_a_result_line_is_a_provider_failure"
+++

## Problem

wk-tiers' delta-only fake exited 0 without ever printing a `result` event, and its attempt passed. A Claude CLI stream that ends without its `result` line is truncated: the provider never reported completion, usage or cost.

## Why it matters

Honest verdicts (epic spec-e9d7ec): a truncated run should not count as a completed provider call. The task's own verify still decides whether the work is done, but the provider outcome, usage and cost records should say the run was cut short.

## Plan

1. In `claude_cli_agent.rs`, treat a zero exit with no `result` event as a provider failure (truncated output), keeping whatever usage the stream carried.
2. Add `a_run_without_a_result_line_is_a_provider_failure`.

## Done when

- [ ] A run without a result line is recorded as a provider failure.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-tiers): implemented on `work/gap-9eebcb-l7` at `fe18ce50e`; cargo verification deferred to the batch check.
  - A stream-json run with no `result` event on either stream fails with `claude exited without its final result event (truncated output)`, and keeps its estimated streamed usage. Plain-text output is unaffected.
  - 20 test fixtures that streamed only deltas and expected success now print a result line: in roko-agent; in roko-cli, including `tests/smoke.rs` and `tests/e2e_domain.rs`; and in roko-serve's `dispatch.rs` and `service_factory.rs`.
  - Test: `a_run_without_a_result_line_is_a_provider_failure`.
