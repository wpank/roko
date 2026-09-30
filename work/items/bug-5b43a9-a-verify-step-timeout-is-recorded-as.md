+++
id = "bug-5b43a9"
kind = "bug"
title = "A verify-step timeout is recorded as a permanent failure, and roko diagnose counts no timed-out attempt"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/gates", "roko-cli/diagnose"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-evidence's report on gap-09e478)"
anchors = ["crates/roko-cli/src/runner/gate_report.rs::classify_failure_kind", "crates/roko-cli/src/commands/diagnose.rs::mentions_timeout", "crates/roko-cli/src/runner/gate_dispatch.rs:1498"]
lane = "rust-cold"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["gap-09e478", "bug-6f7f72"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_verify_step_timeout_is_recorded_as_a_timeout' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_verify_step_timeout_is_recorded_as_a_timeout"
+++

## Problem

wk-evidence ran a plan whose verify step overran its `timeout_ms` of 1500 ms. The three records of that failure disagree:

- the `dashboard.gate_result` event says `timed out after 1500 ms`;
- `.roko/learn/gate-failures.jsonl` records `failure_kind = "permanent"`;
- `roko diagnose` reports `timed_out_attempts: 0`.

## Why it matters

Honest verdicts (epic spec-e9d7ec). A timeout is usually retryable, or a sign the step needs more time; it is not a permanent defect. Recording it as permanent steers retry policy and failure analysis the wrong way, and the diagnosis hides the cause. Evidence bundles now flag the disagreement as a validation warning (gap-09e478).

## Where

- `crates/roko-cli/src/runner/gate_report.rs::classify_failure_kind` (:158) maps the failure text through `classify_gate_failure` and `RunnerFailureKind::from_output`; a `NeedsHuman` action becomes `Permanent`.
- `crates/roko-cli/src/runner/gate_dispatch.rs:1498` and `:1727` set `failure_kind` for the gate-failure record.
- `crates/roko-cli/src/commands/diagnose.rs`: an attempt counts as timed out only when `mentions_timeout` finds "timed out" in the matched episode's failure reason (:760, :1499). For a verify failure that reason only says how many steps failed.

## Current state

Observed by wk-evidence while building gap-09e478 (its last note). Which branch of `classify_failure_kind` turns the timeout into `permanent` is inferred from the record; pin it down with a unit test first.

## Plan

1. Carry the timeout as structured data: the verify step already knows it timed out, so pass a `timed_out` flag (or a `Timeout` failure kind) into the verdict instead of re-deriving it from text.
2. Write it to `gate-failures.jsonl`, and have `roko diagnose` count an attempt as timed out when any of its gate failures timed out, not only when the episode's reason says so.
3. Add `a_verify_step_timeout_is_recorded_as_a_timeout`, covering both the failure kind and the diagnose count.

## Done when

- [ ] A verify-step timeout is recorded as a timeout in `gate-failures.jsonl` and counted by `roko diagnose`.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-gates): Implemented on `work/bug-5b43a9` at `7a2a1f6b6`; cargo verification deferred to the batch check.
- Pinned at `8a88c6267`: the record comes from the Graph path, `graph_task_dispatch/verification.rs` (the W13 block), not from `runner/gate_report.rs::classify_failure_kind`, whose legacy pipeline no plan-run path calls. The step's authored `fail_msg` (the evidence fixture's `verify failed`) replaces its own `timed out after 1500 ms` reason in the failure text, so `roko_gate::classify_gate_failure` saw an unknown failure and returned `permanent`. Without a `fail_msg` it would have said `transient`. The episode reason, which diagnose read, has the same text.
- `timeout` is a new `roko_gate::GateFailureKind` variant (retryable, as the evidence validator expects). `AttemptOutcome::Timeout` could not be reused there: roko-gate does not depend on roko-learn, and S01 §4.3 keeps a verify-step timeout as `gate_failed{rung}` (`timeout` is the task timeout), so the attempt's outcome stays `gate_failed`. ShellGate marks the classification in its timeout verdict's digest, and `roko_gate::verdict_timed_out` reads it.
- Left as found: the retry prompt and the episode reason still show the authored `fail_msg` without saying the step timed out. The other gates with timeouts (compile, clippy, test, verify-chain, integration, property-test, generated-test) still classify them as `transient`; none is on the plan-run verify path.
